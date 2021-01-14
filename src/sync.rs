use crate::Player;

const TRACK_FILE: &str = "tracks.bin";

pub struct Sync {
    time_secs: f32,
    #[cfg(debug_assertions)]
    rocket: rust_rocket::Client,
    #[cfg(not(debug_assertions))]
    rocket: rust_rocket::Player,
}

impl Sync {
    #[cfg(debug_assertions)]
    pub fn new() -> Self {
        let rocket = {
            loop {
                if let Ok(rocket) = rust_rocket::Client::new() {
                    break rocket;
                }
                println!("Cannot connect to Rocket, retrying in a sec...");
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
        };

        Self {
            time_secs: 0.,
            rocket,
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn new() -> Self {
        let rocket = rust_rocket::Player::new(TRACK_FILE).unwrap();

        Self {
            time_secs: 0.,
            rocket,
        }
    }

    #[cfg(debug_assertions)]
    pub fn get(&mut self, track: &str) -> f32 {
        self.rocket
            .get_track_mut(track)
            .unwrap()
            .get_value(self.time_secs)
    }

    #[cfg(not(debug_assertions))]
    pub fn get(&mut self, track: &str) -> f32 {
        self.rocket
            .get_track(track)
            .unwrap()
            .get_value(self.time_secs)
    }

    pub fn update(&mut self, player: &mut Player) {
        self.time_secs = player.time_secs() as f32;

        #[cfg(debug_assertions)]
        {
            use rust_rocket::client::Event;

            while let Some(event) = self.rocket.poll_events().unwrap() {
                match event {
                    Event::SetRow(row) => {
                        println!("Seeking unimpl. Row: {}", row);
                    }
                    Event::Pause(state) => {
                        if state {
                            player.pause().unwrap();
                        } else {
                            player.play().unwrap();
                        }
                    }
                    Event::SaveTracks => {
                        self.rocket.save_tracks(TRACK_FILE).unwrap();
                    }
                }
            }
            self.rocket.set_row(self.time_secs as u32).unwrap();
        }
    }
}
