use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream,
};
use lewton::inside_ogg::OggStreamReader;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock,
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
    ogg_stream: Arc<RwLock<OggStreamReader<BufReader<File>>>>,
    out_stream: Stream,
    sample_rate: u32,
    last_absgp: Option<u64>,
    time_absgp: Instant,
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
        let last_absgp = ogg_stream.get_last_absgp();
        let time_absgp = Instant::now();
        let ogg_stream = Arc::new(RwLock::new(ogg_stream));
        let at_end = Arc::new(AtomicBool::new(false));

        // Stream with cpal
        let ogg = ogg_stream.clone();
        let end = at_end.clone();
        let out_stream = device.build_output_stream(
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
                    if let Some(new_packet) = ogg.write().unwrap().read_dec_packet_itl().unwrap() {
                        packet_buf = new_packet;
                    } else {
                        // Or if at EOS
                        // Tell asking threads that the track is at end
                        end.store(true, Ordering::SeqCst);

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
                panic!("cpal error {}", e);
            },
        )?;

        out_stream.pause()?;

        Ok(Self {
            ogg_stream,
            out_stream,
            sample_rate,
            last_absgp,
            time_absgp,
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
        self.playing = true;
        self.out_stream.play()?;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), Error> {
        self.playing = false;
        self.out_stream.pause()?;
        Ok(())
    }

    pub fn time_secs(&mut self) -> f64 {
        // Checking and creating new Instants too often will cause frequent jitter and unnecessary
        // lock acquiring, limit to once per second
        if self.time_absgp.elapsed() > Duration::new(1, 0) {
            // Check vorbis stream's approx position in samples
            let absgp = self.ogg_stream.read().unwrap().get_last_absgp();
            // If it's a new reading, store, and take note when it was taken
            if absgp != self.last_absgp {
                self.last_absgp = absgp;
                self.time_absgp = Instant::now();
            }
        }

        (if let Some(absgp) = self.last_absgp {
            // Samples per samples per second makes seconds
            absgp as f64 / self.sample_rate as f64
        } else {
            // Some default if vorbis position is unknown
            0f64
        }) + if self.is_playing() {
            // Add precision by adding time elapsed since last update from vorbis stream
            self.time_absgp.elapsed().as_nanos() as f64 / 1_000_000_000f64
        } else {
            // But don't add anything if paused, time shouldn't run when paused
            0f64
        }
    }
}
