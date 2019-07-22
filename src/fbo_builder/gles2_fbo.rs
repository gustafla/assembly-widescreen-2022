use opengles::glesv2::{self, GLuint};

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
