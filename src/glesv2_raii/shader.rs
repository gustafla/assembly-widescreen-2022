use log::trace;
use opengles::prelude::*;
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

pub struct Shader(GLuint);

impl Shader {
    pub fn from_source<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn error::Error>> {
        let content = std::fs::read_to_string(&path)?;

        let handle = glesv2::create_shader(match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("frag")) => GL_FRAGMENT_SHADER,
            Some(Some("vert")) => GL_VERTEX_SHADER,
            _ => {
                return Err(Box::new(Error {
                    kind: ErrorKind::DetermineShaderStage,
                    path: PathBuf::from(path.as_ref()),
                }))
            }
        });

        glesv2::shader_source(handle, content.as_str().as_bytes());
        glesv2::compile_shader(handle);

        let status = glesv2::get_shaderiv(handle, GL_COMPILE_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = glesv2::get_shaderiv(handle, GL_INFO_LOG_LENGTH);
            let log = glesv2::get_shader_info_log(handle, log_len);
            return Err(Box::new(Error {
                kind: ErrorKind::Compile(log),
                path: PathBuf::from(path.as_ref()),
            }));
        }

        trace!("Shader {} ({}) compiled", handle, path.as_ref().display());
        Ok(Shader(handle))
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        trace!("Shader {} dropped", self.handle());
        glesv2::delete_shader(self.handle());
    }
}
