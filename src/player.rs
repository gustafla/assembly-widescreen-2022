use alsa::{
    pcm::{self, Frames, State},
    PollDescriptors,
};
use lewton::inside_ogg::OggStreamReader;
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
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    FileAccess(#[from] std::io::Error),
    #[error("Failed to decode ogg vorbis file")]
    Decode(#[from] lewton::VorbisError),
}

// Playback buffering/latency size in frames
const BUF_SIZE: usize = 4096;
// FFT size in samples per channel, eg fft from stereo track reads double this number of i16s
const FFT_SIZE: usize = 1024;

pub struct Player {
    audio_data: Arc<Vec<i16>>,
    sample_rate: u32,
    channels: u8,
    sample_rate_channels: f32,
    len_secs: f32,
    playback_position: Arc<AtomicUsize>,
    playing: Arc<AtomicBool>,
    error_sync_flag: Arc<AtomicBool>,
    playback_thread: std::thread::JoinHandle<()>,
    start_time: Instant,
    pause_time: Instant,
    time_offset: Duration,
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

    fn start_alsa(
        audio_data: Arc<Vec<i16>>,
        sample_rate: u32,
        channels: u8,
        playback_position: Arc<AtomicUsize>,
        playing: Arc<AtomicBool>,
        error_sync_flag: Arc<AtomicBool>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let pcm = alsa::PCM::new("default", alsa::Direction::Playback, false)
                .map_err(|e| {
                    log::error!("Failed to initialize ALSA PCM playback");
                    e
                })
                .unwrap();

            let hwp = pcm::HwParams::any(&pcm).unwrap();
            hwp.set_channels(u32::from(channels)).unwrap();
            hwp.set_rate(sample_rate, alsa::ValueOr::Nearest).unwrap();
            hwp.set_format(pcm::Format::s16()).unwrap();
            hwp.set_access(pcm::Access::RWInterleaved).unwrap();
            let size = Frames::try_from(BUF_SIZE).unwrap();
            hwp.set_buffer_size(size).unwrap();
            hwp.set_period_size(size / 4, alsa::ValueOr::Nearest)
                .unwrap();
            pcm.hw_params(&hwp).unwrap();

            let hwp = pcm.hw_params_current().unwrap();
            let swp = pcm.sw_params_current().unwrap();
            let (bufsize, periodsize) = (
                hwp.get_buffer_size().unwrap(),
                hwp.get_period_size().unwrap(),
            );
            swp.set_start_threshold(bufsize - periodsize).unwrap();
            swp.set_avail_min(periodsize).unwrap();
            pcm.sw_params(&swp).unwrap();

            let got = hwp.get_rate().unwrap();
            if got != sample_rate {
                log::error!(
                    "Required sample rate {} is not supported. Got {}",
                    sample_rate,
                    got
                );
                panic!("Cannot play music on default ALSA PCM device");
            }

            let mut fds = pcm.get().unwrap();
            let io = pcm.io_i16().unwrap();

            loop {
                let avail = usize::try_from(match pcm.avail_update() {
                    Ok(n) => n,
                    Err(e) => {
                        log::error!("{}", e);
                        if let Some(errno) = e.errno() {
                            pcm.recover(errno as std::os::raw::c_int, true)
                                .map_err(|e| {
                                    log::error!("Cannot recover");
                                    e
                                })
                                .unwrap();
                        }
                        error_sync_flag.store(true, Ordering::Relaxed);
                        pcm.avail_update().unwrap()
                    }
                })
                .unwrap();

                if avail > 0 {
                    let avail_samples = avail * usize::from(channels);

                    // Load position and advance to next audio slice
                    // Might overflow in theory but not in realistic use
                    let pos = playback_position.fetch_add(avail_samples, Ordering::Relaxed);

                    // How many i16s can actually still be read
                    let remaining_samples = audio_data.len() - pos.min(audio_data.len());
                    let can_write_samples = avail_samples.min(remaining_samples);

                    if can_write_samples == 0 {
                        // Output silence after end to avoid underruns
                        io.writei(&[0; BUF_SIZE]).ok();
                    } else {
                        // Usually writes the right amount when no signal or underrun occurred,
                        // don't bother checking the return value :)
                        io.writei(&audio_data[pos..][..can_write_samples]).ok();
                    }
                }

                match (pcm.state(), playing.load(Ordering::Relaxed)) {
                    (State::Running, true) => {}
                    (State::Running, false) => pcm.pause(true).unwrap(),
                    (State::Prepared, true) => pcm.start().unwrap(),
                    (State::Prepared, false) => std::thread::park(),
                    (State::Paused, true) => pcm.pause(false).unwrap(),
                    (State::Paused, false) => std::thread::park(),
                    _ => continue, // Try to recover, skip polling
                }

                alsa::poll::poll(&mut fds, 100).unwrap();
            }
        })
    }

    pub fn new(ogg_path: impl AsRef<Path>) -> Result<Self, Error> {
        log::info!("Loading {}", ogg_path.as_ref().display());

        // Read and decode ogg file
        let ogg_file = File::open(ogg_path)?;
        let ogg_reader = BufReader::new(ogg_file);
        let (audio_data, sample_rate, channels) = Self::decode_ogg(ogg_reader)?;
        let sample_rate_channels = (sample_rate * u32::from(channels)) as f32;
        let len_secs = audio_data.len() as f32 / sample_rate_channels;

        let playback_position = Arc::new(AtomicUsize::new(0));
        let playing = Arc::new(AtomicBool::new(false));
        let error_sync_flag = Arc::new(AtomicBool::new(false));

        let playback_thread = Self::start_alsa(
            audio_data.clone(),
            sample_rate,
            channels,
            playback_position.clone(),
            playing.clone(),
            error_sync_flag.clone(),
        );

        // Initialize FFT
        let mut fft_planner = FftPlanner::new();
        let fft = fft_planner.plan_fft_forward(FFT_SIZE);
        let fft_scratch = vec![Complex::new(0., 0.); fft.get_inplace_scratch_len()];

        let time = Instant::now();

        Ok(Self {
            audio_data,
            sample_rate,
            channels,
            sample_rate_channels,
            len_secs,
            playback_position,
            playing,
            error_sync_flag,
            playback_thread,
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
        self.playing.load(Ordering::Relaxed)
    }

    pub fn play(&mut self) {
        self.time_offset = self.pos_to_duration(self.playback_position.load(Ordering::Relaxed));
        self.start_time = Instant::now();
        self.playing.store(true, Ordering::Relaxed);
        self.playback_thread.thread().unpark();
    }

    pub fn pause(&mut self) {
        self.pause_time = Instant::now();
        self.playing.store(false, Ordering::Relaxed);
    }

    pub fn time_secs(&mut self) -> f32 {
        let timer_secs = (if self.is_playing() {
            if self.error_sync_flag.fetch_and(false, Ordering::Relaxed) {
                log::info!("ALSA PCM output errors, trying to sync");
                self.play();
            }
            self.start_time.elapsed()
        } else {
            self.pause_time.duration_since(self.start_time)
        } + self.time_offset)
            .as_micros() as f32
            / 1_000_000f32;

        timer_secs.min(self.len_secs)
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
        self.time_offset = self.pos_to_duration(pos);
        let time = Instant::now();
        self.start_time = time;
        self.pause_time = time;

        // Unpark if needed
        if self.is_playing() {
            self.playback_thread.thread().unpark();
        }
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

        // Limit to audio data range
        pos = pos
            .min(self.audio_data.len() - FFT_SIZE * usize::from(self.channels) - 1)
            .max(0);

        // Align to channel
        pos -= pos % usize::from(self.channels);

        // Take the audio data slice and convert to windowed complex number Vec
        let fft_size_f32 = FFT_SIZE as f32;
        let mut fft_buffer: Vec<_> = self.audio_data[pos..]
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
