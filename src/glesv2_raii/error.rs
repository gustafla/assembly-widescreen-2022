use opengles::glesv2::{self, constants::*, types::*};
use std::error;
use std::fmt;

#[derive(Debug)]
pub struct Error(GLenum);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "OpenGL ES 2.0 error: {}",
            stringify_match!(
                self.0,
                (
                    GL_INVALID_ENUM,
                    GL_INVALID_VALUE,
                    GL_INVALID_OPERATION,
                    GL_INVALID_FRAMEBUFFER_OPERATION,
                    GL_OUT_OF_MEMORY
                )
            )
        )
    }
}

impl error::Error for Error {}

pub fn check() -> Result<(), Error> {
    let status = glesv2::get_error();
    if status != GL_NO_ERROR {
        Err(Error(status))
    } else {
        Ok(())
    }
}
