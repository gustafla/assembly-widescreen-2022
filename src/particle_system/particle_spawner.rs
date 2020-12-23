use cgmath::{InnerSpace, Vector3};
use rand::prelude::*;
use rand_xorshift::XorShiftRng;

#[derive(Clone, Copy)]
pub enum ParticleSpawnerKind {
    #[allow(dead_code)]
    Point(f32, f32, f32),
    Box((f32, f32, f32), (f32, f32, f32)),
    #[allow(dead_code)]
    Sphere(f32, f32), // internal radius, external radius
}

pub enum ParticleSpawnerMethod {
    Once(usize),
    #[allow(dead_code)]
    Rate(f32, f32),
}

pub struct ParticleSpawner {
    position: Vector3<f32>,
    kind: ParticleSpawnerKind,
    method: ParticleSpawnerMethod,
    rng: XorShiftRng,
    spawned: usize,
    remainder: f32,
}

impl ParticleSpawner {
    pub fn new(
        position: Vector3<f32>,
        kind: ParticleSpawnerKind,
        method: ParticleSpawnerMethod,
    ) -> Self {
        ParticleSpawner {
            position,
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

    fn spawn(&mut self, n: usize) -> Vec<Vector3<f32>> {
        match self.kind {
            ParticleSpawnerKind::Point(x, y, z) => vec![Vector3::new(x, y, z) + self.position; n],
            ParticleSpawnerKind::Box(pos1, pos2) => (0..n)
                .map(|_| {
                    Vector3::new(
                        // TODO rand::distributions::uniform::Uniform
                        self.rng.gen_range(pos1.0..pos2.0),
                        self.rng.gen_range(pos1.1..pos2.1),
                        self.rng.gen_range(pos1.2..pos2.2),
                    ) + self.position
                })
                .collect(),
            ParticleSpawnerKind::Sphere(int_r, ext_r) => (0..n)
                .map(|_| {
                    Vector3::new(
                        self.rng.gen_range(-1. ..1.),
                        self.rng.gen_range(-1. ..1.),
                        self.rng.gen_range(-1. ..1.),
                    )
                    .normalize()
                        * self.rng.gen_range(int_r..ext_r)
                        + self.position
                })
                .collect(),
        }
    }

    pub fn split(mut self, n: usize) -> Vec<Self> {
        (0..n)
            .map(|_| {
                use ParticleSpawnerMethod::*;
                ParticleSpawner {
                    position: self.position,
                    kind: self.kind,
                    method: match self.method {
                        Once(count) => Once(count / n),
                        Rate(r, dt) => Rate(r / n as f32, dt),
                    },
                    rng: XorShiftRng::seed_from_u64(self.rng.gen()),
                    spawned: self.spawned / n,
                    remainder: self.remainder / n as f32,
                }
            })
            .collect()
    }
}

impl std::iter::Iterator for ParticleSpawner {
    type Item = Vec<Vector3<f32>>;

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
