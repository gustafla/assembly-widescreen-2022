use super::{Buffer, Program, RcGl, Shader};
use std::collections::HashMap;
use std::path::{PathBuf, Path};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Cannot read {0}: {1}")]
    FileAccess(PathBuf, std::io::Error),
    #[error(transparent)]
    Shader(#[from] super::shader::Error),
    #[error(transparent)]
    Program(#[from] super::program::Error),
    #[error(transparent)]
    Buffer(#[from] super::buffer::Error),
}

pub struct ResourceMapper {
    datapath: PathBuf,
    #[allow(dead_code)]
    shaders: HashMap<PathBuf, Shader>,
    programs: HashMap<String, Program>,
    buffers: HashMap<PathBuf, Buffer>,
}

impl ResourceMapper {
    pub fn new(gl: RcGl, datapath: impl AsRef<Path>) -> Result<Self, Error> {
        log::trace!("Loading resources");

        let mut shaders = HashMap::new();
        let mut buffers = HashMap::new();

        let datapath = datapath.as_ref().to_path_buf();

        for item in
            std::fs::read_dir(&datapath).map_err(|e| Error::FileAccess(datapath.clone(), e))?
        {
            let path = item
                .map_err(|e| Error::FileAccess(datapath.clone(), e))?
                .path();
            match path.extension().map(|s| s.to_str()) {
                Some(Some("vert")) | Some(Some("frag")) => {
                    shaders.insert(
                        path.clone(),
                        Shader::from_source(gl.clone(), path)?,
                    );
                }
                Some(Some("abuf")) | Some(Some("ibuf")) => {
                    buffers.insert(
                        path.clone(),
                        Buffer::from_file(gl.clone(), path)?,
                    );
                }
                _ => (),
            }
        }

        let mut programs = HashMap::new();

        let programs_desc_path = datapath.join("programs.txt");
        let programs_desc = std::fs::read_to_string(&programs_desc_path)
            .map_err(|e| Error::FileAccess(programs_desc_path, e))?;

        for desc in programs_desc.lines() {
            programs.insert(
                desc.to_string(),
                Program::from_shaders(
                    gl.clone(),
                    desc.split_ascii_whitespace()
                        .map(|p| {
                            shaders
                                .get(&datapath.join(p))
                                .expect(&format!("Shader {} doesn't exist.", p))
                                .handle()
                        })
                        .collect::<Vec<_>>()
                        .as_slice(),
                )?,
            );
        }

        log::trace!("Done, calling glReleaseShaderCompiler");
        unsafe {
            gl.ReleaseShaderCompiler();
        }

        Ok(ResourceMapper {
            datapath,
            shaders,
            programs,
            buffers,
        })
    }

    #[allow(dead_code)]
    fn shader(&self, path: impl AsRef<Path>) -> Option<&Shader> {
        self.shaders.get(&self.datapath.join(path))
    }

    pub fn program(&self, desc: &str) -> Option<&Program> {
        self.programs.get(desc)
    }

    pub fn buffer(&self, path: impl AsRef<Path>) -> Option<&Buffer> {
        self.buffers.get(&self.datapath.join(path))
    }
}
