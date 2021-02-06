use std::time::{Duration, Instant};

pub struct FrameCounter {
    frames: u32,
    last_frame: Instant,
    max_frametime: Option<Duration>,
    since: Instant,
    interval: Duration,
}

impl FrameCounter {
    pub fn with_interval(interval: Duration) -> Self {
        let time = Instant::now();
        Self {
            frames: 0,
            last_frame: time,
            max_frametime: None,
            since: time,
            interval,
        }
    }

    pub fn new() -> Self {
        Self::with_interval(Duration::new(1, 0))
    }

    pub fn tick(&mut self) {
        self.frames += 1;

        let frametime = self.last_frame.elapsed();
        self.last_frame = Instant::now();

        if let Some(max_frametime) = &mut self.max_frametime {
            if frametime > *max_frametime {
                *max_frametime = frametime;
            }

            let elapsed = self.since.elapsed();
            if elapsed >= self.interval {
                self.since = Instant::now();

                log::info!(
                    "{} frames in {}s. Frame avg {:.2}ms, max {:.2}ms",
                    self.frames,
                    elapsed.as_secs(),
                    (elapsed.as_secs_f64() / self.frames as f64) * 1000.,
                    max_frametime.as_secs_f64() * 1000.,
                );

                // Reset counting fields
                self.frames = 0;
                self.max_frametime = None;
            }
        } else {
            self.max_frametime = Some(frametime);
        }
    }
}
