mod particle_spawner;

#[cfg(feature = "discrete-gpu")]
use crate::glesv2_raii::Buffer;
use crate::Scene;
use cgmath::{Vector1, Vector3, VectorSpace};
use opengles::glesv2::{self, constants::*, types::*};
pub use particle_spawner::*;
use std::thread;

pub struct ParticleSystem {
    position_frames: Vec<Vec<Vec<Vector3<f32>>>>, // group(frame(coords, n = particles))
    density_frames: Vec<Vec<Vec<Vector1<f32>>>>,  // group(frame(density, n = x*y*z of voxels))
    density_voxel_count: (GLint, GLint, GLint),
    density_voxel_scale: (f32, f32, f32),
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
        density_voxel_count: (usize, usize, usize),
        density_voxel_scale: (f32, f32, f32),
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
                let mut density_frames = Vec::with_capacity(steps);

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

                    // Compute density voxels
                    let mut densities = Vec::with_capacity(
                        density_voxel_count.0 * density_voxel_count.1 * density_voxel_count.2,
                    );
                    let scale = Vector3::new(
                        density_voxel_scale.0,
                        density_voxel_scale.1,
                        density_voxel_scale.2,
                    );
                    for z in 0..density_voxel_count.2 {
                        for y in 0..density_voxel_count.1 {
                            for x in 0..density_voxel_count.0 {
                                let pos1 = Vector3::new(
                                    (x as f32 - density_voxel_count.0 as f32 / 2.) * scale.x,
                                    (y as f32 - density_voxel_count.1 as f32 / 2.) * scale.y,
                                    (z as f32 - density_voxel_count.2 as f32 / 2.) * scale.z,
                                );
                                let pos2 = pos1 + scale;
                                densities.push(Vector1::new(
                                    positions
                                        .iter()
                                        .filter(|p| {
                                            p.x > pos1.x
                                                && p.x < pos2.x
                                                && p.y > pos1.y
                                                && p.y < pos2.y
                                                && p.z > pos1.z
                                                && p.z < pos2.z
                                        })
                                        .count() as f32
                                        / positions.len() as f32,
                                ));
                            }
                        }
                    }
                    density_frames.push(densities);
                }

                (position_frames, density_frames)
            }));
        }

        let (position_frames, density_frames): (Vec<_>, Vec<_>) =
            threads.into_iter().map(|t| t.join().unwrap()).unzip();

        #[cfg(feature = "discrete-gpu")]
        let buffer = {
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
            buffer.data(&vec![0f32; largest * cpus], GL_DYNAMIC_DRAW);
            buffer
        };

        ParticleSystem {
            position_frames,
            density_frames,
            density_voxel_count: (
                density_voxel_count.0 as GLint,
                density_voxel_count.1 as GLint,
                density_voxel_count.2 as GLint,
            ),
            density_voxel_scale,
            time_step,
            #[cfg(feature = "discrete-gpu")]
            buffer,
        }
    }

    pub fn render(&self, scene: &Scene, time: f32) {
        let program = scene
            .resources
            .program("./particle.vert ./flatshade.frag")
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
                self.buffer.sub_data(0, &interpolated);
                glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, 0, 0);
            }

            #[cfg(not(feature = "discrete-gpu"))]
            glesv2::vertex_attrib_pointer(index_pos, 3, GL_FLOAT, false, 0, &interpolated);

            glesv2::draw_arrays(GL_POINTS, 0, interpolated.len() as GLint);
        }
    }

    pub fn get_densities(&self, time: f32) -> Vec<f32> {
        let mut output = vec![0f32; self.density_frames[0][0].len()];
        let i = (time / self.time_step) as usize;
        for frame_group in &self.density_frames {
            let i = i.min(frame_group.len() - 2); // clamp to frame count
            let interpolated: Vec<_> = frame_group[i]
                .iter()
                .zip(frame_group[i + 1].iter())
                .map(|(i1, i2)| i1.lerp(*i2, (time / self.time_step) - i as f32))
                .collect();
            for (i, v) in interpolated.iter().enumerate() {
                output[i] += v.x / self.density_frames.len() as f32;
            }
        }
        output
    }

    pub fn get_density_voxel_count(&self) -> (GLint, GLint, GLint) {
        self.density_voxel_count
    }

    pub fn get_density_voxel_scale(&self) -> (f32, f32, f32) {
        self.density_voxel_scale
    }
}
