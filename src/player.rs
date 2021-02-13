use lewton::inside_ogg::OggStreamReader;
use parking_lot::Mutex;
use pulse::{
    def::BufferAttr,
    sample::{Format, Spec},
    stream::Direction,
};
use pulse_simple::Simple;
use std::convert::TryInto;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    FileAccess(#[from] std::io::Error),
    #[error("Failed to decode ogg vorbis file")]
    Decode(#[from] lewton::VorbisError),
    #[error("PulseAudio error: {0}")]
    PulseAudio(#[from] pulse::error::PAErr),
}

// Playback buffering/latency size in samples
// Assuming 44100Hz stereo, 256 samples is about a frame of latency
const BUF_SIZE: usize = 256;
// FFT size in samples per channel, eg fft from stereo track reads double this number of i16s
const FFT_SIZE: usize = 1024;

pub struct Player {
    audio_data: Arc<Vec<i16>>,
    playback_position: Arc<Mutex<usize>>,
    sample_rate: usize,
    channels: usize,
    start_time: Instant,
    pause_time: Instant,
    time_offset: Duration,
    playing: Arc<AtomicBool>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_scratch: Vec<num_complex::Complex<f32>>,
}

impl Player {
    fn decode_ogg(reader: impl Read + Seek) -> Result<(Arc<Vec<i16>>, usize, usize), Error> {
        let mut ogg_stream_reader = OggStreamReader::new(reader)?;

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
                    return Err(e.into());
                }
            }
        }

        Ok((
            Arc::new(audio_data),
            ogg_stream_reader.ident_hdr.audio_sample_rate as usize,
            ogg_stream_reader.ident_hdr.audio_channels as usize,
        ))
    }

    pub fn new(ogg_path: impl AsRef<Path>) -> Result<Self, Error> {
        log::info!("Loading {}", ogg_path.as_ref().display());

        // Read and decode ogg file
        let ogg_file = File::open(ogg_path)?;
        let ogg_reader = BufReader::new(ogg_file);
        let (audio_data, sample_rate, channels) = Self::decode_ogg(ogg_reader)?;

        // Initialize FFT
        let mut fft_planner = rustfft::FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(FFT_SIZE);
        let fft_scratch = vec![num_complex::Complex::new(0., 0.); fft.get_inplace_scratch_len()];

        // Initialize libpulse_simple
        let spec = Spec {
            format: Format::S16NE, // Signed 16-bit in native endian
            channels: channels.try_into().unwrap(),
            rate: sample_rate.try_into().unwrap(),
        };
        let buffer_attr = BufferAttr {
            maxlength: std::u32::MAX,
            tlength: (BUF_SIZE * std::mem::size_of::<i16>()).try_into().unwrap(),
            prebuf: std::u32::MAX,
            minreq: std::u32::MAX,
            fragsize: std::u32::MAX,
        };
        let simple = Simple::new(
            None,   // Default server
            "demo", // Application name
            Direction::Playback,
            None,    // Default device
            "Music", // Description
            &spec,   // Sample format
            None,    // Default channel map
            Some(&buffer_attr),
        )?;

        let playback_position = Arc::new(Mutex::new(0));
        let playing = Arc::new(AtomicBool::new(false));

        // Start a thread for music streaming (from RAM to pulse)
        {
            let audio_data = audio_data.clone();
            let playback_position = playback_position.clone();
            let playing = playing.clone();
            std::thread::spawn(move || loop {
                {
                    if playing.load(Ordering::SeqCst) {
                        let mut playback_position = playback_position.lock();
                        let samples = BUF_SIZE.min(audio_data.len() - *playback_position);
                        if samples > 0 {
                            let bytes = samples * std::mem::size_of::<i16>();
                            unsafe {
                                let ptr = audio_data.as_ptr().add(*playback_position);
                                let buf_slice = std::slice::from_raw_parts(ptr as *const u8, bytes);
                                simple.write(buf_slice).unwrap();
                            }
                            *playback_position += samples;
                            continue;
                        }
                    }

                    // When nothing to play, sleep for about a frame (assuming 60FPS)
                    std::thread::sleep(Duration::new(0, 1_000_000_000 / 60));
                }
            });
        }

        let time = Instant::now();

        Ok(Self {
            audio_data,
            playback_position,
            sample_rate,
            channels,
            start_time: time,
            pause_time: time,
            time_offset: Duration::new(0, 0),
            playing,
            fft,
            fft_scratch,
        })
    }

    pub fn is_at_end(&self) -> bool {
        *self.playback_position.lock() == self.audio_data.len()
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::SeqCst)
    }

    pub fn play(&mut self) {
        self.time_offset = self.pos_to_duration(*self.playback_position.lock());
        self.start_time = Instant::now();
        self.playing.store(true, Ordering::SeqCst);
    }

    pub fn pause(&mut self) {
        self.pause_time = Instant::now();
        self.playing.store(false, Ordering::SeqCst);
    }

    pub fn time_secs(&mut self) -> f64 {
        // Naive approach, just tell where the output stream actually is
        // This is required to avoid telling time over the length of the music track
        if self.is_at_end() {
            return self.pos_to_duration(self.audio_data.len()).as_secs_f64();
        }

        // Otherwise resort to Rust's timers for smoother frames
        (if self.is_playing() {
            self.start_time.elapsed()
        } else {
            self.pause_time.duration_since(self.start_time)
        } + self.time_offset)
            .as_nanos() as f64
            / 1_000_000_000f64
    }

    pub fn seek(&mut self, secs: f64) {
        // Calculate new playback position
        let sample_rate_channels = (self.sample_rate * self.channels) as f64;
        let mut pos = (secs * sample_rate_channels) as usize;

        // Align to channel
        pos -= pos % self.channels;

        // Limit position to avoid buffer over-read panic
        pos = pos.min(self.audio_data.len());

        // Set new position and update timing etc
        *self.playback_position.lock() = pos;
        self.time_offset = self.pos_to_duration(pos);
        let time = Instant::now();
        self.start_time = time;
        self.pause_time = time;
    }

    fn pos_to_duration(&self, pos: usize) -> Duration {
        let sample_rate_channels = self.sample_rate * self.channels;
        Duration::new(
            (pos / sample_rate_channels) as u64,
            (((pos % sample_rate_channels) * 1_000_000_000) / sample_rate_channels) as u32,
        )
    }

    /// Compute average Power Spectral Density of bass (30-300Hz)
    pub fn bass_psd(&mut self, at_secs: f64) -> f32 {
        // Compute the position
        let mut pos = (at_secs * self.sample_rate as f64 * self.channels as f64) as usize;

        // Limit to audio data range
        pos = pos
            .min(self.audio_data.len() - FFT_SIZE * self.channels - 1)
            .max(0);

        // Align to channel
        pos -= pos % self.channels;

        // Take the audio data slice and convert to windowed complex number Vec
        let fft_size_f32 = FFT_SIZE as f32;
        let mut fft_buffer: Vec<_> = self.audio_data[pos..][..FFT_SIZE * self.channels]
            .chunks(self.channels)
            .enumerate()
            .map(|(n, all_channels_sample)| {
                // First channel mono
                let sample_f32 = all_channels_sample[0] as f32 / i16::MAX as f32;
                num_complex::Complex::new(
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
