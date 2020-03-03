use super::{types::*, RcGl};
use log::trace;

pub struct Texture {
    gl: RcGl,
    handle: GLuint,
}

impl Texture {
    pub fn new(gl: RcGl) -> Texture {
        let mut handle = 0;
        unsafe {
            gl.GenTextures(1, &mut handle);
        }
        trace!("Texture {} created", handle);
        Texture { gl, handle }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn image<T>(
        gl: RcGl,
        target: GLenum,
        level: GLint,
        format: GLenum,
        width: GLsizei,
        height: GLsizei,
        type_: GLenum,
        buffer: Option<&[T]>,
    ) {
        unsafe {
            gl.TexImage2D(
                target,
                level,
                format as GLint,
                width,
                height,
                0,
                format,
                type_,
                if let Some(buffer) = buffer {
                    buffer.as_ptr() as *const GLvoid
                } else {
                    0 as *const GLvoid
                },
            );
        }
    }

    pub fn sub_image<T>(
        gl: RcGl,
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        buffer: &[T],
    ) {
        unsafe {
            gl.TexSubImage2D(
                target,
                level,
                xoffset,
                yoffset,
                width,
                height,
                format,
                type_,
                buffer.as_ptr() as *const GLvoid,
            );
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        trace!("Texture {} dropped", self.handle());
        unsafe {
            self.gl.DeleteTextures(1, &self.handle());
        }
    }
}
