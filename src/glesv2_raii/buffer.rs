use opengles::glesv2::{self, types::*};
use log::trace;

pub struct Buffer(GLuint);

impl Buffer {
    pub fn new() -> Buffer {
        let handle = glesv2::gen_buffers(1)[0];
        trace!("Buffer {} created", handle);
        Buffer(handle)
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        trace!("Buffer {} dropped", self.handle());
        glesv2::delete_buffers(&[self.handle()]);
    }
}
