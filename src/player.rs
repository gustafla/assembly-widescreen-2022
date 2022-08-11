use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lewton::inside_ogg::OggStreamReader;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{
    convert::TryFrom,
    io::{Read, Seek},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

// Playback buffering/latency size in frames
const BUF_SIZE: u32 = 4096;
// FFT size in samples per channel, eg fft from stereo track reads double this number of i16s
const FFT_SIZE: usize = 1024;

#[derive(Clone, Default)]
struct SharedParams {
    audio_data: Arc<Vec<i16>>,
    playback_position: Arc<AtomicUsize>,
    playing: Arc<AtomicBool>,
    error_sync_flag: Arc<AtomicBool>,
}

struct StartParams {
    device: cpal::Device,
    config: cpal::StreamConfig,
    shared: SharedParams,
}

pub struct Player {
    shared: SharedParams,
    sample_rate: u32,
    channels: u8,
    sample_rate_channels: f32,
    len_secs: f32,
    playback_stream: cpal::Stream,
    start_time: Instant,
    pause_time: Instant,
    time_offset: Duration,
    fft: Arc<dyn Fft<f32>>,
    fft_scratch: Vec<Complex<f32>>,
}

impl Player {
    fn decode_ogg(reader: impl Read + Seek) -> (Arc<Vec<i16>>, u32, u8) {
        let mut ogg_stream_reader =
            OggStreamReader::new(reader).expect("Failed to decode ogg stream. This is a bug.");

        // Because lewton doesn't have time seek at the time of writing,
        // I'm gonna waste memory with full uncompressed audio
        // This also makes FFT pretty straightforward
        let mut audio_data = Vec::new();
        loop {
            match ogg_stream_reader.read_dec_packet_itl() {
                Ok(Some(packet)) => audio_data.extend_from_slice(&packet),
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    log::error!("{}", e);
                    panic!("Failed to decode ogg stream. This is a bug.");
                }
            }
        }

        (
            Arc::new(audio_data),
            ogg_stream_reader.ident_hdr.audio_sample_rate,
            ogg_stream_reader.ident_hdr.audio_channels,
        )
    }

    fn conf_meets_specs(
        conf: &cpal::SupportedStreamConfigRange,
        sample_rate: u32,
        channels: u8,
    ) -> bool {
        conf.channels() == channels.into()
            && conf.min_sample_rate() <= cpal::SampleRate(sample_rate)
            && conf.max_sample_rate() >= cpal::SampleRate(sample_rate)
    }

    fn init(
        sample_rate: u32,
        channels: u8,
    ) -> Result<(cpal::Device, cpal::StreamConfig, cpal::SampleFormat)> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .context("Unable to find default audio output device")?;

        // Find best configuration from device's supported configs
        let supported_config = if let Some(supported_config) = device
            .supported_output_configs()
            .context("Failed to query audio device parameters")?
            // Try to find suitable i16 output config
            .find(|conf| {
                Self::conf_meets_specs(conf, sample_rate, channels)
                    && conf.sample_format() == cpal::SampleFormat::I16
            }) {
            supported_config
        } else {
            // If no i16 output format, try again for any suitable and use sample conversion
            log::warn!("Default audio output device does not support i16 sample format");
            device
                .supported_output_configs()
                .context("Failed to query audio device parameters")?
                .find(|conf| Self::conf_meets_specs(conf, sample_rate, channels))
                .context("Audio device does not support required parameters")?
        }
        .with_sample_rate(cpal::SampleRate(sample_rate));

        let format = supported_config.sample_format();
        let buffer_size = supported_config.buffer_size().clone();
        let mut config: cpal::StreamConfig = supported_config.into();

        // Try to set small buffer size for minimal latency
        match buffer_size {
            cpal::SupportedBufferSize::Range { min, max } if min <= BUF_SIZE && max >= BUF_SIZE => {
                config.buffer_size = cpal::BufferSize::Fixed(BUF_SIZE);
                log::info!("Using audio output buffer size {}", BUF_SIZE);
            }
            _ => {
                log::warn!("Unable to set audio output buffer size, demo might run out of sync");
            }
        }

        Ok((device, config, format))
    }

    fn start<T: cpal::Sample>(p: StartParams) -> Result<cpal::Stream> {
        let stream = p
            .device
            .build_output_stream(
                &p.config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    let avail = data.len();

                    // Load position and advance to next audio slice
                    // Might overflow in theory but not in realistic use
                    let pos = p
                        .shared
                        .playback_position
                        .fetch_add(avail, Ordering::Relaxed);

                    // How many i16s can actually still be read
                    let remaining_samples =
                        p.shared.audio_data.len() - pos.min(p.shared.audio_data.len());
                    let can_write_samples = avail.min(remaining_samples);

                    if can_write_samples == 0 || !p.shared.playing.load(Ordering::Relaxed) {
                        // Output silence after end to avoid underruns
                        for sample in data.iter_mut() {
                            *sample = cpal::Sample::from(&0.);
                        }
                    } else {
                        for (i, sample) in data.iter_mut().enumerate().take(can_write_samples) {
                            *sample = cpal::Sample::from(&p.shared.audio_data[pos..][i]);
                        }
                    }
                },
                move |err| {
                    p.shared.error_sync_flag.store(true, Ordering::Relaxed);
                    log::error!("{}", err);
                },
            )
            .context("Failed to build audio output stream")?;

        Ok(stream)
    }

    pub fn new(ogg_path: impl AsRef<Path>) -> Result<Self> {
        log::info!("Loading {}", ogg_path.as_ref().display());

        // Read and decode ogg file
        let (audio_data, sample_rate, channels) = {
            #[cfg(debug_assertions)]
            {
                match std::fs::File::open(
                    std::path::PathBuf::from(crate::RESOURCES_PATH).join(ogg_path),
                ) {
                    Ok(file) => Self::decode_ogg(std::io::BufReader::new(file)),
                    Err(e) => {
                        log::warn!("Cannot load audio: {}", e);
                        (Default::default(), 48000, 2)
                    }
                }
            }
            #[cfg(not(debug_assertions))]
            {
                let ogg_reader = std::io::Cursor::new(
                    crate::RESOURCES_DIR
                        .get_file(ogg_path)
                        .expect("File not present in binary. This is a bug.")
                        .contents(),
                );
                Self::decode_ogg(ogg_reader)
            }
        };
        let sample_rate_channels = (sample_rate * u32::from(channels)) as f32;
        let len_secs = audio_data.len() as f32 / sample_rate_channels;

        // Initialize audio device
        let (device, config, format) = Self::init(sample_rate, channels)?;

        // Start audio output stream
        let shared = SharedParams {
            audio_data,
            ..Default::default()
        };
        let start_parm = StartParams {
            device,
            config,
            shared: shared.clone(),
        };
        let playback_stream = match format {
            cpal::SampleFormat::I16 => Self::start::<i16>(start_parm)?,
            cpal::SampleFormat::U16 => Self::start::<u16>(start_parm)?,
            cpal::SampleFormat::F32 => Self::start::<f32>(start_parm)?,
        };

        // Start paused
        playback_stream
            .pause()
            .unwrap_or_else(|e| log::error!("Cannot pause audio output stream: {}", e));

        // Initialize FFT
        let mut fft_planner = FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(FFT_SIZE);
        let fft_scratch = vec![Complex::new(0., 0.); fft.get_inplace_scratch_len()];

        let time = Instant::now();

        Ok(Self {
            shared,
            sample_rate,
            channels,
            sample_rate_channels,
            len_secs,
            playback_stream,
            start_time: time,
            pause_time: time,
            time_offset: Duration::new(0, 0),
            fft,
            fft_scratch,
        })
    }

    pub fn len_secs(&self) -> f32 {
        self.len_secs
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn play(&mut self) {
        self.time_offset =
            self.pos_to_duration(self.shared.playback_position.load(Ordering::Relaxed));
        self.start_time = Instant::now();
        self.shared.playing.store(true, Ordering::Relaxed);
        self.playback_stream
            .play()
            .unwrap_or_else(|e| log::error!("Cannot play audio output stream: {}", e));
    }

    pub fn pause(&mut self) {
        self.pause_time = Instant::now();
        self.shared.playing.store(false, Ordering::Relaxed);
        self.playback_stream
            .pause()
            .unwrap_or_else(|e| log::error!("Cannot pause audio output stream: {}", e));
    }

    pub fn time_secs(&mut self) -> f32 {
        let timer_secs = (if self.is_playing() {
            if self
                .shared
                .error_sync_flag
                .fetch_and(false, Ordering::Relaxed)
            {
                log::info!("Audio output errors, trying to sync");
                self.play();
            }
            self.start_time.elapsed()
        } else {
            self.pause_time.duration_since(self.start_time)
        } + self.time_offset)
            .as_secs_f32();

        // Hack to enable development without audio track
        #[cfg(debug_assertions)]
        if self.shared.audio_data.is_empty() {
            return timer_secs;
        }

        timer_secs.min(self.len_secs)
    }

    pub fn seek(&mut self, secs: f32) {
        // Calculate new playback position
        let mut pos = (secs * self.sample_rate_channels) as usize;

        // Align to channel
        pos -= pos % usize::from(self.channels);

        // Set new position and update timing etc
        self.shared.playback_position.store(pos, Ordering::Relaxed);
        self.time_offset = self.pos_to_duration(pos);
        let time = Instant::now();
        self.start_time = time;
        self.pause_time = time;
    }

    fn pos_to_duration(&self, pos: usize) -> Duration {
        let sample_rate_channels = u64::from(self.sample_rate) * u64::from(self.channels);
        let pos = u64::try_from(pos).unwrap();
        Duration::new(
            pos / sample_rate_channels,
            u32::try_from(((pos % sample_rate_channels) * 1_000_000_000) / sample_rate_channels)
                .unwrap(),
        )
    }

    /// Compute average Power Spectral Density of bass (30-300Hz)
    pub fn bass_psd(&mut self, at_secs: f32) -> f32 {
        // Compute the position
        let mut pos = (at_secs * self.sample_rate_channels) as usize;

        if pos >= self.shared.audio_data.len() {
            return 0.;
        }

        // Limit to audio data range
        pos = pos
            .min(self.shared.audio_data.len() - FFT_SIZE * usize::from(self.channels) - 1)
            .max(0);

        // Align to channel
        pos -= pos % usize::from(self.channels);

        // Take the audio data slice and convert to windowed complex number Vec
        let fft_size_f32 = FFT_SIZE as f32;
        let mut fft_buffer: Vec<_> = self.shared.audio_data[pos..]
            [..FFT_SIZE * usize::from(self.channels)]
            .chunks(self.channels.into())
            .enumerate()
            .map(|(n, all_channels_sample)| {
                // First channel mono
                let sample_f32 = all_channels_sample[0] as f32 / i16::MAX as f32;
                Complex::new(
                    // Hann window function
                    ((std::f32::consts::PI * n as f32) / fft_size_f32)
                        .sin()
                        .powi(2)
                        * sample_f32,
                    0.,
                )
            })
            .collect();

        // Compute FFT
        self.fft
            .process_with_scratch(&mut fft_buffer, &mut self.fft_scratch);

        // Compute average of bass bins
        let freq_per_bin = (self.sample_rate as f32 / 2.) / fft_size_f32;
        let start = (30. / freq_per_bin).floor() as usize;
        let end = (300. / freq_per_bin).ceil() as usize;
        let normalization_scale = 1. / fft_size_f32.sqrt();
        fft_buffer[start..end]
            .iter()
            // Normalize (see https://docs.rs/rustfft/5.0.1/rustfft/#normalization)
            .map(|complex| (complex * normalization_scale).norm())
            .sum::<f32>()
            / (end - start) as f32
    }
}
