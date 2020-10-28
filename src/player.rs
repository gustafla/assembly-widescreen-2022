use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream,
};
use lewton::inside_ogg::OggStreamReader;
use parking_lot::Mutex;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    FileAccess(#[from] std::io::Error),
    #[error(transparent)]
    Decode(#[from] lewton::VorbisError),
    #[error("No audio output devices")]
    NoAudioOutputDevices,
    #[error("Failed to query audio device")]
    QueryAudioOutputSupport(#[from] cpal::SupportedStreamConfigsError),
    #[error("Default audio device doesn't support required configuration")]
    NoSupportForRequiredConfiguration,
    #[error("Ogg file doesn't contain audio")]
    NoAudioStreamInFile,
    #[error(transparent)]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error(transparent)]
    PauseStream(#[from] cpal::PauseStreamError),
}

pub struct Player {
    ogg_stream: Arc<Mutex<OggStreamReader<BufReader<File>>>>,
    out_stream: Stream,
    sample_rate: u64,
    start_time: Instant,
    pause_time: Instant,
    time_offset: Duration,
    error_sync_rx: mpsc::Receiver<()>,
    playing: bool,
    at_end: Arc<AtomicBool>,
}

impl Player {
    pub fn new(ogg_path: impl AsRef<Path>) -> Result<Self, Error> {
        log::info!("Loading {}", ogg_path.as_ref().display());

        // Read ogg file headers
        let ogg_file = File::open(ogg_path)?;
        let ogg_reader = BufReader::new(ogg_file);
        let mut ogg_stream = OggStreamReader::new(ogg_reader)?;
        let sample_rate = ogg_stream.ident_hdr.audio_sample_rate;

        // Initialize cpal output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(Error::NoAudioOutputDevices)?;
        let mut supported_configs_range = device.supported_output_configs()?;
        let config = supported_configs_range
            .find(|c| {
                c.channels() == ogg_stream.ident_hdr.audio_channels.into()
                    && c.min_sample_rate().0 <= sample_rate
                    && c.max_sample_rate().0 >= sample_rate
                    && c.sample_format() == cpal::SampleFormat::I16
            })
            .ok_or(Error::NoSupportForRequiredConfiguration)?
            .with_sample_rate(SampleRate(ogg_stream.ident_hdr.audio_sample_rate));

        // Initialize vorbis streaming
        let mut packet_buf = ogg_stream
            .read_dec_packet_itl()?
            .ok_or(Error::NoAudioStreamInFile)?;
        let mut packet_read = 0;
        let ogg_stream = Arc::new(Mutex::new(ogg_stream));
        let at_end = Arc::new(AtomicBool::new(false));
        let (error_sync_tx, error_sync_rx) = mpsc::channel();

        // Stream with cpal
        let out_stream = {
            let ogg_stream = ogg_stream.clone();
            let at_end = at_end.clone();
            let mut errors = 0;
            device.build_output_stream(
                &config.into(),
                move |output_buf: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut output_written = 0;

                    loop {
                        // Slice to current positions
                        let packet = &packet_buf[packet_read..];
                        let output = &mut output_buf[output_written..];

                        // Copy audio from previously decoded vorbis packet
                        if packet.len() >= output.len() {
                            output.copy_from_slice(&packet[..output.len()]);
                            packet_read += output.len();
                            // Output buffer is full, job done
                            return;
                        } else {
                            (&mut output[..packet.len()]).copy_from_slice(packet);
                            output_written += packet.len();
                            // Output buffer is not filled yet, continue
                        };

                        // When necessary, decode a new packet
                        if let Some(new_packet) = ogg_stream.lock().read_dec_packet_itl().unwrap() {
                            // Swap buffer
                            packet_buf = new_packet;
                        } else {
                            // Or if at EOS
                            // Tell asking threads that the track is at end
                            at_end.store(true, Ordering::SeqCst);

                            // And play silence
                            for sample in packet_buf.iter_mut() {
                                *sample = 0;
                            }
                        }
                        packet_read = 0;

                        // Loop until output buffer is filled
                    }
                },
                move |e| {
                    log::error!("Audio playback error: {}", e);
                    errors += 1;
                    if errors > 100 {
                        panic!("Frequent audio playback errors");
                    }
                    log::info!("Trying to resync");
                    error_sync_tx.send(()).unwrap();
                },
            )?
        };

        out_stream.pause()?;

        let time = Instant::now();

        Ok(Self {
            ogg_stream,
            out_stream,
            sample_rate: u64::from(sample_rate),
            start_time: time,
            pause_time: time,
            time_offset: Duration::new(0, 0),
            error_sync_rx,
            playing: false,
            at_end,
        })
    }

    pub fn is_at_end(&self) -> bool {
        self.at_end.load(Ordering::SeqCst)
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn play(&mut self) -> Result<(), Error> {
        if !self.is_at_end() && !self.is_playing() {
            self.playing = true;
            if let Some(absgp) = self.ogg_stream.lock().get_last_absgp() {
                self.time_offset = Duration::from_nanos((absgp * 1_000_000_000) / self.sample_rate);
            } else {
                self.time_offset += self.pause_time.duration_since(self.start_time);
            }
            self.start_time = Instant::now();
            self.out_stream.play()?;
        }
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), Error> {
        if self.is_playing() {
            self.playing = false;
            self.pause_time = Instant::now();
            self.out_stream.pause()?;
        }
        Ok(())
    }

    pub fn time_secs(&mut self) -> f64 {
        // If playback errors (underruns?) have happened, try to sync with the stream position
        if self.error_sync_rx.try_recv().is_ok() {
            if let Some(absgp) = self.ogg_stream.lock().get_last_absgp() {
                self.time_offset = Duration::from_nanos((absgp * 1_000_000_000) / self.sample_rate);
                self.start_time = Instant::now();
            }
        }

        (if self.is_playing() {
            self.start_time.elapsed()
        } else {
            self.pause_time.duration_since(self.start_time)
        } + self.time_offset)
            .as_nanos() as f64
            / 1_000_000_000f64
    }
}
