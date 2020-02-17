use crate::glesv2_raii::{Buffer, Program, Shader};
use opengles::glesv2::{self, constants::*};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::num;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    ParseFloat(num::ParseFloatError),
    Shader(super::shader::Error),
    Program(super::program::Error),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl From<num::ParseFloatError> for Error {
    fn from(error: num::ParseFloatError) -> Self {
        Error::ParseFloat(error)
    }
}

impl From<super::shader::Error> for Error {
    fn from(error: super::shader::Error) -> Self {
        Error::Shader(error)
    }
}

impl From<super::program::Error> for Error {
    fn from(error: super::program::Error) -> Self {
        Error::Program(error)
    }
}

pub struct ResourceMapper {
    shaders: HashMap<String, Shader>,
    programs: HashMap<String, Program>,
    array_buffers: HashMap<String, Buffer>,
}

impl ResourceMapper {
    pub fn new() -> Result<Self, Error> {
        let mut shaders = HashMap::new();
        let mut array_buffers = HashMap::new();

        for item in std::fs::read_dir("./")? {
            let path = item.unwrap().path();
            match path.extension().map(|s| s.to_str().unwrap()) {
                Some("vert") | Some("frag") => {
                    shaders.insert(path.display().to_string(), Shader::from_source(path)?);
                }
                Some("abuf") => {
                    let mut file = File::open(&path)?;
                    let mut content = String::new();
                    file.read_to_string(&mut content)?;

                    let mut values = Vec::new();
                    for v in content.split_ascii_whitespace() {
                        values.push(v.parse::<f32>()?);
                    }

                    let buffer = Buffer::new();
                    glesv2::bind_buffer(GL_ARRAY_BUFFER, buffer.handle());
                    glesv2::buffer_data(GL_ARRAY_BUFFER, values.as_slice(), GL_STATIC_DRAW);
                    array_buffers.insert(path.display().to_string(), buffer);
                }
                _ => (),
            }
        }

        let mut programs = HashMap::new();

        let mut programs_file = File::open("programs.txt")?;
        let mut programs_desc = String::new();
        programs_file.read_to_string(&mut programs_desc)?;

        for desc in programs_desc.lines() {
            programs.insert(
                desc.to_string(),
                Program::from_shaders(
                    desc.split_ascii_whitespace()
                        .map(|p| {
                            shaders
                                .get(p)
                                .expect(&format!("Shader {} doesn't exist.", p))
                                .handle()
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                )?,
            );
        }

        Ok(ResourceMapper {
            shaders,
            programs,
            array_buffers,
        })
    }

    fn shader(&self, path: &str) -> &Shader {
        self.shaders.get(path).unwrap()
    }

    pub fn program(&self, shader_paths: &str) -> &Program {
        self.programs.get(shader_paths).unwrap()
    }

    pub fn array_buffer(&self, path: &str) -> &Buffer {
        self.array_buffers.get(path).unwrap()
    }
}
