use crate::Player;

const TRACK_FILE: &str = "resources/tracks.bin";

pub struct Sync {
    row: f32,
    beats_per_sec: f32,
    rows_per_beat: f32,
    #[cfg(debug_assertions)]
    rocket: rust_rocket::Client,
    #[cfg(not(debug_assertions))]
    rocket: rust_rocket::Player,
}

#[cfg(debug_assertions)]
fn connect() -> rust_rocket::Client {
    loop {
        if let Ok(rocket) = rust_rocket::Client::new() {
            return rocket;
        }
        log::info!("Cannot connect to Rocket, retrying in a sec...");
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

impl Sync {
    pub fn new(bpm: f32, rows_per_beat: f32) -> Self {
        #[cfg(debug_assertions)]
        let rocket = connect();
        #[cfg(not(debug_assertions))]
        let rocket = rust_rocket::Player::new(TRACK_FILE)
            .unwrap_or_else(|e| panic!("{}: {}", TRACK_FILE, e));

        Self {
            row: 0.,
            beats_per_sec: bpm / 60.,
            rows_per_beat,
            rocket,
        }
    }

    #[cfg(debug_assertions)]
    pub fn get(&mut self, track: &str) -> f32 {
        loop {
            if let Ok(track) = self.rocket.get_track_mut(track) {
                return track.get_value(self.row);
            }
            self.reconnect();
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn get(&mut self, track: &str) -> f32 {
        self.rocket
            .get_track(track)
            .unwrap_or_else(|| panic!("Sync track {} is not present. This is a bug, sorry.", track))
            .get_value(self.row)
    }

    pub fn update(&mut self, player: &mut Player) {
        self.row = self.secs_to_row(player.time_secs());

        #[cfg(debug_assertions)]
        {
            use rust_rocket::client::Event;

            loop {
                if let Ok(result) = self.rocket.poll_events() {
                    if let Some(event) = result {
                        match event {
                            Event::SetRow(row) => {
                                player.seek(self.row_to_secs(row as f32)).unwrap();
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

            loop {
                if self.rocket.set_row(self.row as u32).is_ok() {
                    break;
                }
                self.reconnect();
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
        self.rocket
            .save_tracks(TRACK_FILE)
            .unwrap_or_else(|e| panic!("{}: {}", TRACK_FILE, e));
        log::info!("Tracks saved to {}", TRACK_FILE);
    }

    fn secs_to_row(&self, secs: f32) -> f32 {
        secs * self.beats_per_sec * self.rows_per_beat
    }

    fn row_to_secs(&self, row: f32) -> f32 {
        let beat = row / self.rows_per_beat;
        beat / self.beats_per_sec
    }
}

#[cfg(debug_assertions)]
impl Drop for Sync {
    fn drop(&mut self) {
        self.save_tracks();
    }
}
