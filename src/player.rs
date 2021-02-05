use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream,
};
use lewton::inside_ogg::OggStreamReader;
use parking_lot::Mutex;
use std::fs::File;
use std::io::{BufReader, Read, Seek};
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
    #[error("Failed to decode ogg vorbis file")]
    Decode(#[from] lewton::VorbisError),
    #[error("No audio output devices")]
    NoAudioOutputDevices,
    #[error("Failed to query audio device")]
    QueryAudioOutputSupport(#[from] cpal::SupportedStreamConfigsError),
    #[error("Default audio device doesn't support required configuration")]
    NoSupportForRequiredConfiguration,
    #[error("Ogg file doesn't contain audio")]
    NoAudioStreamInFile,
    #[error("Failed to build audio output stream")]
    BuildStream(#[from] cpal::BuildStreamError),
    #[error("Failed to start audio output stream")]
    PlayStream(#[from] cpal::PlayStreamError),
    #[error("Failed to pause audio output stream")]
    PauseStream(#[from] cpal::PauseStreamError),
}

const FFT_SIZE: usize = 4096;

pub struct Player {
    audio_data: Arc<Vec<i16>>,
    playback_position: Arc<Mutex<usize>>,
    out_stream: Stream,
    sample_rate: usize,
    channels: usize,
    start_time: Instant,
    pause_time: Instant,
    time_offset: Duration,
    error_sync_rx: mpsc::Receiver<()>,
    playing: bool,
    at_end: Arc<AtomicBool>,
    fft: Arc<dyn rustfft::Fft<f32>>,
    fft_scratch: Vec<num_complex::Complex<f32>>,
}

impl Player {
    fn decode_ogg(reader: impl Read + Seek) -> Result<(Arc<Vec<i16>>, usize, usize), Error> {
        let mut ogg_stream_reader = OggStreamReader::new(reader)?;

        // Because lewton doesn't have time seek at the time of writing,
        // I'm gonna waste memory with full uncompressed audio
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

        // Initialize cpal output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(Error::NoAudioOutputDevices)?;
        let mut supported_configs_range = device.supported_output_configs()?;
        let config = supported_configs_range
            .find(|c| {
                c.channels() as usize == channels
                    && c.min_sample_rate().0 <= sample_rate as u32
                    && c.max_sample_rate().0 >= sample_rate as u32
                    && c.sample_format() == cpal::SampleFormat::I16
            })
            .ok_or(Error::NoSupportForRequiredConfiguration)?
            .with_sample_rate(SampleRate(sample_rate as u32));

        let playback_position = Arc::new(Mutex::new(0));
        let at_end = Arc::new(AtomicBool::new(false));
        let (error_sync_tx, error_sync_rx) = mpsc::channel();

        // Stream with cpal
        let out_stream = {
            let audio_data = audio_data.clone();
            let at_end = at_end.clone();
            let playback_position = playback_position.clone();
            let mut errors = 0;
            device.build_output_stream(
                &config.into(),
                move |output_buf: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    // Lock position mutex during critical section
                    let mut playback_position = playback_position.lock();

                    // TODO Callback buffer fill, notify at_end
                    let audio_data = &audio_data[*playback_position..];

                    if audio_data.len() >= output_buf.len() {
                        output_buf.copy_from_slice(&audio_data[..output_buf.len()]);
                        *playback_position += output_buf.len();
                    } else {
                        output_buf[..audio_data.len()].copy_from_slice(&audio_data);
                        output_buf[audio_data.len()..]
                            .iter_mut()
                            .for_each(|value| *value = 0);
                        *playback_position += audio_data.len();
                        at_end.store(true, Ordering::SeqCst);
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
            audio_data,
            playback_position,
            out_stream,
            sample_rate,
            channels,
            start_time: time,
            pause_time: time,
            time_offset: Duration::new(0, 0),
            error_sync_rx,
            playing: false,
            at_end,
            fft,
            fft_scratch,
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
            self.time_offset = self.pos_to_duration(*self.playback_position.lock());
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
        // If playback errors (underruns?) have happened, sync with the stream position
        if self.error_sync_rx.try_recv().is_ok() {
            self.time_offset = self.pos_to_duration(*self.playback_position.lock());
            let time = Instant::now();
            self.start_time = time;
            self.pause_time = time;
        }

        (if self.is_playing() {
            self.start_time.elapsed()
        } else {
            self.pause_time.duration_since(self.start_time)
        } + self.time_offset)
            .as_nanos() as f64
            / 1_000_000_000f64
    }

    pub fn seek(&mut self, secs: f64) {
        let sample_rate_channels = (self.sample_rate * self.channels) as f64;
        let mut pos = (secs * sample_rate_channels) as usize;
        pos -= pos % self.channels; // Align to channel
        *self.playback_position.lock() = pos;
        self.time_offset = self.pos_to_duration(pos);
        let time = Instant::now();
        self.start_time = time;
        self.pause_time = time;
    }

    pub fn fft(&mut self, secs: f64) -> Vec<num_complex::Complex<f32>> {
        // Compute the position
        let mut pos = (secs * self.sample_rate as f64 * self.channels as f64) as usize;

        // Limit to audio data range
        pos = pos
            .min(self.audio_data.len() - FFT_SIZE * self.channels - 1)
            .max(0);

        // Align to channel
        pos -= pos % self.channels;

        // Take the audio data slice and convert to windowed complex number Vec
        let mut fft_buffer: Vec<_> = self.audio_data[pos..][..FFT_SIZE * self.channels]
            .chunks(self.channels)
            .enumerate()
            .map(|(n, all_channels_sample)| {
                // All channels average
                //num_complex::Complex::new(
                //    all_channels_sample
                //        .iter()
                //        .map(|&s| s as f32 / i16::MAX as f32)
                //        .sum::<f32>()
                //        / self.channels as f32,
                //    0.,
                //)

                // First channel mono
                let normalized = all_channels_sample[0] as f32 / i16::MAX as f32;
                num_complex::Complex::new(Self::fft_window(n) * normalized, 0.)
            })
            .collect();

        // Compute FFT
        self.fft
            .process_with_scratch(&mut fft_buffer, &mut self.fft_scratch);

        // Normalize
        for output in &mut fft_buffer {
            *output /= (FFT_SIZE as f32).sqrt();
        }

        fft_buffer
    }

    fn fft_window(n: usize) -> f32 {
        // Hann
        ((std::f32::consts::PI * n as f32) / FFT_SIZE as f32).sin().powi(2)
    }

    fn pos_to_duration(&self, pos: usize) -> Duration {
        let sample_rate_channels = self.sample_rate * self.channels;
        Duration::new(
            (pos / sample_rate_channels) as u64,
            (((pos % sample_rate_channels) * 1_000_000_000) / sample_rate_channels) as u32,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_works() {
        assert!((Player::fft_window(0) - 0.).abs() <= f32::EPSILON);
        assert!((Player::fft_window(FFT_SIZE/2) - 1.).abs() <= f32::EPSILON);
        assert!((Player::fft_window(FFT_SIZE) - 0.).abs() <= f32::EPSILON);
    }
}
