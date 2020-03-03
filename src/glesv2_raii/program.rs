use super::Shader;
use log::trace;
use opengles::prelude::*;
use std::error;
use std::fmt;
use std::path::Path;

#[derive(Debug)]
pub struct Error(Option<String>);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Failed to link program")?; // TODO path
        if let Some(s) = &self.0 {
            writeln!(f, "{}", s)?; // TODO path
        }
        Ok(())
    }
}

impl error::Error for Error {}

pub struct Program(GLuint);

impl Program {
    pub fn from_shaders(shaders: &[GLuint]) -> Result<Program, Box<dyn error::Error>> {
        let handle = glesv2::create_program();

        for shader in shaders {
            glesv2::attach_shader(handle, *shader);
        }

        glesv2::link_program(handle);

        let status = glesv2::get_programiv(handle, GL_LINK_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = glesv2::get_programiv(handle, GL_INFO_LOG_LENGTH);
            let log = glesv2::get_program_info_log(handle, log_len);
            glesv2::delete_program(handle);
            return Err(Box::new(Error(log)));
        }

        trace!("Program {} {:?} linked", handle, shaders);
        Ok(Program(handle))
    }

    pub fn from_sources<P: AsRef<Path>>(paths: &[P]) -> Result<Program, Box<dyn error::Error>> {
        trace!(
            "Linking program from {:?}...",
            paths
                .iter()
                .map(|p| p.as_ref().display().to_string())
                .collect::<Vec<_>>()
        );

        let mut shaders = Vec::new();
        for path in paths {
            shaders.push(Shader::from_source(path)?);
        }

        Self::from_shaders(
            shaders
                .iter()
                .map(|s| s.handle())
                .collect::<Vec<_>>()
                .as_slice(),
        )
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn attrib_location(&self, name: &str) -> Option<GLint> {
        let loc = glesv2::get_attrib_location(self.handle(), name);
        match loc {
            -1 => None,
            _ => Some(loc),
        }
    }

    pub fn uniform_location(&self, name: &str) -> Option<GLint> {
        let loc = glesv2::get_uniform_location(self.handle(), name);
        match loc {
            -1 => None,
            _ => Some(loc),
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        trace!("Program {} dropped", self.handle());
        glesv2::delete_program(self.handle());
    }
}
