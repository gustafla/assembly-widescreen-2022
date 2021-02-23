use lewton::inside_ogg::OggStreamReader;
use pulse::{
    def::BufferAttr,
    sample::{Format, Spec},
    stream::Direction,
};
use pulse_simple::Simple;
use rustfft::{num_complex::Complex, Fft, FftPlanner};
use std::{
    convert::TryFrom,
    fs::File,
    io::{BufReader, Read, Seek},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::JoinHandle,
};
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
const BUF_SIZE: usize = 1024;
// FFT size in frames, eg fft from stereo track reads double this number of i16s
const FFT_SIZE: usize = 1024;

pub struct Player {
    audio_data: Arc<Vec<i16>>,
    sample_rate: u32,
    channels: u8,
    sample_rate_channels: f32,
    len_secs: f32,
    playback_position: Arc<AtomicUsize>,
    playing: Arc<AtomicBool>,
    playback_thread: JoinHandle<()>,
    fft: Arc<dyn Fft<f32>>,
    fft_scratch: Vec<Complex<f32>>,
}

impl Player {
    fn decode_ogg(reader: impl Read + Seek) -> Result<(Arc<Vec<i16>>, u32, u8), Error> {
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
            ogg_stream_reader.ident_hdr.audio_sample_rate,
            ogg_stream_reader.ident_hdr.audio_channels,
        ))
    }

    fn start_pulse(
        title: &str,
        audio_data: Arc<Vec<i16>>,
        sample_rate: u32,
        channels: u8,
        playback_position: Arc<AtomicUsize>,
        playing: Arc<AtomicBool>,
    ) -> Result<JoinHandle<()>, Error> {
        let spec = Spec {
            format: Format::S16NE, // Signed 16-bit in native endian
            channels,
            rate: sample_rate,
        };
        let buf_len = u32::try_from(BUF_SIZE * std::mem::size_of::<i16>()).unwrap();
        let buffer_attr = BufferAttr {
            maxlength: buf_len,
            tlength: buf_len,
            prebuf: std::u32::MAX,
            minreq: std::u32::MAX,
            fragsize: std::u32::MAX,
        };
        let pulseaudio = Simple::new(
            None,  // Default server
            title, // Application name
            Direction::Playback,
            None,    // Default device
            "Music", // Description
            &spec,   // Sample format
            None,    // Default channel map
            Some(&buffer_attr),
        )?;

        // Start a thread for music streaming (from RAM to pulse)
        Ok(std::thread::spawn(move || loop {
            {
                if playing.load(Ordering::Relaxed) {
                    // Load position and advance to next audio slice
                    // Might overflow in theory but not in realistic use
                    let pos = playback_position.fetch_add(BUF_SIZE, Ordering::Relaxed);
                    // How many samples can actually still be read
                    let samples = BUF_SIZE.min(audio_data.len() - pos.min(audio_data.len()));
                    // If any, let's play them
                    if samples > 0 {
                        let slice = &audio_data[pos..][..samples];
                        pulseaudio.write(bytemuck::cast_slice(slice)).unwrap();
                        continue;
                    }
                }

                // When nothing to play, park
                std::thread::park();
            }
        }))
    }

    pub fn new(ogg_path: impl AsRef<Path>, title: &str) -> Result<Self, Error> {
        log::info!("Loading {}", ogg_path.as_ref().display());

        // Read and decode ogg file
        let ogg_file = File::open(ogg_path)?;
        let ogg_reader = BufReader::new(ogg_file);
        let (audio_data, sample_rate, channels) = Self::decode_ogg(ogg_reader)?;
        let sample_rate_channels = (sample_rate * u32::from(channels)) as f32;
        let len_secs = audio_data.len() as f32 / sample_rate_channels;

        // Initialize libpulse_simple
        let playback_position = Arc::new(AtomicUsize::new(0));
        let playing = Arc::new(AtomicBool::new(false));
        let playback_thread = Self::start_pulse(
            title,
            audio_data.clone(),
            sample_rate,
            channels,
            playback_position.clone(),
            playing.clone(),
        )?;

        // Initialize FFT
        let mut fft_planner = FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(FFT_SIZE);
        let fft_scratch = vec![Complex::new(0., 0.); fft.get_inplace_scratch_len()];

        Ok(Self {
            audio_data,
            sample_rate,
            channels,
            sample_rate_channels,
            len_secs,
            playback_position,
            playing,
            playback_thread,
            fft,
            fft_scratch,
        })
    }

    pub fn len_secs(&self) -> f32 {
        self.len_secs
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn play(&mut self) {
        self.playing.store(true, Ordering::Relaxed);
        self.playback_thread.thread().unpark();
    }

    pub fn pause(&mut self) {
        self.playing.store(false, Ordering::Relaxed);
    }

    pub fn time_secs(&mut self) -> f32 {
        (self.playback_position.load(Ordering::Relaxed) as f32 / self.sample_rate_channels)
            .min(self.len_secs)
    }

    pub fn seek(&mut self, secs: f32) {
        // Calculate new playback position
        let mut pos = (secs * self.sample_rate_channels) as usize;

        // Align to channel
        pos -= pos % usize::from(self.channels);

        // Limit position to avoid buffer over-read panic
        pos = pos.min(self.audio_data.len());

        // Set new position and update timing etc
        self.playback_position.store(pos, Ordering::Relaxed);

        // Unpark playback if needed
        if self.is_playing() {
            self.playback_thread.thread().unpark();
        }
    }

    /// Compute average Power Spectral Density of bass (30-300Hz)
    pub fn bass_psd(&mut self) -> f32 {
        // Load the position
        let pos = self
            .playback_position
            .load(Ordering::Relaxed)
            .min(self.audio_data.len() - FFT_SIZE * usize::from(self.channels) - 1);

        // Take the audio data slice and convert to windowed complex number Vec
        let fft_size_f32 = FFT_SIZE as f32;
        let mut fft_buffer: Vec<_> = self.audio_data[pos..]
            [..FFT_SIZE * usize::from(self.channels)]
            .chunks(usize::from(self.channels))
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
