use super::{types::*, RcGl};
use log::trace;
use std::error;
use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
enum ErrorKind {
    DetermineShaderStage,
    Compile(Option<String>), // file path, error log
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    path: PathBuf,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            ErrorKind::DetermineShaderStage => {
                writeln!(f, "Failed to determine stage of {}", self.path.display())
            }
            ErrorKind::Compile(s) => {
                writeln!(f, "{}: shader compiler error", self.path.display())?;
                if let Some(s) = s {
                    writeln!(f, "{}", s)?;
                }
                Ok(())
            }
        }
    }
}

impl error::Error for Error {}

pub struct Shader {
    gl: RcGl,
    handle: GLuint,
}

impl Shader {
    pub fn from_source<P: AsRef<Path>>(gl: RcGl, path: P) -> Result<Self, Box<dyn error::Error>> {
        let content = std::fs::read_to_string(&path)?;

        let handle = unsafe {
            gl.CreateShader(match path.as_ref().extension().map(|s| s.to_str()) {
                Some(Some("frag")) => super::FRAGMENT_SHADER,
                Some(Some("vert")) => super::VERTEX_SHADER,
                _ => {
                    return Err(Box::new(Error {
                        kind: ErrorKind::DetermineShaderStage,
                        path: PathBuf::from(path.as_ref()),
                    }))
                }
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

            return Err(Box::new(Error {
                kind: ErrorKind::Compile(Some(info_log)),
                path: PathBuf::from(path.as_ref()),
            }));
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
