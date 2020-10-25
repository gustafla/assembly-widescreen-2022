use super::{types::*, RcGl};
use log::trace;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to read shader source file {0}: {1}")]
    ReadSourceFile(PathBuf, std::io::Error),
    #[error("Failed to determine the stage of shader {0}")]
    DetermineShaderStage(PathBuf),
    #[error("Compiling shader {0} failed: {1:?}")]
    Compile(PathBuf, Option<String>),
}

pub struct Shader {
    gl: RcGl,
    handle: GLuint,
}

impl Shader {
    pub fn from_source<P: AsRef<Path>>(gl: RcGl, path: P) -> Result<Self, Error> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::ReadSourceFile(PathBuf::from(path.as_ref()), e))?;

        let handle = unsafe {
            gl.CreateShader(match path.as_ref().extension().map(|s| s.to_str()) {
                Some(Some("frag")) => super::FRAGMENT_SHADER,
                Some(Some("vert")) => super::VERTEX_SHADER,
                _ => return Err(Error::DetermineShaderStage(PathBuf::from(path.as_ref()))),
            })
        };

        let content = content.as_str().as_bytes();
        let length = content.len() as GLsizei;
        unsafe {
            gl.ShaderSource(handle, 1, &(content.as_ptr() as *const GLchar), &length);
            gl.CompileShader(handle);
        }

        let mut status = 0;
        unsafe {
            gl.GetShaderiv(handle, super::COMPILE_STATUS, &mut status);
        }

        if status as GLboolean == super::FALSE {
            let info_log = gl.get_info_log(
                handle,
                super::Gles2::GetShaderiv,
                super::Gles2::GetShaderInfoLog,
            );

            unsafe {
                gl.DeleteShader(handle);
            }

            return Err(Error::Compile(PathBuf::from(path.as_ref()), Some(info_log)));
        }

        trace!("Shader {} ({}) compiled", handle, path.as_ref().display());
        Ok(Shader { gl, handle })
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        trace!("Shader {} dropped", self.handle());
        unsafe {
            self.gl.DeleteShader(self.handle());
        }
    }
}
