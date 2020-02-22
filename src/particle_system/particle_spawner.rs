use rand::prelude::*;
use rand_xorshift::XorShiftRng;

pub enum ParticleSpawnerKind {
    Point([f32; 3]),
    Box([f32; 3], [f32; 3]),
}

pub enum ParticleSpawnerMethod {
    Once(usize),
    Rate(f32, f32),
}

pub struct ParticleSpawner {
    kind: ParticleSpawnerKind,
    method: ParticleSpawnerMethod,
    rng: XorShiftRng,
    spawned: usize,
    remainder: f32,
}

impl ParticleSpawner {
    pub fn new(kind: ParticleSpawnerKind, method: ParticleSpawnerMethod) -> Self {
        ParticleSpawner {
            kind,
            method,
            rng: XorShiftRng::seed_from_u64(609),
            spawned: 0,
            remainder: 0.,
        }
    }

    pub fn count_hint(&self, frames: usize) -> usize {
        match self.method {
            ParticleSpawnerMethod::Once(n) => n,
            ParticleSpawnerMethod::Rate(r, dt) => (frames as f32 * r * dt) as usize,
        }
    }

    fn spawn(&mut self, n: usize) -> Vec<f32> {
        match self.kind {
            ParticleSpawnerKind::Point(pos) => {
                let positions: Vec<_> = std::iter::repeat(pos).take(n).collect();
                positions.iter().flatten().map(|f| *f).collect()
            }
            ParticleSpawnerKind::Box(pos1, pos2) => {
                let positions: Vec<_> = (0..n)
                    .map(|_| {
                        [
                            self.rng.gen_range(pos1[0], pos2[0]),
                            self.rng.gen_range(pos1[1], pos2[1]),
                            self.rng.gen_range(pos1[2], pos2[2]),
                        ]
                    })
                    .collect();
                positions.iter().flatten().map(|f| *f).collect()
            }
        }
    }
}

impl std::iter::Iterator for ParticleSpawner {
    type Item = Vec<f32>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.method {
            ParticleSpawnerMethod::Once(n) => {
                if self.spawned != 0 {
                    None
                } else {
                    Some(self.spawn(n))
                }
            }
            ParticleSpawnerMethod::Rate(r, dt) => {
                let count = r * dt + self.remainder;
                self.remainder = count.fract();
                Some(self.spawn(count as usize))
            }
        };

        if let Some(v) = &result {
            self.spawned += v.len();
        }

        result
    }
}
