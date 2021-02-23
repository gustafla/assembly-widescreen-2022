use super::*;
use log::trace;

pub struct Renderbuffer {
    handle: GLuint,
}

impl Renderbuffer {
    pub fn new() -> Renderbuffer {
        let mut handle = 0;
        unsafe {
            GenRenderbuffers(1, &mut handle);
        }
        trace!("Renderbuffer {} created", handle);
        Renderbuffer { handle }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self) {
        unsafe {
            BindRenderbuffer(RENDERBUFFER, self.handle());
        }
    }

    pub fn storage(&self, format: GLenum, width: GLsizei, height: GLsizei) {
        self.bind();

        unsafe {
            RenderbufferStorage(RENDERBUFFER, format, width, height);
        }
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        trace!("Renderbuffer {} dropped", self.handle());
        unsafe {
            DeleteRenderbuffers(1, &self.handle());
        }
    }
}
