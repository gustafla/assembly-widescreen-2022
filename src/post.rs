use crate::glesv2_raii::{Buffer, Framebuffer, Program, Shader, Texture, TextureAttachment};
use crate::Scene;
use opengles::glesv2::{self, constants::*, types::*};
use std::path::Path;

lazy_static::lazy_static! {
    static ref BUFFER: Buffer = {
        let buffer = Buffer::new();
        glesv2::bind_buffer(GL_ARRAY_BUFFER, buffer.handle());
        glesv2::buffer_data(
            GL_ARRAY_BUFFER,
            &[
                -1f32, -1., 0., 0., 0., 1., -1., 0., 1., 0., 1., 1., 0., 1., 1., -1., -1., 0., 0.,
                0., 1., 1., 0., 1., 1., -1., 1., 0., 0., 1.,
            ],
            GL_STATIC_DRAW,
        );
        buffer
    };
}

pub struct Post {
    program: Program,
    pub fbo: Framebuffer,
}

impl Post {
    pub fn new<P: AsRef<Path>>(w: i32, h: i32, frag_path: P) -> Self {
        let fbo_texture = Texture::new();
        glesv2::bind_texture(GL_TEXTURE_2D, fbo_texture.handle());
        Texture::image::<u8>(GL_TEXTURE_2D, 0, GL_RGB, w, h, GL_UNSIGNED_BYTE, &[]);
        Texture::set_filters(GL_TEXTURE_2D, GL_NEAREST);
        let fbo = Framebuffer::new(
            Some(vec![(
                GL_COLOR_ATTACHMENT0,
                TextureAttachment {
                    target: GL_TEXTURE_2D,
                    texture: fbo_texture,
                    mipmap_level: 0,
                },
            )]),
            None,
        )
        .unwrap();

        Self {
            program: Program::from_shaders(&[
                crate::VERT_SHADER.handle(),
                Shader::from_source(frag_path).unwrap().handle(),
            ])
            .unwrap(),
            fbo,
        }
    }

    pub fn render(&self, scene: &Scene) {
        glesv2::active_texture(GL_TEXTURE0);
        glesv2::bind_texture(
            GL_TEXTURE_2D,
            self.fbo.texture_handle(GL_COLOR_ATTACHMENT0).unwrap(),
        );

        glesv2::bind_buffer(GL_ARRAY_BUFFER, BUFFER.handle());
        let index_pos = self.program.attrib_location("a_Pos");
        let index_tex_coord = self.program.attrib_location("a_TexCoord");
        let stride = (std::mem::size_of::<f32>() * 5) as GLint;
        glesv2::enable_vertex_attrib_array(index_pos);
        glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, stride, 0);
        glesv2::enable_vertex_attrib_array(index_tex_coord);
        glesv2::vertex_attrib_pointer_offset(
            index_tex_coord,
            2,
            GL_FLOAT,
            false,
            stride,
            std::mem::size_of::<f32>() as GLuint * 3,
        );

        glesv2::use_program(self.program.handle());
        glesv2::uniform1f(
            self.program.uniform_location("u_NoiseTime"),
            scene.sync_get("noise_time") as f32,
        );
        glesv2::uniform1f(
            self.program.uniform_location("u_NoiseAmount"),
            scene.sync_get("noise_amount") as f32,
        );
        glesv2::uniform1i(self.program.uniform_location("u_InputSampler"), 0);
        glesv2::uniform2f(
            self.program.uniform_location("u_Resolution"),
            scene.resolution.0 as f32,
            scene.resolution.1 as f32,
        );

        glesv2::draw_arrays(GL_TRIANGLES, 0, 6);
    }
}
