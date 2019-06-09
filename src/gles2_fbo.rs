use opengles::glesv2::{self, GLenum, GLsizei, GLuint, GLint};

#[derive(Default)]
pub struct Fbo {
    frame_buffer: GLuint,
    render_buffer: Option<GLuint>,
    textures: Vec<GLuint>,
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
enum FboBuilderError {
    RboLimit,
}

#[derive(Default)]
struct FboBuilder {
    fbo: Fbo,
}

impl FboBuilder {
    pub fn add_render_buffer(
        mut self,
        format: GLenum,
        size: (GLsizei, GLsizei),
    ) -> Result<FboBuilder, FboBuilderError> {
        if let Some(_) = self.fbo.render_buffer {
            Err(FboBuilderError::RboLimit)
        } else {
            let rbo = glesv2::gen_renderbuffers(1)[0];
            glesv2::bind_renderbuffer(glesv2::GL_RENDERBUFFER, rbo);
            glesv2::renderbuffer_storage(glesv2::GL_RENDERBUFFER, format, size.0, size.1);
            self.fbo.render_buffer = Some(rbo);
            Ok(self)
        }
    }

    pub fn add_texture(mut self, format: GLenum, size: (GLsizei, GLsizei)) -> FboBuilder {
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
            &[0]);
        self.fbo.textures.push(tex);
        self
    }

    pub fn build(self) -> Result<Fbo, String> {
        
    }
}
