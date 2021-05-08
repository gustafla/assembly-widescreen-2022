use crate::glesv2::{self, types::*};
use crate::Model;
use glam::Vec3;

pub fn generate(xsize: GLushort, zsize: GLushort, height_map: fn(f32, f32) -> f32) -> Model {
    let vertex_buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
    let mut geometry = Vec::with_capacity(xsize as usize * zsize as usize);

    for x in 0i32..xsize as i32 {
        for z in 0i32..zsize as i32 {
            let x = (x - xsize as i32 / 2) as f32;
            let z = -(z - zsize as i32 / 2) as f32;
            geometry.push(Vec3::new(x, height_map(x, z), z)); // Position

            let pos1 = Vec3::new(x, height_map(x, z - 1.), z - 1.);
            let pos2 = Vec3::new(x - 1., height_map(x - 1., z), z);
            let pos3 = geometry.last().unwrap();

            let u = pos2 - pos1;
            let v = *pos3 - pos1;

            geometry.push(u.cross(v).normalize()); // Normal
        }
    }

    vertex_buffer.bind();
    vertex_buffer.data(&geometry, glesv2::STATIC_DRAW);

    let index_buffer = glesv2::Buffer::new(glesv2::ELEMENT_ARRAY_BUFFER);
    let mut indices: Vec<GLushort> = Vec::new();

    for i in 0..xsize - 1 {
        for j in 0..zsize {
            indices.push(i * zsize + j); // A
            indices.push(i * zsize + zsize + j); // B
        }

        // Degenerate triangle(s?)
        indices.push(i * zsize + zsize - 1); // C
        indices.push(i * zsize + zsize - 1); // C
        indices.push((i + 1) * zsize); // D
        indices.push((i + 1) * zsize); // D
    }

    index_buffer.bind();
    index_buffer.data(&indices, glesv2::STATIC_DRAW);

    Model {
        mode: glesv2::TRIANGLE_STRIP,
        vertex_buffer,
        index_buffer: Some(index_buffer),
        num_elements: indices.len() as GLint,
    }
}
