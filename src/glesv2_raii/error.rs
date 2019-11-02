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
        use Error::*;
        match value {
            GL_INVALID_ENUM => InvalidEnum,
            GL_INVALID_VALUE => InvalidValue,
            GL_INVALID_OPERATION => InvalidOperation,
            GL_INVALID_FRAMEBUFFER_OPERATION => InvalidFramebufferOperation,
            GL_OUT_OF_MEMORY => OutOfMemory,
            _ => Unknown,
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
