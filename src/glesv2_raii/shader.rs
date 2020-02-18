use log::trace;
use opengles::glesv2::{self, constants::*, types::*};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    DetermineShaderStage,
    Compile(String, Option<String>), // file path, error log
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::DetermineShaderStage => writeln!(f, "Failed to determine shader stage"),
            Self::Compile(p, s) => {
                writeln!(f, "Failed to compile shader {}", p)?;
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
        let mut file = File::open(path.as_ref())?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;

        let handle = glesv2::create_shader(match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("frag")) => GL_FRAGMENT_SHADER,
            Some(Some("vert")) => GL_VERTEX_SHADER,
            _ => return Err(Box::new(Error::DetermineShaderStage)),
        });

        glesv2::shader_source(handle, string.as_str().as_bytes());
        glesv2::compile_shader(handle);

        let status = glesv2::get_shaderiv(handle, GL_COMPILE_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = glesv2::get_shaderiv(handle, GL_INFO_LOG_LENGTH);
            let log = glesv2::get_shader_info_log(handle, log_len);
            return Err(Box::new(Error::Compile(format!("{}", path.as_ref().display()), log)));
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
