use crate::glesv2::{self, types::*};
use crate::{Demo, DemoSync};

struct Building {
    buffer: glesv2::Buffer,
    count: GLint,
}

impl Building {
    fn new() -> Self {
        let buffer = glesv2::Buffer::new(glesv2::ARRAY_BUFFER);
        buffer.bind();
        #[rustfmt::skip]
        buffer.data(
            &[
                // Front
                -1f32, -1., 1., 0., 0., 1.,
                1., -1., 1., 0., 0., 1.,
                1., 1., 1., 0., 0., 1.,

                -1., -1., 1., 0., 0., 1.,
                1., 1., 1., 0., 0., 1.,
                -1., 1., 1., 0., 0., 1.,

                // Right
                1., -1., 1., 1., 0., 0.,
                1., -1., -1., 1., 0., 0.,
                1., 1., -1., 1., 0., 0.,

                1., -1., 1., 1., 0., 0.,
                1., 1., -1., 1., 0., 0.,
                1., 1., 1., 1., 0., 0.,

                // Back
                1., -1., -1., 0., 0., -1.,
                -1., -1., -1., 0., 0., -1.,
                -1., 1., -1., 0., 0., -1.,

                1., -1., -1., 0., 0., -1.,
                -1., 1., -1., 0., 0., -1.,
                1., 1., -1., 0., 0., -1.,

                // Left
                -1., -1., -1., -1., 0., 0.,
                -1., -1., 1., -1., 0., 0.,
                -1., 1., 1., -1., 0., 0.,

                -1., -1., -1., -1., 0., 0.,
                -1., 1., 1., -1., 0., 0.,
                -1., 1., -1., -1., 0., 0.,

                // Bottom
                -1., -1., -1., 0., -1., 0.,
                1., -1., -1., 0., -1., 0.,
                1., -1., 1., 0., -1., 0.,

                -1., -1., -1., 0., -1., 0.,
                1., -1., 1., 0., -1., 0.,
                -1., -1., 1., 0., -1., 0.,

                // Top
                -1., 1., 1., 0., 1., 0.,
                1., 1., 1., 0., 1., 0.,
                1., 1., -1., 0., 1., 0.,

                -1., 1., 1., 0., 1., 0.,
                1., 1., -1., 0., 1., 0.,
                -1., 1., -1., 0., 1., 0.,
            ],
            glesv2::STATIC_DRAW,
        );

        Self { buffer, count: 36 }
    }

    fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        let program = demo
            .resources
            .program("gouraud.vert flatshade.frag")
            .unwrap();

        let time = sync.get("sim_time");
        let model = glam::Mat4::from_rotation_ypr(time, time * 0.7, -time * 0.44);
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
}

impl City {
    pub fn new() -> Self {
        Self {
            buildings: vec![Building::new()],
        }
    }

    pub fn render(&self, demo: &Demo, sync: &mut DemoSync) {
        for building in &self.buildings {
            building.render(demo, sync);
        }
    }
}
