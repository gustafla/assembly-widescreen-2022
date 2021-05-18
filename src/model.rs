use crate::glesv2::{self, types::*};
use crate::Demo;
use crate::DemoSync;

pub struct Model {
    pub mode: GLenum,
    pub vertex_buffer: glesv2::Buffer,
    pub index_buffer: Option<glesv2::Buffer>,
    pub num_elements: GLint,
}

impl Model {
    pub fn draw(&self, demo: &Demo, sync: &mut DemoSync, mat_model: glam::Mat4) {
        let program = demo
            .resources
            .program("gouraud.vert flatshade.frag")
            .unwrap();

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
                glesv2::UniformValue::Matrix4fv(1, mat_model.as_ref().as_ptr()),
            ),
            (
                program.uniform_location("u_ModelNormal").unwrap(),
                glesv2::UniformValue::Matrix3fv(
                    1,
                    glam::Mat3::from(mat_model)
                        .inverse()
                        .transpose()
                        .as_ref()
                        .as_ptr(),
                ),
            ),
            (
                program.uniform_location("u_SunDir").unwrap(),
                glesv2::UniformValue::Vec3f(
                    sync.get("light:sundir.x"),
                    sync.get("light:sundir.y"),
                    sync.get("light:sundir.z"),
                ),
            ),
            (
                program.uniform_location("u_AmbientLevel").unwrap(),
                glesv2::UniformValue::Float(sync.get("light:ambient")),
            ),
            (
                program.uniform_location("u_SunLevel").unwrap(),
                glesv2::UniformValue::Float(sync.get("light:sun")),
            ),
        ]));

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_normal = program.attrib_location("a_Normal").unwrap() as GLuint;

        self.vertex_buffer.bind();
        if let Some(index_buffer) = &self.index_buffer {
            index_buffer.bind();
        }

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

            if let Some(_) = self.index_buffer {
                glesv2::DrawElements(
                    self.mode,
                    self.num_elements,
                    glesv2::UNSIGNED_SHORT,
                    std::ptr::null::<GLvoid>(),
                );
            } else {
                glesv2::DrawArrays(self.mode, 0, self.num_elements);
            }
        }
    }
}
