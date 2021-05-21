mod particle_spawner;

use crate::{
    glesv2::{self, types::*},
    Demo,
};
use glam::Vec3;
pub use particle_spawner::*;
use std::thread;

pub struct ParticleSystem {
    position_frames: Vec<Vec<Vec<Vec3>>>, // group(frame(coords, n = particles))
    interpolated: Vec<Vec3>,
    time_step: f32,
}

impl ParticleSystem {
    pub fn new(
        spawner: ParticleSpawner,
        duration: f32,
        steps: usize,
        force_field: fn(Vec3, f32) -> Vec3, // fn(pos, time) -> force
    ) -> ParticleSystem {
        let time_step = duration / steps as f32;

        let cpus = num_cpus::get();
        let mut threads = Vec::with_capacity(cpus);

        for (cpu, mut spawner) in spawner.split(cpus).into_iter().enumerate() {
            threads.push(thread::spawn(move || {
                let count_hint = spawner.count_hint(steps);
                let mut positions = Vec::with_capacity(count_hint);
                let mut velocities = Vec::with_capacity(count_hint);
                let mut masses = Vec::with_capacity(count_hint);
                let mut position_frames = Vec::with_capacity(steps);

                for step in 0..steps {
                    // Print progress
                    if cpu == 0 && step % (steps / 20) == 0 {
                        log::info!("{}%", step * 100 / steps);
                    }

                    // Spawn particles
                    if let Some(v) = spawner.next() {
                        positions.extend(&v);
                        velocities.extend(vec![Vec3::new(0., 0., 0.); v.len()]);
                        masses.extend(vec![1f32; v.len()]);
                    }

                    // Simulate wind and gravity etc
                    for i in 0..positions.len().min(masses.len()) {
                        let force = force_field(positions[i], time_step * step as f32);
                        velocities[i] += force / masses[i] * time_step;
                    }

                    // Simulate drag TODO?
                    /*for v in &mut velocities {
                        *v *= 0.98;
                    }*/

                    // Integrate position
                    for i in 0..positions.len().min(velocities.len()) {
                        positions[i] += velocities[i] * time_step;
                    }

                    // Store frame state
                    position_frames.push(positions.clone());

                    // Compute density voxels
                }

                position_frames
            }));
        }

        let position_frames: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();

        // Find maximum count of particles
        let largest = position_frames
            .iter()
            .flatten()
            .map(|v| v.len())
            .max()
            .unwrap();

        ParticleSystem {
            position_frames,
            interpolated: Vec::with_capacity(largest * cpus),
            time_step,
        }
    }

    pub fn prepare(&mut self, time: f32, cam_pos: Vec3) {
        // Clear last results
        self.interpolated.clear();

        let i = (time / self.time_step) as usize;
        for frame_group in &self.position_frames {
            let i = i.min(frame_group.len() - 2); // clamp to frame count
            let interpolated: Vec<_> = frame_group[i]
                .iter()
                .zip(frame_group[i + 1].iter())
                .map(|(p1, p2)| p1.lerp(*p2, (time / self.time_step) - i as f32))
                .collect();

            self.interpolated.extend(interpolated);
        }

        if glesv2::get_booleanv(glesv2::DEPTH_TEST) && glesv2::get_booleanv(glesv2::BLEND) {
            // Sort particles because of alpha blending + depth testing = difficult
            self.interpolated.sort_unstable_by(|a, b| {
                // sort by distance squared
                {
                    let btoc = cam_pos - *b;
                    &(btoc.x * btoc.x + btoc.y * btoc.y)
                }
                .partial_cmp({
                    let atoc = cam_pos - *a;
                    &(atoc.x * atoc.x + atoc.y * atoc.y)
                })
                .unwrap()
            });
        }
    }

    pub fn render(&self, demo: &Demo, projection: glam::Mat4, view: glam::Mat4) {
        let program = demo
            .resources
            .program("particle.vert flatshade.frag")
            .unwrap();

        program.bind(Some(&[
            (
                program.uniform_location("u_Resolution").unwrap(),
                glesv2::UniformValue::Vec2f(
                    demo.resolution().width as f32,
                    demo.resolution().height as f32,
                ),
            ),
            (
                program.uniform_location("u_Projection").unwrap(),
                glesv2::UniformValue::Matrix4fv(1, projection.as_ref().as_ptr()),
            ),
            (
                program.uniform_location("u_View").unwrap(),
                glesv2::UniformValue::Matrix4fv(1, view.as_ref().as_ptr()),
            ),
        ]));

        unsafe {
            glesv2::BindBuffer(glesv2::ARRAY_BUFFER, 0);

            let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
            glesv2::EnableVertexAttribArray(index_pos);
            glesv2::VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                0,
                self.interpolated.as_ptr() as *const GLvoid,
            );
            glesv2::DrawArrays(glesv2::POINTS, 0, self.interpolated.len() as GLint);
        }
    }
}
