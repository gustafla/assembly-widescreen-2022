use opengles::glesv2::*;

#[derive(Debug)]
pub enum Error {
    Unknown,
    InvalidEnum,
    InvalidValue,
    InvalidOperation,
    InvalidFramebufferOperation,
    OutOfMemory,
}

impl From<GLenum> for Error {
    fn from(value: GLenum) -> Self {
        match value {
            GL_INVALID_ENUM => Error::InvalidEnum,
            GL_INVALID_VALUE => Error::InvalidValue,
            GL_INVALID_OPERATION => Error::InvalidOperation,
            GL_INVALID_FRAMEBUFFER_OPERATION => Error::InvalidFramebufferOperation,
            GL_OUT_OF_MEMORY => Error::OutOfMemory,
            _ => Error::Unknown,
        }
    }
}

pub fn check() -> Result<(), Error> {
    let status = get_error();
    if status != GL_NO_ERROR {
        Err(Error::from(status))
    } else {
        Ok(())
    }
}
