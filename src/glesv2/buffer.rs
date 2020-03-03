use super::{types::*, RcGl};
use log::trace;
use std::error;
use std::fmt;
use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
enum ErrorKind {
    DetermineBufferType,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    path: PathBuf,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::DetermineBufferType => writeln!(
                f,
                "Failed to determine buffer type for {}",
                self.path.display()
            ),
        }
    }
}

impl error::Error for Error {}

trait BufBinding {
    const BIND: GLenum;
}

impl BufBinding for i32 {
    const BIND: GLenum = super::ELEMENT_ARRAY_BUFFER;
}

impl BufBinding for f32 {
    const BIND: GLenum = super::ARRAY_BUFFER;
}

pub struct Buffer {
    gl: RcGl,
    handle: GLuint,
    binding: GLenum,
}

impl Buffer {
    pub fn new(gl: RcGl, binding: GLenum) -> Self {
        let mut handle: GLuint = 0;
        unsafe {
            gl.GenBuffers(1, &mut handle);
        }
        trace!("Buffer {} created", handle);
        Buffer {
            gl,
            handle,
            binding,
        }
    }

    fn from_string<T: BufBinding + FromStr, P: AsRef<Path>>(
        gl: RcGl,
        data: String,
        path: P,
    ) -> Result<Self, Box<dyn error::Error>>
    where
        <T as std::str::FromStr>::Err: std::error::Error + 'static,
    {
        let mut values = Vec::new();
        for v in data.split_ascii_whitespace() {
            values.push(v.parse::<T>()?);
        }

        let buffer = Self::new(gl, T::BIND);

        buffer.bind();
        buffer.data(values.as_slice(), super::STATIC_DRAW);

        trace!("From {}", path.as_ref().display());
        Ok(buffer)
    }

    pub fn from_file<P: AsRef<Path>>(gl: RcGl, path: P) -> Result<Self, Box<dyn error::Error>> {
        let content = std::fs::read_to_string(&path)?;

        match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("abuf")) => Self::from_string::<f32, P>(gl, content, path),
            Some(Some("ibuf")) => Self::from_string::<i32, P>(gl, content, path),
            _ => Err(Box::new(Error {
                path: PathBuf::from(path.as_ref()),
                kind: ErrorKind::DetermineBufferType,
            })),
        }
    }

    pub fn bind(&self) {
        unsafe {
            self.gl.BindBuffer(self.binding, self.handle);
        }
    }

    pub fn data<T>(&self, data: &[T], hint: GLenum) {
        unsafe {
            self.gl.BufferData(
                self.binding,
                (mem::size_of::<T>() * data.len()) as GLsizeiptr,
                data.as_ptr() as *const GLvoid,
                hint,
            );
        }
    }

    pub fn sub_data<T>(&self, offset: GLintptr, data: &[T]) {
        unsafe {
            self.gl.BufferSubData(
                self.binding,
                offset,
                (mem::size_of::<T>() * data.len()) as GLsizeiptr,
                data.as_ptr() as *const GLvoid,
            );
        }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        trace!("Buffer {} dropped", self.handle);
        unsafe {
            self.gl.DeleteBuffers(1, &self.handle);
        }
    }
}
