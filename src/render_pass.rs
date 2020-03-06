use crate::glesv2::{
    self, types::*, Framebuffer, RcGl, RenderbufferAttachment, Texture, TextureAttachment,
    UniformValue,
};
use crate::Scene;

pub struct RenderPass {
    gl: RcGl,
    pub fbo: Framebuffer,
    shader_path: String,
}

impl RenderPass {
    pub fn new(
        gl: RcGl,
        w: i32,
        h: i32,
        frag_path: &str,
        renderbuffers: Option<Vec<(GLenum, RenderbufferAttachment)>>,
    ) -> Self {
        let fbo_texture = Texture::new(gl.clone(), glesv2::TEXTURE_2D);
        fbo_texture.image::<u8>(0, glesv2::RGB, w, h, glesv2::UNSIGNED_BYTE, None);
        fbo_texture.parameters(&[
            (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_WRAP_S, glesv2::CLAMP_TO_EDGE),
            (glesv2::TEXTURE_WRAP_T, glesv2::CLAMP_TO_EDGE),
        ]);

        let fbo = Framebuffer::new(
            gl.clone(),
            Some(vec![(
                glesv2::COLOR_ATTACHMENT0,
                TextureAttachment {
                    target: glesv2::TEXTURE_2D,
                    texture: fbo_texture,
                    mipmap_level: 0,
                },
            )]),
            renderbuffers,
        )
        .unwrap();

        Self {
            gl,
            fbo,
            shader_path: format!("./shader.vert {}", frag_path),
        }
    }

    pub fn render(&self, scene: &Scene, textures: &[GLuint], uniforms: &[(&str, UniformValue)]) {
        let program = scene.resources.program(&self.shader_path).unwrap();

        unsafe {
            self.gl.UseProgram(program.handle());
            self.gl.ActiveTexture(glesv2::TEXTURE0);
            self.gl.BindTexture(
                glesv2::TEXTURE_2D,
                self.fbo.texture_handle(glesv2::COLOR_ATTACHMENT0).unwrap(),
            );
            self.gl
                .Uniform1i(program.uniform_location("u_InputSampler0").unwrap(), 0);
        }

        for (i, texture) in textures.iter().enumerate() {
            unsafe {
                self.gl.ActiveTexture(glesv2::TEXTURE1 + i as GLuint);
                self.gl.BindTexture(glesv2::TEXTURE_2D, *texture);
                let i = i as GLint + 1;
                self.gl.Uniform1i(
                    program
                        .uniform_location(&format!("u_InputSampler{}", i))
                        .unwrap(),
                    i,
                );
            }
        }

        if let Some(loc) = program.uniform_location("u_Resolution") {
            unsafe {
                self.gl
                    .Uniform2f(loc, scene.resolution.0 as f32, scene.resolution.1 as f32);
            }
        }

        for ufm in uniforms {
            let loc = program.uniform_location(ufm.0).unwrap();
            unsafe {
                match ufm.1 {
                    UniformValue::Float(x) => self.gl.Uniform1f(loc, x),
                    UniformValue::Vec2(x, y) => self.gl.Uniform2f(loc, x, y),
                    UniformValue::Vec3(x, y, z) => self.gl.Uniform3f(loc, x, y, z),
                    UniformValue::Vec4(x, y, z, w) => self.gl.Uniform4f(loc, x, y, z, w),
                }
            }
        }

        scene.resources.buffer("./quad.abuf").unwrap().bind();
        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_tex_coord = program.attrib_location("a_TexCoord").unwrap() as GLuint;
        let stride = (std::mem::size_of::<f32>() * 5) as GLint;
        unsafe {
            self.gl.EnableVertexAttribArray(index_pos);
            self.gl.VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                0 as *const GLvoid,
            );
            self.gl.EnableVertexAttribArray(index_tex_coord);
            self.gl.VertexAttribPointer(
                index_tex_coord,
                2,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (std::mem::size_of::<f32>() * 3) as *const GLvoid,
            );

            self.gl.DrawArrays(glesv2::TRIANGLES, 0, 6);
        }
    }
}
