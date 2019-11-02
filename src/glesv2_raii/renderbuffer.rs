use opengles::glesv2::{self, constants::*, types::*};

pub struct Renderbuffer(GLuint);

impl Renderbuffer {
    pub fn new() -> Renderbuffer {
        Renderbuffer(glesv2::gen_renderbuffers(1)[0])
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn storage<T>(format: GLenum, width: GLsizei, height: GLsizei) {
        glesv2::renderbuffer_storage(GL_RENDERBUFFER, format, width, height);
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        glesv2::delete_renderbuffers(&[self.handle()]);
    }
}
