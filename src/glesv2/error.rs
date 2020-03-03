use super::{types::*, RcGl};
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
                    INVALID_ENUM,
                    INVALID_VALUE,
                    INVALID_OPERATION,
                    INVALID_FRAMEBUFFER_OPERATION,
                    OUT_OF_MEMORY
                )
            )
        )
    }
}

impl error::Error for Error {}

pub fn check(gl: RcGl) -> Result<(), Error> {
    let status = unsafe { gl.GetError() };
    if status != super::NO_ERROR {
        Err(Error(status))
    } else {
        Ok(())
    }
}