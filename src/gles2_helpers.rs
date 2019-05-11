use std::path::Path;
use std::fs::File;
use std::io::{self, prelude::*};
use opengles::glesv2::{self, GLuint, GLboolean};

#[derive(Debug)]
pub enum ShaderError {
    IoError(io::Error),
    TypeDeterminationError,
    CompilationError(String, Option<String>), // file path, error log
    LinkingError(Option<String>),
}

impl From<io::Error> for ShaderError {
    fn from(error: io::Error) -> Self {
        ShaderError::IoError(error)
    }
}

fn compile_shader<P: AsRef<Path>>(path: P) -> Result<GLuint, ShaderError> {
    let mut file = File::open(path.as_ref())?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;

    let shader = glesv2::create_shader(match path.as_ref().extension() {
        None => return Err(ShaderError::TypeDeterminationError),
        Some(os_str) => {
            match os_str.to_str() {
                Some("frag") => glesv2::GL_FRAGMENT_SHADER,
                Some("vert") => glesv2::GL_VERTEX_SHADER,
                _ => return Err(ShaderError::TypeDeterminationError),
            }
        }
    });

    glesv2::shader_source(shader, string.as_str().as_bytes());
    glesv2::compile_shader(shader);

    let status = glesv2::get_shaderiv(shader, glesv2::GL_COMPILE_STATUS);
    if status as GLboolean == glesv2::GL_FALSE {
        let log_len = glesv2::get_shaderiv(shader, glesv2::GL_INFO_LOG_LENGTH);
        return Err(ShaderError::CompilationError(format!("{}", path.as_ref().display()),
        glesv2::get_shader_info_log(shader, log_len)));
    }

    Ok(shader)
}

pub fn link_program<P: AsRef<Path>>(paths: &[P]) -> Result<GLuint, ShaderError> {
    let program = glesv2::create_program();
    let mut shaders = Vec::new();
    for path in paths {
        let shader = compile_shader(path)?;
        shaders.push(shader);
        glesv2::attach_shader(program, shader);
    }

    glesv2::link_program(program);

    let status = glesv2::get_programiv(program, glesv2::GL_LINK_STATUS);
    if status as GLboolean == glesv2::GL_FALSE {
        let log_len = glesv2::get_programiv(program, glesv2::GL_INFO_LOG_LENGTH);
        return Err(ShaderError::LinkingError(glesv2::get_program_info_log(program, log_len)));
    }

    for shader in shaders {
        glesv2::delete_shader(shader);
    }

    Ok(program)
}
