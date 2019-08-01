use opengles::glesv2::{self, GLboolean, GLint, GLuint};
use std::fs::File;
use std::io::{self, prelude::*};
use std::path::Path;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    DetermineShaderStage,
    Compile(String, Option<String>), // file path, error log
    Link(Option<String>),
    InvalidUniformSize,
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

        let shader = glesv2::create_shader(match path.as_ref().extension() {
            None => return Err(Error::DetermineShaderStage),
            Some(os_str) => match os_str.to_str() {
                Some("frag") => glesv2::GL_FRAGMENT_SHADER,
                Some("vert") => glesv2::GL_VERTEX_SHADER,
                _ => return Err(Error::DetermineShaderStage),
            },
        });

        glesv2::shader_source(shader, string.as_str().as_bytes());
        glesv2::compile_shader(shader);

        let status = glesv2::get_shaderiv(shader, glesv2::GL_COMPILE_STATUS);
        if status as GLboolean == glesv2::GL_FALSE {
            let log_len = glesv2::get_shaderiv(shader, glesv2::GL_INFO_LOG_LENGTH);
            return Err(Error::Compile(
                format!("{}", path.as_ref().display()),
                glesv2::get_shader_info_log(shader, log_len),
            ));
        }

        eprintln!("Shader {} created", shader);
        Ok(Shader(shader))
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        eprintln!("Shader {} dropped", self.0);
        glesv2::delete_shader(self.0);
    }
}

pub struct Program(GLuint);

impl Program {
    pub fn bind(&self) {
        glesv2::use_program(self.0);
    }

    pub fn attrib(&self, name: &str) -> GLuint {
        glesv2::get_attrib_location(self.0, name) as GLuint
    }

    pub fn uniform(&self, name: &str) -> GLint {
        glesv2::get_uniform_location(self.0, name)
    }

    pub fn from_sources<P: AsRef<Path>>(paths: &[P]) -> Result<Program, Error> {
        let program = glesv2::create_program();

        let mut shaders = Vec::new();
        for path in paths {
            let shader = Shader::from_source(path)?;
            glesv2::attach_shader(program, shader.0);
            shaders.push(shader);
        }

        glesv2::link_program(program);

        let status = glesv2::get_programiv(program, glesv2::GL_LINK_STATUS);
        if status as GLboolean == glesv2::GL_FALSE {
            let log_len = glesv2::get_programiv(program, glesv2::GL_INFO_LOG_LENGTH);
            return Err(Error::Link(glesv2::get_program_info_log(program, log_len)));
        }

        eprintln!("Program {} linked", program);
        Ok(Program(program))
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        eprintln!("Program {} dropped", self.0);
        glesv2::delete_program(self.0);
    }
}
