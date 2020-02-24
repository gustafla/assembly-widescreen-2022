mod particle_spawner;

use crate::Scene;
use cgmath::Vector3;
use opengles::glesv2::{self, constants::*, types::*};
pub use particle_spawner::*;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ParticleSystem {
    position_frames: Vec<Vec<Vec<f32>>>, // group(frame(coords(f32)))
    timestep: f32,
}

impl ParticleSystem {
    pub fn new(
        spawner: ParticleSpawner,
        frames: usize,
        timestep: f32,
        wind_field: Option<fn(Vector3<f32>, f32) -> Vector3<f32>>, // fn(pos, time) -> force
    ) -> ParticleSystem {
        let cpus = num_cpus::get();
        let mut position_frames = Vec::with_capacity(cpus);
        for _ in 0..cpus {
            position_frames.push(Mutex::new(Vec::with_capacity(frames)));
        }
        let position_frames = Arc::new(position_frames);
        let spawners = Arc::new(spawner.split(cpus));

        let mut threads = Vec::with_capacity(cpus);

        for cpu in 0..cpus {
            let position_frames = position_frames.clone();
            let spawners = spawners.clone();

            threads.push(thread::spawn(move || {
                let count_hint = spawners[cpu].lock().unwrap().count_hint(frames);
                let mut positions = Vec::with_capacity(count_hint * 3);
                let mut velocities = Vec::with_capacity(count_hint * 3);
                let mut masses = Vec::with_capacity(count_hint);

                for frame in 0..frames {
                    // Print progress
                    if cpu == 0 && frame % (frames / 20) == 0 {
                        log::info!("{}%", frame * 100 / frames);
                    }

                    // Spawn particles
                    if let Some(v) = spawners[cpu].lock().unwrap().next() {
                        positions.extend(&v);
                        velocities.extend(vec![0f32; v.len()]);
                        masses.extend(vec![2f32; v.len() / 3]);
                    }

                    // Simulate wind
                    if let Some(wind_field) = wind_field {
                        for i in 0..masses.len() {
                            let force = wind_field(
                                Vector3::new(
                                    positions[i * 3],
                                    positions[i * 3 + 1],
                                    positions[i * 3 + 2],
                                ),
                                timestep * frame as f32,
                            );

                            velocities[i * 3] += force.x / masses[i] * timestep;
                            velocities[i * 3 + 1] += force.y / masses[i] * timestep;
                            velocities[i * 3 + 2] += force.z / masses[i] * timestep;
                        }
                    }

                    // Simulate gravity
                    for i in 0..masses.len() {
                        velocities[i * 3 + 1] -= masses[i] * timestep;
                    }

                    // Simulate drag
                    for v in &mut velocities {
                        *v *= 0.98;
                    }

                    // Integrate position
                    for i in 0..positions.len().min(velocities.len()) / 3 {
                        positions[i * 3] += velocities[i * 3] * timestep;
                        positions[i * 3 + 1] += velocities[i * 3 + 1] * timestep;
                        positions[i * 3 + 2] += velocities[i * 3 + 2] * timestep;
                    }

                    // Store frame state
                    position_frames[cpu].lock().unwrap().push(positions.clone());
                }
            }));
        }

        threads.into_iter().for_each(|t| t.join().unwrap());

        let position_frames: Vec<_> = Arc::try_unwrap(position_frames)
            .unwrap()
            .into_iter()
            .map(|m| m.into_inner().unwrap())
            .collect();

        ParticleSystem {
            position_frames,
            timestep,
        }
    }

    pub fn render(&self, scene: &Scene, time: f32) {
        let program = scene
            .resources
            .program("./particle.vert ./particle.frag")
            .unwrap();

        glesv2::use_program(program.handle());

        glesv2::uniform_matrix4fv(
            program.uniform_location("u_Projection").unwrap(),
            false,
            &scene.projection,
        );
        glesv2::uniform_matrix4fv(
            program.uniform_location("u_View").unwrap(),
            false,
            &scene.view,
        );

        glesv2::bind_buffer(GL_ARRAY_BUFFER, 0);
        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        glesv2::enable_vertex_attrib_array(index_pos);

        let i = (time / self.timestep) as usize;
        for frame_group in &self.position_frames {
            let i = i.min(frame_group.len() - 1); // clamp to frame count
            glesv2::vertex_attrib_pointer(index_pos, 3, GL_FLOAT, false, 0, &frame_group[i]);
            glesv2::draw_arrays(GL_POINTS, 0, frame_group[i].len() as GLint / 3);
        }
    }
}
