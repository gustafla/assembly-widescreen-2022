use super::Shader;
use opengles::glesv2::{self, constants::*, types::*};
use std::path::Path;

pub struct Program(GLuint);

#[derive(Debug)]
pub enum Error {
    Shader(super::shader::Error),
    Link(Option<String>),
}

impl From<super::shader::Error> for Error {
    fn from(e: super::shader::Error) -> Error {
        Error::Shader(e)
    }
}

impl Program {
    pub fn from_sources<P: AsRef<Path>>(paths: &[P]) -> Result<Program, Error> {
        let program = glesv2::create_program();

        let mut shaders = Vec::new();
        for path in paths {
            let shader = Shader::from_source(path)?;
            glesv2::attach_shader(program, shader.handle());
            shaders.push(shader);
        }

        glesv2::link_program(program);

        let status = glesv2::get_programiv(program, GL_LINK_STATUS);
        if status as GLboolean == GL_FALSE {
            let log_len = glesv2::get_programiv(program, GL_INFO_LOG_LENGTH);
            return Err(Error::Link(glesv2::get_program_info_log(program, log_len)));
        }

        eprintln!("Program {} linked", program);
        Ok(Program(program))
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn attrib_location(&self, name: &str) -> GLuint {
        glesv2::get_attrib_location(self.handle(), name) as GLuint
    }

    pub fn uniform_location(&self, name: &str) -> GLint {
        glesv2::get_uniform_location(self.handle(), name)
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        eprintln!("Program {} dropped", self.handle());
        glesv2::delete_program(self.handle());
    }
}
