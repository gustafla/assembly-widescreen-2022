use log::trace;
use opengles::glesv2::{self, constants::*, types::*};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug)]
enum ErrorKind {
    Io(std::io::Error),
    DetermineBufferType,
    Parse,
}

impl From<std::io::Error> for ErrorKind {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct Error {
    path: PathBuf,
    kind: ErrorKind,
}

macro_rules! make_error {
    ($path:ident) => {
        |e| Error {
            path: PathBuf::from($path.as_ref()),
            kind: ErrorKind::from(e),
        }
    };
}

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
    ) -> Result<Self, Error> {
        let mut values = Vec::new();
        for v in data.split_ascii_whitespace() {
            values.push(v.parse::<T>().map_err(|_| Error {
                path: PathBuf::from(path.as_ref()),
                kind: ErrorKind::Parse,
            })?);
        }

        let buffer = Self {
            handle: glesv2::gen_buffers(1)[0],
            binding: T::BIND,
        };

        buffer.bind();
        glesv2::buffer_data(T::BIND, values.as_slice(), GL_STATIC_DRAW);

        trace!("Buffer {} ({}) created", buffer.handle, path.as_ref().display());
        Ok(buffer)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let mut file = File::open(&path).map_err(make_error!(path))?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(make_error!(path))?;

        match path.as_ref().extension().map(|s| s.to_str()) {
            Some(Some("abuf")) => Self::from_string::<f32, P>(content, path),
            Some(Some("ibuf")) => Self::from_string::<i32, P>(content, path),
            _ => Err(Error {
                path: PathBuf::from(path.as_ref()),
                kind: ErrorKind::DetermineBufferType,
            }),
        }
    }

    pub fn bind(&self) {
        glesv2::bind_buffer(self.binding, self.handle);
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        trace!("Buffer {} dropped", self.handle);
        glesv2::delete_buffers(&[self.handle]);
    }
}
