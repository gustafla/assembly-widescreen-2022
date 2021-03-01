use super::*;
use std::{
    fmt::{Debug, Display},
    path::{Path, PathBuf},
};
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

#[derive(Clone)]
pub struct Shader {
    handle: GLuint,
    name: String,
}

impl Display for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.handle)
    }
}

impl Debug for Shader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Shader {}", self)
    }
}

impl AsRef<Shader> for Shader {
    fn as_ref(&self) -> &Self {
        &self
    }
}

impl Shader {
    pub fn from_source<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::ReadSourceFile(PathBuf::from(path.as_ref()), e))?;

        let handle = unsafe {
            CreateShader(match path.as_ref().extension().map(|s| s.to_str()) {
                Some(Some("frag")) => FRAGMENT_SHADER,
                Some(Some("vert")) => VERTEX_SHADER,
                _ => return Err(Error::DetermineShaderStage(PathBuf::from(path.as_ref()))),
            })
        };

        let content = content.as_str().as_bytes();
        let length = content.len() as GLsizei;
        unsafe {
            ShaderSource(handle, 1, &(content.as_ptr() as *const GLchar), &length);
            CompileShader(handle);
        }

        let mut status = 0;
        unsafe {
            GetShaderiv(handle, COMPILE_STATUS, &mut status);
        }

        if status as GLboolean == FALSE {
            let info_log = get_info_log(handle, GetShaderiv, GetShaderInfoLog);

            unsafe {
                DeleteShader(handle);
            }

            return Err(Error::Compile(PathBuf::from(path.as_ref()), Some(info_log)));
        }

        let shader = Shader {
            handle,
            name: path.as_ref().display().to_string(),
        };
        log::trace!("Shader {} compiled", shader);
        Ok(shader)
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        log::trace!("Shader {} dropped", self);
        unsafe {
            DeleteShader(self.handle());
        }
    }
}
