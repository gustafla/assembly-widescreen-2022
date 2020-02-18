use crate::glesv2_raii::{Buffer, Program, Shader};
use std::collections::HashMap;
use std::error;

pub struct ResourceMapper {
    shaders: HashMap<String, Shader>,
    programs: HashMap<String, Program>,
    buffers: HashMap<String, Buffer>,
}

impl ResourceMapper {
    pub fn new() -> Result<Self, Box<dyn error::Error>> {
        let mut shaders = HashMap::new();
        let mut buffers = HashMap::new();

        for item in std::fs::read_dir("./")? {
            let path = item?.path();
            match path.extension().map(|s| s.to_str()) {
                Some(Some("vert")) | Some(Some("frag")) => {
                    shaders.insert(path.display().to_string(), Shader::from_source(path)?);
                }
                Some(Some("abuf")) | Some(Some("ibuf")) => {
                    buffers.insert(path.display().to_string(), Buffer::from_file(path)?);
                }
                _ => (),
            }
        }

        let mut programs = HashMap::new();

        let programs_desc = std::fs::read_to_string("programs.txt")?;

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
            buffers,
        })
    }

    fn shader(&self, path: &str) -> Option<&Shader> {
        self.shaders.get(path)
    }

    pub fn program(&self, shader_paths: &str) -> Option<&Program> {
        self.programs.get(shader_paths)
    }

    pub fn buffer(&self, path: &str) -> Option<&Buffer> {
        self.buffers.get(path)
    }
}
