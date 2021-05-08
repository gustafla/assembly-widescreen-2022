use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync, Model};
use std::convert::TryFrom;

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

        Self { buildings, layout }
    }

    pub fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        for layout in &self.layout {
            self.buildings[layout.0].draw(demo, layout.1);
        }
    }
}
