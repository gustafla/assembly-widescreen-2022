use super::{types::*, RcGl};
use log::trace;

pub struct Renderbuffer {
    gl: RcGl,
    handle: GLuint,
}

impl Renderbuffer {
    pub fn new(gl: RcGl) -> Renderbuffer {
        let mut handle = 0;
        unsafe {
            gl.GenRenderbuffers(1, &mut handle);
        }
        trace!("Renderbuffer {} created", handle);
        Renderbuffer { gl, handle }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.BindRenderbuffer(super::RENDERBUFFER, self.handle());
        }
    }

    pub fn storage(&self, format: GLenum, width: GLsizei, height: GLsizei) {
        self.bind();

        unsafe {
            self.gl
                .RenderbufferStorage(super::RENDERBUFFER, format, width, height);
        }
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        trace!("Renderbuffer {} dropped", self.handle());
        unsafe {
            self.gl.DeleteRenderbuffers(1, &self.handle());
        }
    }
}
