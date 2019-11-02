use opengles::glesv2::{self, types::*};

pub struct Buffer(GLuint);

impl Buffer {
    pub fn new() -> Buffer {
        let handle = glesv2::gen_buffers(1)[0];
        eprintln!("Buffer handle {} created", handle);
        Buffer(handle)
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        eprintln!("Buffer handle {} dropped", self.0);
        glesv2::delete_buffers(&[self.0]);
    }
}
