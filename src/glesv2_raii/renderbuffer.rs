use opengles::glesv2::*;

pub struct Renderbuffer(GLuint);

impl Renderbuffer {
    pub fn new() -> Renderbuffer {
        Renderbuffer(gen_renderbuffers(1)[0])
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn storage<T>(format: GLenum, width: GLsizei, height: GLsizei) {
        renderbuffer_storage(GL_RENDERBUFFER, format, width, height);
    }
}

impl Drop for Renderbuffer {
    fn drop(&mut self) {
        delete_renderbuffers(&[self.handle()]);
    }
}
