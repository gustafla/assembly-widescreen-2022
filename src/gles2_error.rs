use opengles::glesv2::{self, GLenum};

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
            glesv2::GL_INVALID_ENUM => Error::InvalidEnum,
            glesv2::GL_INVALID_VALUE => Error::InvalidValue,
            glesv2::GL_INVALID_OPERATION => Error::InvalidOperation,
            glesv2::GL_INVALID_FRAMEBUFFER_OPERATION => Error::InvalidFramebufferOperation,
            glesv2::GL_OUT_OF_MEMORY => Error::OutOfMemory,
            _ => Error::Unknown,
        }
    }
}

pub fn check() -> Result<(), Error> {
    let status = glesv2::get_error();
    if status != glesv2::GL_NO_ERROR {
        Err(Error::from(status))
    } else {
        Ok(())
    }
}
