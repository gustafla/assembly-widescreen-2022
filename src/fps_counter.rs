use std::time::{Duration, Instant};

pub struct FpsCounter {
    frames: u32,
    since: Option<Instant>,
    interval: Duration,
}

impl FpsCounter {
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            frames: 0,
            since: None,
            interval,
        }
    }

    pub fn new() -> Self {
        Self::with_interval(Duration::new(1, 0))
    }

    pub fn tick(&mut self) -> Option<u32> {
        self.frames += 1;
        if let Some(since) = self.since {
            let elapsed = since.elapsed();
            if elapsed >= self.interval {
                let fps = self.frames * 1_000 / elapsed.as_millis() as u32;
                self.frames = 0;
                self.since = Some(Instant::now());
                return Some(fps);
            }
        } else {
            self.since = Some(Instant::now());
        }
        None
    }
}
