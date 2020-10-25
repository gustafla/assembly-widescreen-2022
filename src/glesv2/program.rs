use super::{types::*, RcGl, Shader, UniformValue};
use log::trace;
use std::ffi::CString;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Shader(#[from] super::shader::Error),
    #[error("Failed to link shaders {0:?}: {1:?}")]
    Link(Option<Vec<PathBuf>>, Option<String>),
}

pub struct Program {
    gl: RcGl,
    handle: GLuint,
}

impl Program {
    pub fn from_shaders(gl: RcGl, shaders: &[GLuint]) -> Result<Program, Error> {
        let handle = unsafe { gl.CreateProgram() };

        for shader in shaders {
            unsafe {
                gl.AttachShader(handle, *shader);
            }
        }

        unsafe {
            gl.LinkProgram(handle);
        }

        let mut status = 0;
        unsafe {
            gl.GetProgramiv(handle, super::LINK_STATUS, &mut status);
        }

        if status as GLboolean == super::FALSE {
            let info_log = gl.get_info_log(
                handle,
                super::Gles2::GetProgramiv,
                super::Gles2::GetProgramInfoLog,
            );

            unsafe {
                gl.DeleteProgram(handle);
            }

            return Err(Error::Link(None, Some(info_log)));
        }

        trace!("Program {} {:?} linked", handle, shaders);
        Ok(Program { gl, handle })
    }

    pub fn from_sources<P: AsRef<Path>>(gl: RcGl, paths: &[P]) -> Result<Program, Error> {
        trace!(
            "Linking program from {:?}...",
            paths
                .iter()
                .map(|p| p.as_ref().display().to_string())
                .collect::<Vec<_>>()
        );

        let mut shaders = Vec::new();
        for path in paths {
            shaders.push(Shader::from_source(gl.clone(), path)?);
        }

        Self::from_shaders(
            gl,
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
            self.gl.UseProgram(self.handle());
            if let Some(uniforms) = uniforms {
                for (location, value) in uniforms {
                    match value {
                        UniformValue::Float(x) => {
                            self.gl.Uniform1f(*location, *x);
                        }
                        UniformValue::Vec2f(x, y) => {
                            self.gl.Uniform2f(*location, *x, *y);
                        }
                        UniformValue::Vec3f(x, y, z) => {
                            self.gl.Uniform3f(*location, *x, *y, *z);
                        }
                        UniformValue::Vec4f(x, y, z, w) => {
                            self.gl.Uniform4f(*location, *x, *y, *z, *w);
                        }
                        UniformValue::Floatv(count, ptr) => {
                            self.gl.Uniform1fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec2fv(count, ptr) => {
                            self.gl.Uniform2fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec3fv(count, ptr) => {
                            self.gl.Uniform3fv(*location, *count, *ptr);
                        }
                        UniformValue::Vec4fv(count, ptr) => {
                            self.gl.Uniform4fv(*location, *count, *ptr);
                        }
                        UniformValue::Matrix2fv(count, ptr) => {
                            self.gl
                                .UniformMatrix2fv(*location, *count, super::FALSE, *ptr);
                        }
                        UniformValue::Matrix3fv(count, ptr) => {
                            self.gl
                                .UniformMatrix3fv(*location, *count, super::FALSE, *ptr);
                        }
                        UniformValue::Matrix4fv(count, ptr) => {
                            self.gl
                                .UniformMatrix4fv(*location, *count, super::FALSE, *ptr);
                        }
                        UniformValue::Int(i) => {
                            self.gl.Uniform1i(*location, *i);
                        }
                    }
                }
            }
        }
    }

    pub fn attrib_location(&self, name: &str) -> Option<GLint> {
        let name = CString::new(name).unwrap();
        let loc = unsafe { self.gl.GetAttribLocation(self.handle(), name.as_ptr()) };
        match loc {
            -1 => None,
            _ => Some(loc),
        }
    }

    pub fn uniform_location(&self, name: &str) -> Option<GLint> {
        let name = CString::new(name).unwrap();
        let loc = unsafe { self.gl.GetUniformLocation(self.handle(), name.as_ptr()) };
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
            self.gl.DeleteProgram(self.handle());
        }
    }
}
