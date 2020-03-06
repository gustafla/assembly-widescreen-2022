use super::{types::*, RcGl};
use log::trace;

pub struct Texture {
    gl: RcGl,
    handle: GLuint,
    target: GLenum,
}

impl Texture {
    pub fn new(gl: RcGl, target: GLenum) -> Texture {
        let mut handle = 0;
        unsafe {
            gl.GenTextures(1, &mut handle);
        }
        trace!("Texture {} created", handle);
        Texture { gl, handle, target }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.BindTexture(self.target, self.handle());
        }
    }

    pub fn image<T>(
        &self,
        level: GLint,
        format: GLenum,
        width: GLsizei,
        height: GLsizei,
        type_: GLenum,
        buffer: Option<&[T]>,
    ) {
        self.bind();
        unsafe {
            self.gl.TexImage2D(
                self.target,
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
        &self,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        buffer: &[T],
    ) {
        self.bind();
        unsafe {
            self.gl.TexSubImage2D(
                self.target,
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

    pub fn parameters(&self, params: &[(GLenum, GLenum)]) {
        self.bind();
        for param in params {
            unsafe {
                self.gl
                    .TexParameteri(self.target, param.0, param.1 as GLint);
            }
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
