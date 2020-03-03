use super::Shader;
use super::{types::*, RcGl};
use log::trace;
use std::error;
use std::ffi::CString;
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

pub struct Program {
    gl: RcGl,
    handle: GLuint,
}

impl Program {
    pub fn from_shaders(gl: RcGl, shaders: &[GLuint]) -> Result<Program, Box<dyn error::Error>> {
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
            let info_log = super::get_info_log(
                gl.clone(),
                handle,
                super::Gles2::GetProgramiv,
                super::Gles2::GetProgramInfoLog,
            );

            unsafe {
                gl.DeleteProgram(handle);
            }

            return Err(Box::new(Error(Some(info_log))));
        }

        trace!("Program {} {:?} linked", handle, shaders);
        Ok(Program { gl, handle })
    }

    pub fn from_sources<P: AsRef<Path>>(
        gl: RcGl,
        paths: &[P],
    ) -> Result<Program, Box<dyn error::Error>> {
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
    }

    pub fn handle(&self) -> GLuint {
        self.handle
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
