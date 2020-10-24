#![allow(clippy::all)]
#[macro_use]
mod macros;
mod buffer;
mod error;
mod framebuffer;
mod program;
mod renderbuffer;
mod resource_mapper;
mod shader;
mod texture;
mod uniform_value;

pub use self::buffer::Buffer;
pub use self::error::*;
pub use self::framebuffer::{Framebuffer, RenderbufferAttachment, TextureAttachment};
pub use self::program::Program;
pub use self::renderbuffer::Renderbuffer;
pub use self::resource_mapper::ResourceMapper;
pub use self::shader::Shader;
pub use self::texture::Texture;
pub use self::uniform_value::UniformValue;
use std::ffi::CString;
use std::ops::Deref;
use std::rc::Rc;
use types::*;

// Generated by gl_generator. See build.rs
include!(concat!(env!("OUT_DIR"), "/gles2_bindings.rs"));

#[derive(Clone)]
pub struct RcGl(Rc<Gles2>);

impl Deref for RcGl {
    type Target = Rc<Gles2>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for RcGl {
    fn default() -> Self {
        Self::new()
    }
}

impl RcGl {
    pub fn new() -> Self {
        // Static Gles2 struct doesn't need a working loadfn (build.rs)
        let gl = Self(Rc::new(Gles2::load_with(|_| std::ptr::null())));
        log::info!("GL ES 2.0 loaded.");
        gl
    }

    pub fn get_info_log(
        &self,
        handle: GLuint,
        get_iv: unsafe fn(&Gles2, GLuint, GLenum, *mut GLint),
        get_il: unsafe fn(&Gles2, GLuint, GLsizei, *mut GLsizei, *mut GLchar),
    ) -> String {
        let mut log_len = 0;
        unsafe {
            get_iv(self, handle, INFO_LOG_LENGTH, &mut log_len);
        }

        let mut log = vec![0; log_len as usize];

        let mut got_len = 0;
        unsafe {
            get_il(
                self,
                handle,
                log_len,
                &mut got_len,
                log.as_mut_ptr() as *mut GLchar,
            );
        }

        log.truncate(got_len as usize);

        CString::new(log).unwrap().into_string().unwrap()
    }

    pub fn enable(&self, name: GLenum) {
        unsafe {
            self.Enable(name);
        }
    }

    pub fn disable(&self, name: GLenum) {
        unsafe {
            self.Disable(name);
        }
    }

    pub fn viewport(&self, x: GLint, y: GLint, width: GLsizei, height: GLsizei) {
        unsafe {
            self.Viewport(x, y, width, height);
        }
    }

    pub fn blend_func(&self, sfactor: GLenum, dfactor: GLenum) {
        unsafe {
            self.BlendFunc(sfactor, dfactor);
        }
    }

    pub fn depth_func(&self, func: GLenum) {
        unsafe {
            self.DepthFunc(func);
        }
    }

    pub fn get_booleanv(&self, name: GLenum) -> bool {
        let mut value = FALSE;
        unsafe {
            self.GetBooleanv(name, &mut value);
        }
        value == TRUE
    }

    pub fn get_integerv(&self, name: GLenum) -> GLint {
        let mut value = 0;
        unsafe {
            self.GetIntegerv(name, &mut value);
        }
        value
    }
}
