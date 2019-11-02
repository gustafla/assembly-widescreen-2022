use opengles::glesv2::{self, constants::*, types::*};
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;
use log::trace;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    DetermineShaderStage,
    Compile(String, Option<String>), // file path, error log
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

pub struct Shader(GLuint);

impl Shader {
    pub fn from_source<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut file = File::open(path.as_ref())?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;

        let handle = glesv2::create_shader(match path.as_ref().extension() {
            None => return Err(Error::DetermineShaderStage),
            Some(os_str) => match os_str.to_str() {
                Some("frag") => GL_FRAGMENT_SHADER,
                Some("vert") => GL_VERTEX_SHADER,
                _ => return Err(Error::DetermineShaderStage),
            },
        });

        glesv2::shader_source(handle, string.as_str().as_bytes());
        glesv2::compile_shader(handle);

        let status = glesv2::get_shaderiv(handle, GL_COMPILE_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = glesv2::get_shaderiv(handle, GL_INFO_LOG_LENGTH);
            let log = glesv2::get_shader_info_log(handle, log_len);
            return Err(Error::Compile(format!("{}", path.as_ref().display()), log));
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
