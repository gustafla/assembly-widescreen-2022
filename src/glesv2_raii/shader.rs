use opengles::glesv2::*;
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;

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

        let shader = create_shader(match path.as_ref().extension() {
            None => return Err(Error::DetermineShaderStage),
            Some(os_str) => match os_str.to_str() {
                Some("frag") => GL_FRAGMENT_SHADER,
                Some("vert") => GL_VERTEX_SHADER,
                _ => return Err(Error::DetermineShaderStage),
            },
        });

        shader_source(shader, string.as_str().as_bytes());
        compile_shader(shader);

        let status = get_shaderiv(shader, GL_COMPILE_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = get_shaderiv(shader, GL_INFO_LOG_LENGTH);
            return Err(Error::Compile(
                format!("{}", path.as_ref().display()),
                get_shader_info_log(shader, log_len),
            ));
        }

        eprintln!("Shader {} created", shader);
        Ok(Shader(shader))
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        eprintln!("Shader {} dropped", self.handle());
        delete_shader(self.handle());
    }
}
