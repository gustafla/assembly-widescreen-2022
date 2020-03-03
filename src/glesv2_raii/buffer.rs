use log::trace;
use opengles::prelude::*;
use std::error;
use std::fmt;
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
    const BIND: GLenum = GL_ELEMENT_ARRAY_BUFFER;
}

impl BufBinding for f32 {
    const BIND: GLenum = GL_ARRAY_BUFFER;
}

pub struct Buffer {
    handle: GLuint,
    binding: GLenum,
}

impl Buffer {
    pub fn new(binding: GLenum) -> Self {
        let handle = glesv2::gen_buffers(1)[0];
        trace!("Buffer {} created", handle);
        Buffer { handle, binding }
    }

    fn from_string<T: BufBinding + FromStr, P: AsRef<Path>>(
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

        let buffer = Self {
            handle: glesv2::gen_buffers(1)[0],
            binding: T::BIND,
        };

        buffer.bind();
        buffer.data(values.as_slice(), GL_STATIC_DRAW);

        trace!(
            "Buffer {} ({}) created",
            buffer.handle,
            path.as_ref().display()
        );
        Ok(buffer)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn error::Error>> {
        let content = std::fs::read_to_string(&path)?;

        match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("abuf")) => Self::from_string::<f32, P>(content, path),
            Some(Some("ibuf")) => Self::from_string::<i32, P>(content, path),
            _ => Err(Box::new(Error {
                path: PathBuf::from(path.as_ref()),
                kind: ErrorKind::DetermineBufferType,
            })),
        }
    }

    pub fn bind(&self) {
        glesv2::bind_buffer(self.binding, self.handle);
    }

    pub fn data<T>(&self, data: &[T], hint: GLenum) {
        glesv2::buffer_data(self.binding, data, hint);
    }

    pub fn sub_data<T>(&self, offset: i32, data: &[T]) {
        glesv2::buffer_sub_data(self.binding, offset, data);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        trace!("Buffer {} dropped", self.handle);
        glesv2::delete_buffers(&[self.handle]);
    }
}
