use crate::Scene;
use cgmath::{prelude::*, Matrix4};
use opengles::glesv2::{self, constants::*, types::*};
use rand::prelude::*; // for seed_from_u64, gen

pub struct ParticleSystem {
    positions: Vec<Vec<f32>>,
    timestep: f32,
}

impl ParticleSystem {
    pub fn new<R: RngCore>(
        rng: &mut R,
        particle_count: usize,
        frames: usize,
        timestep: f32,
    ) -> ParticleSystem {
        let mut positions = Vec::with_capacity(particle_count * 3 * frames);
        let mut particles = vec![0f32; particle_count * 3];
        let mut velocities: Vec<_> = particles
            .iter()
            .map(|_| rng.gen::<f32>() * 2. - 1.)
            .collect();

        for _ in 0..frames {
            // Simulate gravity
            for i in 0..particle_count {
                velocities[i * 3 + 1] -= 0.4 * timestep;
            }

            // Simulate drag
            for v in &mut velocities {
                *v *= 0.98;
            }

            // Integrate position
            for i in 0..particle_count {
                particles[i * 3] += velocities[i * 3] * timestep;
                particles[i * 3 + 1] += velocities[i * 3 + 1] * timestep;
                particles[i * 3 + 2] += velocities[i * 3 + 2] * timestep;
            }

            // Store frame state
            positions.push(particles.clone());
        }

        ParticleSystem {
            positions,
            timestep,
        }
    }

    pub fn render(&self, scene: &Scene, time: f32) {
        let i = (time / self.timestep) as usize;
        let i = i.min(self.positions.len() - 1); // clamp to frame count

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
        let id: [f32; 16] = *Matrix4::identity().as_ref();
        glesv2::uniform_matrix4fv(program.uniform_location("u_Model").unwrap(), false, &id);

        glesv2::bind_buffer(GL_ARRAY_BUFFER, 0);
        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;

        glesv2::enable_vertex_attrib_array(index_pos);
        glesv2::vertex_attrib_pointer(index_pos, 3, GL_FLOAT, false, 0, &self.positions[i]);

        glesv2::draw_arrays(GL_POINTS, 0, self.positions[i].len() as GLint / 3);
    }
}
