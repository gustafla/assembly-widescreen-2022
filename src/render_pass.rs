use crate::glesv2::{self, types::*};
use crate::Demo;
use crate::Resolution;
use std::convert::TryFrom;
pub struct RenderPass {
    pub fbo: glesv2::Framebuffer,
    shader_path: String,
    resolution: (f32, f32),
}

impl RenderPass {
    pub fn new(
        resolution: Resolution,
        frag_path: &str,
        renderbuffers: Option<Vec<(GLenum, glesv2::RenderbufferAttachment)>>,
    ) -> Self {
        let fbo_texture = glesv2::Texture::new(glesv2::TEXTURE_2D);
        fbo_texture.image::<u8>(
            0,
            glesv2::RGB,
            i32::try_from(resolution.width).unwrap(),
            i32::try_from(resolution.height).unwrap(),
            glesv2::UNSIGNED_BYTE,
            None,
        );
        fbo_texture.parameters(&[
            (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_WRAP_S, glesv2::CLAMP_TO_EDGE),
            (glesv2::TEXTURE_WRAP_T, glesv2::CLAMP_TO_EDGE),
        ]);

        let fbo = glesv2::Framebuffer::new(
            Some(vec![(
                glesv2::COLOR_ATTACHMENT0,
                glesv2::TextureAttachment {
                    target: glesv2::TEXTURE_2D,
                    texture: fbo_texture,
                    mipmap_level: 0,
                },
            )]),
            renderbuffers,
        )
        .unwrap();

        Self {
            fbo,
            shader_path: format!("shader.vert {}", frag_path),
            resolution: (resolution.width as f32, resolution.height as f32),
        }
    }

    pub fn render(
        &self,
        demo: &Demo,
        textures: &[&glesv2::Texture],
        uniforms: &[(&str, glesv2::UniformValue)],
        to_resolution: Option<Resolution>,
    ) {
        let program = demo.resources.program(&self.shader_path).unwrap();

        let mut uniforms: Vec<(GLint, glesv2::UniformValue)> = uniforms
            .iter()
            .map(|(loc, val)| (program.uniform_location(loc).unwrap(), *val))
            .collect();

        uniforms.push((
            program.uniform_location("u_InputSampler0").unwrap(),
            glesv2::UniformValue::Int(0),
        ));

        self.fbo.texture(glesv2::COLOR_ATTACHMENT0).unwrap().bind(0);
        for (i, texture) in textures.iter().enumerate() {
            let i = i + 1;
            texture.bind(i as GLuint);
            uniforms.push((
                program
                    .uniform_location(&format!("u_InputSampler{}", i))
                    .unwrap(),
                glesv2::UniformValue::Int(i as GLint),
            ));
        }

        if let Some(loc) = program.uniform_location("u_Resolution") {
            uniforms.push((
                loc,
                glesv2::UniformValue::Vec2f(self.resolution.0, self.resolution.1),
            ));
        }

        program.bind(Some(&uniforms));

        // Generate a quad
        let mut left = -1.;
        let mut right = 1.;
        let mut down = -1.;
        let mut up = 1.;

        // Letterbox the aspect ratio difference
        if let Some(to_resolution) = to_resolution {
            glesv2::viewport(
                0,
                0,
                i32::try_from(to_resolution.width).unwrap(),
                i32::try_from(to_resolution.height).unwrap(),
            );

            let from_w = self.resolution.0;
            let from_h = self.resolution.1;
            let to_w = to_resolution.width as f32;
            let to_h = to_resolution.height as f32;
            let from_aspect_ratio = from_w / from_h;
            let to_aspect_ratio = to_w / to_h;
            let h_scale = from_h / to_h;
            let w_scale = from_w / to_w;

            if from_aspect_ratio < to_aspect_ratio {
                right = w_scale / h_scale;
                left = -right;
            } else {
                up = h_scale / w_scale;
                down = -up;
            };
        }

        #[rustfmt::skip]
        let quad = [
            left, down, 0., 0., 0.,
            right, down, 0., 1., 0.,
            right, up, 0., 1., 1.,
            left, down, 0., 0., 0.,
            right, up, 0., 1., 1.,
            left, up, 0., 0., 1.,
        ];

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_tex_coord = program.attrib_location("a_TexCoord").unwrap() as GLuint;
        let stride = (std::mem::size_of::<f32>() * 5) as GLint;
        unsafe {
            glesv2::BindBuffer(glesv2::ARRAY_BUFFER, 0);
            glesv2::EnableVertexAttribArray(index_pos);
            glesv2::VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                quad.as_ptr() as *const GLvoid,
            );
            glesv2::EnableVertexAttribArray(index_tex_coord);
            glesv2::VertexAttribPointer(
                index_tex_coord,
                2,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (quad.as_ptr().add(3)) as *const GLvoid,
            );

            glesv2::DrawArrays(glesv2::TRIANGLES, 0, 6);
        }
    }
}
