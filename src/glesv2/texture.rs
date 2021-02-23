use super::*;
use log::trace;

pub struct Texture {
    handle: GLuint,
    target: GLenum,
}

impl Texture {
    pub fn new(target: GLenum) -> Texture {
        let mut handle = 0;
        unsafe {
            GenTextures(1, &mut handle);
        }
        trace!("Texture {} created", handle);
        Texture { handle, target }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self, unit: GLuint) {
        unsafe {
            ActiveTexture(TEXTURE0 + unit);
            BindTexture(self.target, self.handle());
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
        self.bind(0);
        unsafe {
            TexImage2D(
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
                    std::ptr::null()
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
        self.bind(0);
        unsafe {
            TexSubImage2D(
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
        self.bind(0);
        for param in params {
            unsafe {
                TexParameteri(self.target, param.0, param.1 as GLint);
            }
        }
    }

    pub fn generate_mipmaps(&self) {
        self.bind(0);
        unsafe {
            GenerateMipmap(self.target);
        }
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        trace!("Texture {} dropped", self.handle());
        unsafe {
            DeleteTextures(1, &self.handle());
        }
    }
}
