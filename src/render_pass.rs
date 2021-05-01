use crate::glesv2::{self, types::*};
use crate::shader_quad::ShaderQuad;
use crate::Demo;
use crate::Resolution;
use std::convert::TryFrom;

pub struct RenderPass {
    fbo: glesv2::Framebuffer,
    shader_quad: ShaderQuad,
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
            shader_quad: ShaderQuad::new(resolution, frag_path),
        }
    }

    pub fn fbo(&self) -> &glesv2::Framebuffer {
        &self.fbo
    }

    pub fn render(
        &self,
        demo: &Demo,
        textures: &[&glesv2::Texture],
        uniforms: &[(&str, glesv2::UniformValue)],
        to_resolution: Option<Resolution>,
    ) {
        let mut textures_with_fbo = vec![self.fbo.texture(glesv2::COLOR_ATTACHMENT0).unwrap()];
        textures_with_fbo.extend_from_slice(&textures);
        self.shader_quad
            .render(demo, &textures_with_fbo, uniforms, to_resolution);
    }
}
