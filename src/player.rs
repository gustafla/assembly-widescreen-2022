use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream,
};
use lewton::inside_ogg::OggStreamReader;
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
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
    out_stream: Stream,
    last_pos: Arc<AtomicU64>,
    millis_at_pos: Arc<AtomicU64>,
    start_time: Instant,
    sample_rate: u32,
}

impl Player {
    pub fn new(ogg_path: impl AsRef<Path>) -> Result<Self, Error> {
        log::info!("Loading {}", ogg_path.as_ref().display());
        let ogg_file = File::open(ogg_path)?;
        let ogg_reader = BufReader::new(ogg_file);
        let mut ogg_stream = OggStreamReader::new(ogg_reader)?;
        let sample_rate = ogg_stream.ident_hdr.audio_sample_rate;

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
        let last_pos = Arc::new(AtomicU64::new(0));
        let millis_at_pos = Arc::new(AtomicU64::new(0));

        // Stream with cpal
        let pos = last_pos.clone();
        let millis = millis_at_pos.clone();
        let start_time = Instant::now();
        let out_stream = device.build_output_stream(
            &config.into(),
            move |output_buf: &mut [i16], _: &cpal::OutputCallbackInfo| {
                let mut output_written = 0;

                loop {
                    pos.store(
                        ogg_stream
                            .get_last_absgp()
                            .expect("No ogg position available, cannot sync"),
                        Ordering::Relaxed,
                    );
                    millis.store(
                        u64::try_from(Instant::now().duration_since(start_time).as_millis())
                            .expect("Music track too long"),
                        Ordering::Relaxed,
                    );

                    // Slice to current positions
                    let packet = &packet_buf[packet_read..];
                    let output = &mut output_buf[output_written..];

                    // Copy audio from previous vorbis packet
                    if packet.len() >= output.len() {
                        output.copy_from_slice(&packet[..output.len()]);
                        packet_read += output.len();
                        return;
                    } else {
                        (&mut output[..packet.len()]).copy_from_slice(packet);
                        output_written += packet.len();
                    };

                    // When necessary, decode more
                    if let Some(new_packet) = ogg_stream.read_dec_packet_itl().unwrap() {
                        packet_buf = new_packet;
                    } else {
                        // Or play silence if at EOS
                        for sample in packet_buf.iter_mut() {
                            *sample = 0;
                        }
                    }
                    packet_read = 0;
                }
            },
            move |e| {
                panic!("cpal error {}", e);
            },
        )?;

        Ok(Self {
            out_stream,
            last_pos,
            millis_at_pos,
            start_time,
            sample_rate,
        })
    }

    pub fn play(&self) -> Result<(), Error> {
        self.out_stream.play()?;
        Ok(())
    }

    pub fn pause(&self) -> Result<(), Error> {
        self.out_stream.pause()?;
        Ok(())
    }

    pub fn time_secs(&self) -> f64 {
        let millis_since_pos =
            u64::try_from(Instant::now().duration_since(self.start_time).as_millis())
                .expect("Music track too long")
                - self.millis_at_pos.load(Ordering::Relaxed);
        dbg!(millis_since_pos);
        self.last_pos.load(Ordering::Relaxed) as f64 / self.sample_rate as f64
            + millis_since_pos as f64 / 1000f64
    }
}
