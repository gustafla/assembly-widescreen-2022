mod frame_counter;

use crate::Player;
use frame_counter::FrameCounter;

const TRACKS_FILE: &str = "tracks.bin";

pub struct DemoSync {
    row: f32,
    beats_per_sec: f32,
    rows_per_beat: f32,
    beat: f32,
    frame_counter: Option<FrameCounter>,
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

impl DemoSync {
    pub fn new(bpm: f32, rows_per_beat: f32, benchmark: bool) -> Self {
        #[cfg(debug_assertions)]
        let rocket = {
            log::info!("Connecting to rocket tracker");
            connect()
        };
        #[cfg(not(debug_assertions))]
        let rocket = {
            log::info!("Loading {}", TRACKS_FILE);
            let file = crate::RESOURCES_DIR
                .get_file(TRACKS_FILE)
                .expect("File not present in binary. This is a bug.");
            let tracks =
                bincode::deserialize_from(file.contents()).expect("Failed to deserialize tracks");
            rust_rocket::RocketPlayer::new(tracks)
        };

        Self {
            row: 0.,
            beats_per_sec: bpm / 60.,
            rows_per_beat,
            beat: 0.,
            frame_counter: benchmark.then(FrameCounter::new),
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

    pub fn get_beat(&self) -> f32 {
        self.beat
    }

    #[cfg(debug_assertions)]
    fn poll_events(&mut self, player: &mut Player) -> bool {
        use rust_rocket::client::Event;

        // Keep track whether this frame had seek events
        // to avoid changing the tracker's position when the user is changing it manually
        let mut seeking = false;

        loop {
            if let Ok(result) = self.rocket.poll_events() {
                if let Some(event) = result {
                    match event {
                        Event::SetRow(row) => {
                            player.seek(self.row_to_secs(row as f32));
                            seeking = true;
                        }
                        Event::Pause(state) => {
                            if state {
                                player.pause();
                            } else {
                                player.play();
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

        seeking
    }

    /// Call once per frame
    ///
    /// # Return value
    ///
    /// Returns true if the demo should should end
    pub fn update(&mut self, player: &mut Player) -> bool {
        if let Some(frame_counter) = &mut self.frame_counter {
            frame_counter.tick();
        }

        // Poll rocket events
        #[cfg(debug_assertions)]
        let seeking = self.poll_events(player);

        // This frame's time to render at
        let secs = player.time_secs();

        // In Release builds, signal exit when the demo has played to the end of music
        #[cfg(not(debug_assertions))]
        if secs >= player.len_secs() {
            return true;
        }

        // Set frame's row for Rocket track gets
        self.row = self.secs_to_row(secs);

        // Update rocket tracker's position when necessary
        #[cfg(debug_assertions)]
        if player.is_playing() && !seeking {
            loop {
                if self.rocket.set_row(self.row as u32).is_ok() {
                    break;
                }
                self.reconnect();
            }
        }

        // Absolute energy in low freq range is a pretty good musical beat value
        self.beat = player.bass_psd(secs);

        false
    }

    #[cfg(debug_assertions)]
    fn reconnect(&mut self) {
        log::error!("Connection lost, reconnecting");
        self.rocket = connect()
    }

    #[cfg(debug_assertions)]
    fn save_tracks(&mut self) {
        log::info!("Saving {}/{}", crate::RESOURCES_PATH, TRACKS_FILE);
        let tracks = self.rocket.save_tracks();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(
                [crate::RESOURCES_PATH, TRACKS_FILE]
                    .into_iter()
                    .collect::<std::path::PathBuf>(),
            )
            .expect("Cannot open track file");
        bincode::serialize_into(file, &tracks).expect("Cannot serialize tracks");
    }

    #[cfg(debug_assertions)]
    fn row_to_secs(&self, row: f32) -> f32 {
        let beat = row / self.rows_per_beat;
        beat / self.beats_per_sec
    }

    fn secs_to_row(&self, secs: f32) -> f32 {
        secs * self.beats_per_sec * self.rows_per_beat
    }
}

#[cfg(debug_assertions)]
impl Drop for DemoSync {
    fn drop(&mut self) {
        self.save_tracks();
    }
}
