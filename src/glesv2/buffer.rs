use super::*;
use log::trace;
use std::mem;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to read file {0}: {1}")]
    ReadFile(PathBuf, std::io::Error),
    #[error("Failed to determine buffer type for {0}")]
    DetermineBufferType(PathBuf),
    #[error("Failed to parse {0} contents")]
    Parse(PathBuf),
}

trait BufBinding {
    const BIND: GLenum;
}

impl BufBinding for i32 {
    const BIND: GLenum = ELEMENT_ARRAY_BUFFER;
}

impl BufBinding for f32 {
    const BIND: GLenum = ARRAY_BUFFER;
}

pub struct Buffer {
    handle: GLuint,
    binding: GLenum,
}

impl Buffer {
    pub fn new(binding: GLenum) -> Self {
        let mut handle: GLuint = 0;
        unsafe {
            GenBuffers(1, &mut handle);
        }
        trace!("Buffer {} created", handle);
        Buffer { handle, binding }
    }

    fn from_string<T: BufBinding + FromStr, P: AsRef<Path>>(
        data: String,
        path: P,
    ) -> Result<Self, Error>
    where
        <T as std::str::FromStr>::Err: std::error::Error + 'static,
    {
        let mut values = Vec::new();
        for v in data.split_ascii_whitespace() {
            values.push(
                v.parse::<T>()
                    .map_err(|_| Error::Parse(PathBuf::from(path.as_ref())))?,
            );
        }

        let buffer = Self::new(T::BIND);

        buffer.bind();
        buffer.data(values.as_slice(), STATIC_DRAW);

        trace!("From {}", path.as_ref().display());
        Ok(buffer)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| Error::ReadFile(PathBuf::from(path.as_ref()), e))?;

        match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("abuf")) => Self::from_string::<f32, P>(content, path),
            Some(Some("ibuf")) => Self::from_string::<i32, P>(content, path),
            _ => Err(Error::DetermineBufferType(PathBuf::from(path.as_ref()))),
        }
    }

    pub fn bind(&self) {
        unsafe {
            BindBuffer(self.binding, self.handle);
        }
    }

    pub fn data<T>(&self, data: &[T], hint: GLenum) {
        unsafe {
            BufferData(
                self.binding,
                (mem::size_of::<T>() * data.len()) as GLsizeiptr,
                data.as_ptr() as *const GLvoid,
                hint,
            );
        }
    }

    pub fn sub_data<T>(&self, offset: GLintptr, data: &[T]) {
        unsafe {
            BufferSubData(
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
            DeleteBuffers(1, &self.handle);
        }
    }
}
