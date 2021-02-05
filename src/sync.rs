use crate::Player;
use std::ops::Range;

const TRACKS_FILE: &str = "resources/tracks.bin";

pub struct Sync {
    row: f64,
    beats_per_sec: f64,
    rows_per_beat: f64,
    fft: Option<crate::player::FftOutput>,
    #[cfg(debug_assertions)]
    rocket: rust_rocket::RocketClient,
    #[cfg(not(debug_assertions))]
    rocket: rust_rocket::RocketPlayer,
}

#[cfg(debug_assertions)]
fn connect() -> rust_rocket::RocketClient {
    loop {
        if let Ok(rocket) = rust_rocket::RocketClient::new() {
            return rocket;
        }
        log::info!("Cannot connect to Rocket, retrying in a sec...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

impl Sync {
    pub fn new(bpm: f64, rows_per_beat: f64) -> Self {
        #[cfg(debug_assertions)]
        let rocket = {
            log::info!("Connecting to rocket tracker");
            connect()
        };
        #[cfg(not(debug_assertions))]
        let rocket = {
            log::info!("Loading {}", TRACKS_FILE);
            let file = std::fs::File::open(TRACKS_FILE).expect("Cannot open track file");
            let tracks = bincode::deserialize_from(file).expect("Failed to deserialize tracks");
            rust_rocket::RocketPlayer::new(tracks)
        };

        Self {
            row: 0.,
            beats_per_sec: bpm / 60.,
            rows_per_beat,
            fft: None,
            rocket,
        }
    }

    #[cfg(debug_assertions)]
    pub fn get(&mut self, track: &str) -> f32 {
        loop {
            if let Ok(track) = self.rocket.get_track_mut(track) {
                return track.get_value(self.row as f32);
            }
            self.reconnect();
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn get(&mut self, track: &str) -> f32 {
        self.rocket
            .get_track(track)
            .unwrap_or_else(|| panic!("Sync track {} is not present. This is a bug, sorry.", track))
            .get_value(self.row as f32)
    }

    pub fn get_fft(&self, freqs: Range<usize>) -> f32 {
        if let Some(fft) = &self.fft {
            fft.average_from_freq_range(freqs)
        } else {
            0.
        }
    }

    pub fn update(&mut self, player: &mut Player) {
        let secs = player.time_secs();
        self.row = self.secs_to_row(secs);
        self.fft = Some(player.fft(secs));

        #[cfg(debug_assertions)]
        {
            use rust_rocket::client::Event;

            loop {
                if let Ok(result) = self.rocket.poll_events() {
                    if let Some(event) = result {
                        match event {
                            Event::SetRow(row) => {
                                player.seek(self.row_to_secs(row as f64));
                            }
                            Event::Pause(state) => {
                                if state {
                                    player.pause().unwrap();
                                } else {
                                    player.play().unwrap();
                                }
                            }
                            Event::SaveTracks => {
                                self.save_tracks();
                            }
                        }
                    } else {
                        break;
                    }
                } else {
                    self.reconnect();
                }
            }

            if player.is_playing() {
                loop {
                    if self.rocket.set_row(self.row as u32).is_ok() {
                        break;
                    }
                    self.reconnect();
                }
            }
        }
    }

    #[cfg(debug_assertions)]
    fn reconnect(&mut self) {
        log::error!("Connection lost, reconnecting");
        self.rocket = connect()
    }

    #[cfg(debug_assertions)]
    fn save_tracks(&mut self) {
        log::info!("Saving {}", TRACKS_FILE);
        let tracks = self.rocket.save_tracks();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(TRACKS_FILE)
            .expect("Cannot open track file");
        bincode::serialize_into(file, &tracks).expect("Cannot serialize tracks");
    }

    #[cfg(debug_assertions)]
    fn row_to_secs(&self, row: f64) -> f64 {
        let beat = row / self.rows_per_beat;
        beat / self.beats_per_sec
    }

    fn secs_to_row(&self, secs: f64) -> f64 {
        secs * self.beats_per_sec * self.rows_per_beat
    }
}

#[cfg(debug_assertions)]
impl Drop for Sync {
    fn drop(&mut self) {
        self.save_tracks();
    }
}
