use opengles::glesv2::{self, GLuint, GLenum, GLint, GLsizei};

#[derive(Default)]
pub struct Fbo {
    pub frame_buffer: GLuint,
    pub render_buffer: Option<GLuint>,
    pub textures: Vec<GLuint>,
}

impl Fbo {
    pub fn bind(&self) {
        glesv2::bind_framebuffer(glesv2::GL_FRAMEBUFFER, self.frame_buffer);
    }
}

impl Drop for Fbo {
    fn drop(&mut self) {
        if let Some(rbo) = self.render_buffer {
            glesv2::delete_renderbuffers(&[rbo]);
        }
        glesv2::delete_textures(&self.textures);
        glesv2::delete_framebuffers(&[self.frame_buffer]);
    }
}

#[derive(Debug)]
pub enum Error {
    RboLimit,
}

pub struct FboBuilder {
    fbo: Fbo,
}

impl FboBuilder {
    pub fn add_render_buffer(
        mut self,
        format: GLenum,
        size: (GLsizei, GLsizei),
    ) -> Result<Self, Error> {
        if let Some(_) = self.fbo.render_buffer {
            Err(Error::RboLimit)
        } else {
            let rbo = glesv2::gen_renderbuffers(1)[0];
            glesv2::bind_renderbuffer(glesv2::GL_RENDERBUFFER, rbo);
            glesv2::renderbuffer_storage(glesv2::GL_RENDERBUFFER, format, size.0, size.1);
            self.fbo.render_buffer = Some(rbo);
            Ok(self)
        }
    }

    pub fn add_texture(mut self, format: GLenum, size: (GLsizei, GLsizei)) -> Self {
        let tex = glesv2::gen_textures(1)[0];
        glesv2::bind_texture(glesv2::GL_TEXTURE_2D, tex);
        glesv2::tex_image_2d(
            glesv2::GL_TEXTURE_2D,
            0,
            format as GLint,
            size.0,
            size.1,
            0,
            format,
            glesv2::GL_UNSIGNED_BYTE,
            &[0], // TODO probably crashes
        );
        self.fbo.textures.push(tex);
        self
    }

    pub fn build(self) -> Result<Fbo, Error> {
        let fbo = glesv2::gen_framebuffers(1)[0];
        glesv2::bind_framebuffer(glesv2::GL_FRAMEBUFFER, fbo);
        if let Some(rbo) = self.fbo.render_buffer {
            glesv2::framebuffer_renderbuffer(
                glesv2::GL_FRAMEBUFFER,
                glesv2::GL_DEPTH_ATTACHMENT,
                glesv2::GL_RENDERBUFFER,
                rbo,
            );
        }
        for texture in &self.fbo.textures {
            glesv2::framebuffer_texture_2d(
                glesv2::GL_FRAMEBUFFER,
                glesv2::GL_COLOR_ATTACHMENT0, // TODO might want to pass this
                glesv2::GL_TEXTURE_2D, // TODO might want to pass this
                *texture,
                0, // Mipmap level must be 0
            );
        };
        Ok(self.fbo)
    }
}
