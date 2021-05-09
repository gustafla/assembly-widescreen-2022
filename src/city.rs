use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync, Model};
use std::convert::TryFrom;

pub fn generate_terrain(
    xsize: GLushort,
    zsize: GLushort,
    height_map: fn(f32, f32) -> f32,
) -> Model {
    let vertex_buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
    let mut geometry = Vec::with_capacity(xsize as usize * zsize as usize);

    for x in 0i32..xsize as i32 {
        for z in 0i32..zsize as i32 {
            let x = (x - xsize as i32 / 2) as f32;
            let z = -(z - zsize as i32 / 2) as f32;
            geometry.push(glam::vec3(x, height_map(x, z), z)); // Position

            let pos1 = glam::vec3(x, height_map(x, z - 1.), z - 1.);
            let pos2 = glam::vec3(x - 1., height_map(x - 1., z), z);
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

fn generate_building(size: glam::Vec3) -> Model {
    // Hard coded cube coordinates
    let coords = &[
        -1f32, -1., 1., 1., -1., 1., 1., 1., 1., -1., -1., 1., 1., 1., 1., -1., 1., 1., 1., -1.,
        1., 1., -1., -1., 1., 1., -1., 1., -1., 1., 1., 1., -1., 1., 1., 1., 1., -1., -1., -1.,
        -1., -1., -1., 1., -1., 1., -1., -1., -1., 1., -1., 1., 1., -1., -1., -1., -1., -1., -1.,
        1., -1., 1., 1., -1., -1., -1., -1., 1., 1., -1., 1., -1., -1., -1., -1., 1., -1., -1., 1.,
        -1., 1., -1., -1., -1., 1., -1., 1., -1., -1., 1., -1., 1., 1., 1., 1., 1., 1., 1., -1.,
        -1., 1., 1., 1., 1., -1., -1., 1., -1.,
    ];

    // Hard coded cube normals
    let normals = &[
        0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.,
        0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1.,
        0., 0., -1., 0., 0., -1., 0., 0., -1., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0.,
        -1., 0., 0., -1., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0.,
        0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0.,
    ];

    let transformation =
            // Move the foundations underground
            glam::Mat4::from_translation(glam::vec3(0., -5., 0.)) *
            // Scale in width, height and depth
            glam::Mat4::from_scale(size) *
            // Raise the cube to sit on origin
            glam::Mat4::from_translation(glam::vec3(0., 1., 0.));

    // Transform into an interleaved array (quite unoptimized)
    let transformation_normal = glam::Mat3::from(transformation).inverse().transpose();
    let mesh: Vec<glam::Vec3> = coords
        .chunks(3)
        .zip(normals.chunks(3))
        .flat_map(|(coord, normal)| {
            let coord: glam::Vec4 = (coord[0], coord[1], coord[2], 1.).into();
            let normal: glam::Vec3 = (normal[0], normal[1], normal[2]).into();
            let coord = transformation * coord;
            let normal = transformation_normal * normal;
            vec![coord.into(), normal]
        })
        .collect();

    let vertex_buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
    vertex_buffer.bind();
    #[rustfmt::skip]
        vertex_buffer.data(
            &mesh,
            glesv2::STATIC_DRAW,
        );

    Model {
        mode: glesv2::TRIANGLES,
        vertex_buffer,
        index_buffer: None,
        num_elements: GLint::try_from(mesh.len() / 2).unwrap(),
    }
}

pub struct City {
    terrain: Model,
    buildings: Vec<Model>,
    layout: Vec<(usize, glam::Mat4)>,
}

impl City {
    pub fn new(mut rng: impl rand::Rng, num_buildings: usize) -> Self {
        let mut buildings = Vec::with_capacity(num_buildings);

        for _ in 0..num_buildings {
            buildings.push(generate_building(glam::vec3(
                1.,
                5. + rng.gen::<f32>() * 4.,
                1.,
            )));
        }

        let range = 100.;
        let clearance = 8.;
        let mut layout = Vec::new();
        let mut x = -range;
        while x < range {
            let mut z = -range;
            while z < range {
                layout.push((
                    rng.gen_range(0..20),
                    glam::Mat4::from_translation(glam::vec3(x, 0., z)),
                ));
                z += clearance;
            }
            x += clearance;
        }

        Self {
            terrain: generate_terrain(200, 200, |x, z| 0.),
            buildings,
            layout,
        }
    }

    pub fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        for layout in &self.layout {
            self.buildings[layout.0].draw(demo, layout.1);
        }
        self.terrain.draw(demo, glam::Mat4::IDENTITY);
    }
}
