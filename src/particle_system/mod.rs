mod particle_spawner;

#[cfg(feature = "discrete-gpu")]
use crate::glesv2_raii::Buffer;
use crate::Scene;
use cgmath::{Vector3, VectorSpace};
use opengles::glesv2::{self, constants::*, types::*};
pub use particle_spawner::*;
use std::thread;

pub struct ParticleSystem {
    position_frames: Vec<Vec<Vec<Vector3<f32>>>>, // group(frame(coords))
    time_step: f32,
    #[cfg(feature = "discrete-gpu")]
    buffer: Buffer,
}

impl ParticleSystem {
    pub fn new(
        spawner: ParticleSpawner,
        duration: f32,
        steps: usize,
        force_field: fn(Vector3<f32>, f32) -> Vector3<f32>, // fn(pos, time) -> force
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
                        velocities.extend(vec![Vector3::new(0., 0., 0.); v.len()]);
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
                }

                position_frames
            }));
        }

        let position_frames: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();

        #[cfg(feature = "discrete-gpu")]
        {
            // Find maximum count of particles
            let largest = position_frames
                .iter()
                .flatten()
                .map(|v| v.len())
                .max()
                .unwrap();

            // Allocate OpenGL buffer for maximum count of particles
            let buffer = Buffer::new(GL_ARRAY_BUFFER);
            buffer.bind();
            glesv2::buffer_data(
                GL_ARRAY_BUFFER,
                &vec![0f32; largest * cpus],
                GL_DYNAMIC_DRAW,
            );

            ParticleSystem {
                position_frames,
                time_step,
                buffer,
            }
        }

        #[cfg(not(feature = "discrete-gpu"))]
        {
            ParticleSystem {
                position_frames,
                time_step,
            }
        }
    }

    pub fn render(&self, scene: &Scene, time: f32) {
        let program = scene
            .resources
            .program("./particle.vert ./particle.frag")
            .unwrap();

        glesv2::use_program(program.handle());

        glesv2::uniform2f(
            program.uniform_location("u_Resolution").unwrap(),
            scene.resolution.0 as f32,
            scene.resolution.1 as f32,
        );
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

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        glesv2::enable_vertex_attrib_array(index_pos);

        // Bind the VBO before usage, or clear binding if not using buffers
        #[cfg(feature = "discrete-gpu")]
        self.buffer.bind();
        #[cfg(not(feature = "discrete-gpu"))]
        glesv2::bind_buffer(GL_ARRAY_BUFFER, 0);

        let i = (time / self.time_step) as usize;
        for frame_group in &self.position_frames {
            let i = i.min(frame_group.len() - 2); // clamp to frame count
            let interpolated: Vec<_> = frame_group[i]
                .iter()
                .zip(frame_group[i + 1].iter())
                .map(|(p1, p2)| p1.lerp(*p2, (time / self.time_step) - i as f32))
                .collect();

            #[cfg(feature = "discrete-gpu")]
            {
                // Upload to OpenGL buffer
                glesv2::buffer_sub_data(GL_ARRAY_BUFFER, 0, &interpolated);
                glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, 0, 0);
            }

            #[cfg(not(feature = "discrete-gpu"))]
            glesv2::vertex_attrib_pointer(index_pos, 3, GL_FLOAT, false, 0, &interpolated);

            glesv2::draw_arrays(GL_POINTS, 0, interpolated.len() as GLint);
        }
    }
}
