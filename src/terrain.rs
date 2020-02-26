use crate::glesv2_raii::Buffer;
use crate::Scene;
use cgmath::Vector3;
use opengles::glesv2::{self, constants::*, types::*};

pub struct Terrain {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    count: GLint,
}

impl Terrain {
    pub fn new(xsize: GLushort, zsize: GLushort, height_map: fn(f32, f32) -> f32) -> Self {
        let vertex_buffer = Buffer::new(GL_ARRAY_BUFFER);
        let mut geometry = Vec::with_capacity(xsize as usize * zsize as usize);

        for x in 0i32..xsize as i32 {
            for z in 0i32..zsize as i32 {
                let x = (x - xsize as i32 / 2) as f32;
                let z = -(z - zsize as i32 / 2) as f32;
                geometry.push(Vector3::new(x, height_map(x, z), z));
            }
        }

        vertex_buffer.bind();
        vertex_buffer.data(&geometry, GL_STATIC_DRAW);

        let index_buffer = Buffer::new(GL_ELEMENT_ARRAY_BUFFER);
        let mut indices: Vec<GLushort> = Vec::new();

        for i in 0..xsize - 1 {
            for j in 0..zsize {
                indices.push(i * zsize + j); // A
                indices.push(i * zsize + zsize + j); // B
            }

            // degenerate triangle(s?)
            indices.push(i * zsize + zsize - 1); // C
            indices.push(i * zsize + zsize - 1); // C
            indices.push((i + 1) * zsize); // D
            indices.push((i + 1) * zsize); // D
        }

        index_buffer.bind();
        index_buffer.data(&indices, GL_STATIC_DRAW);

        Self {
            vertex_buffer,
            index_buffer,
            count: indices.len() as GLint,
        }
    }

    pub fn render(&self, scene: &Scene) {
        let program = scene.resources.program("./poly.vert ./flatshade.frag").unwrap();

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

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;

        self.vertex_buffer.bind();
        self.index_buffer.bind();

        glesv2::enable_vertex_attrib_array(index_pos);
        glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, 0, 0);

        glesv2::draw_elements::<GLushort>(GL_TRIANGLE_STRIP, self.count, GL_UNSIGNED_SHORT, &[]);
    }
}
