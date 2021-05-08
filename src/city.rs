use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync};
use std::convert::TryFrom;

struct Building {
    buffer: glesv2::Buffer,
    count: GLint,
}

impl Building {
    fn new(size: glam::Vec3) -> Self {
        // Hard coded cube coordinates
        let coords = &[
            -1f32, -1., 1., 1., -1., 1., 1., 1., 1., -1., -1., 1., 1., 1., 1., -1., 1., 1., 1.,
            -1., 1., 1., -1., -1., 1., 1., -1., 1., -1., 1., 1., 1., -1., 1., 1., 1., 1., -1., -1.,
            -1., -1., -1., -1., 1., -1., 1., -1., -1., -1., 1., -1., 1., 1., -1., -1., -1., -1.,
            -1., -1., 1., -1., 1., 1., -1., -1., -1., -1., 1., 1., -1., 1., -1., -1., -1., -1., 1.,
            -1., -1., 1., -1., 1., -1., -1., -1., 1., -1., 1., -1., -1., 1., -1., 1., 1., 1., 1.,
            1., 1., 1., -1., -1., 1., 1., 1., 1., -1., -1., 1., -1.,
        ];

        // Hard coded cube normals
        let normals = &[
            0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1.,
            0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 0., -1., 0., 0., -1., 0.,
            0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1., -1., 0., 0., -1., 0., 0., -1., 0., 0.,
            -1., 0., 0., -1., 0., 0., -1., 0., 0., 0., -1., 0., 0., -1., 0., 0., -1., 0., 0., -1.,
            0., 0., -1., 0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1., 0., 0., 1.,
            0., 0., 1., 0.,
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

        let buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
        buffer.bind();
        #[rustfmt::skip]
        buffer.data(
            &mesh,
            glesv2::STATIC_DRAW,
        );

        Self {
            buffer,
            count: GLint::try_from(mesh.len() / 2).unwrap(),
        }
    }

    fn render(&self, demo: &Demo, sync: &mut DemoSync, model: glam::Mat4) {
        let program = demo
            .resources
            .program("gouraud.vert flatshade.frag")
            .unwrap();

        let model_normal = glam::Mat3::from(model).inverse().transpose();

        program.bind(Some(&[
            (
                program.uniform_location("u_Projection").unwrap(),
                glesv2::UniformValue::Matrix4fv(1, demo.projection().as_ref().as_ptr()),
            ),
            (
                program.uniform_location("u_View").unwrap(),
                glesv2::UniformValue::Matrix4fv(1, demo.view().as_ref().as_ptr()),
            ),
            (
                program.uniform_location("u_Model").unwrap(),
                glesv2::UniformValue::Matrix4fv(1, model.as_ref().as_ptr()),
            ),
            (
                program.uniform_location("u_ModelNormal").unwrap(),
                glesv2::UniformValue::Matrix3fv(1, model_normal.as_ref().as_ptr()),
            ),
        ]));

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_normal = program.attrib_location("a_Normal").unwrap() as GLuint;

        self.buffer.bind();

        let float_size = std::mem::size_of::<GLfloat>();
        let stride = float_size as GLsizei * 6;

        unsafe {
            glesv2::EnableVertexAttribArray(index_pos);
            glesv2::EnableVertexAttribArray(index_normal);

            glesv2::VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                std::ptr::null::<GLvoid>(),
            );
            glesv2::VertexAttribPointer(
                index_normal,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (float_size * 3) as *const GLvoid,
            );

            glesv2::DrawArrays(glesv2::TRIANGLES, 0, self.count);
        }
    }
}

pub struct City {
    buildings: Vec<Building>,
    layout: Vec<(usize, glam::Mat4)>,
}

impl City {
    pub fn new(mut rng: impl rand::Rng, num_buildings: usize) -> Self {
        let mut buildings = Vec::with_capacity(num_buildings);

        for _ in 0..num_buildings {
            buildings.push(Building::new(glam::vec3(
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
            self.buildings[layout.0].render(demo, sync, layout.1);
        }
    }
}
