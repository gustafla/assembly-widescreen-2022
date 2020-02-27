use log::trace;
use opengles::glesv2::{self, constants::*, types::*};

pub struct Renderbuffer(GLuint);

impl Renderbuffer {
    pub fn new() -> Renderbuffer {
        let handle = glesv2::gen_renderbuffers(1)[0];
        trace!("Renderbuffer {} created", handle);
        Renderbuffer(handle)
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn storage(format: GLenum, width: GLsizei, height: GLsizei) {
        glesv2::renderbuffer_storage(GL_RENDERBUFFER, format, width, height);
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        trace!("Renderbuffer {} dropped", self.handle());
        glesv2::delete_renderbuffers(&[self.handle()]);
    }
}
