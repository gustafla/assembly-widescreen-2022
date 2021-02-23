use super::*;
use log::trace;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Shader(#[from] shader::Error),
    #[error("Failed to link shaders {0:?}: {1:?}")]
    Link(Option<Vec<PathBuf>>, Option<String>),
}

pub struct Program {
    handle: GLuint,
}

impl Program {
    pub fn from_shaders(shaders: &[GLuint]) -> Result<Program, Error> {
        let handle = unsafe { CreateProgram() };

        for shader in shaders {
            unsafe {
                AttachShader(handle, *shader);
            }
        }

        unsafe {
            LinkProgram(handle);
        }

        let mut status = 0;
        unsafe {
            GetProgramiv(handle, LINK_STATUS, &mut status);
        }

        if status as GLboolean == FALSE {
            let info_log = get_info_log(handle, GetProgramiv, GetProgramInfoLog);

            unsafe {
                DeleteProgram(handle);
            }

            return Err(Error::Link(None, Some(info_log)));
        }

        trace!("Program {} {:?} linked", handle, shaders);
        Ok(Program { handle })
    }

    pub fn from_sources<P: AsRef<Path>>(paths: &[P]) -> Result<Program, Error> {
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
        .map_err(|e| {
            if let Error::Link(None, Some(info_log)) = e {
                Error::Link(
                    Some(paths.iter().map(|p| PathBuf::from(p.as_ref())).collect()),
                    Some(info_log),
                )
            } else {
                e
            }
        })
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self, uniforms: Option<&[(GLint, UniformValue)]>) {
        unsafe {
            UseProgram(self.handle());
            if let Some(uniforms) = uniforms {
                for (location, value) in uniforms {
                    match value {
                        UniformValue::Float(x) => {
                            Uniform1f(*location, *x);
                        }
                        UniformValue::Vec2f(x, y) => {
                            Uniform2f(*location, *x, *y);
                        }
                        UniformValue::Vec3f(x, y, z) => {
                            Uniform3f(*location, *x, *y, *z);
                        }
                        UniformValue::Vec4f(x, y, z, w) => {
                            Uniform4f(*location, *x, *y, *z, *w);
                        }
                        UniformValue::Floatv(count, ptr) => {
                            Uniform1fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec2fv(count, ptr) => {
                            Uniform2fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec3fv(count, ptr) => {
                            Uniform3fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec4fv(count, ptr) => {
                            Uniform4fv(*location, *count, *ptr);
                        }
                        UniformValue::Matrix2fv(count, ptr) => {
                            UniformMatrix2fv(*location, *count, FALSE, *ptr);
                        }
                        UniformValue::Matrix3fv(count, ptr) => {
                            UniformMatrix3fv(*location, *count, FALSE, *ptr);
                        }
                        UniformValue::Matrix4fv(count, ptr) => {
                            UniformMatrix4fv(*location, *count, FALSE, *ptr);
                        }
                        UniformValue::Int(i) => {
                            Uniform1i(*location, *i);
                        }
                    }
                }
            }
        }
    }

    pub fn attrib_location(&self, name: &str) -> Option<GLint> {
        let name = CString::new(name).unwrap();
        let loc = unsafe { GetAttribLocation(self.handle(), name.as_ptr()) };
        match loc {
            -1 => None,
            _ => Some(loc),
        }
    }

    pub fn uniform_location(&self, name: &str) -> Option<GLint> {
        let name = CString::new(name).unwrap();
        let loc = unsafe { GetUniformLocation(self.handle(), name.as_ptr()) };
        match loc {
            -1 => None,
            _ => Some(loc),
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        trace!("Program {} dropped", self.handle());
        unsafe {
            DeleteProgram(self.handle());
        }
    }
}
