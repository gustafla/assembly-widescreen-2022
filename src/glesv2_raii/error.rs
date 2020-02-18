use opengles::glesv2::{self, constants::*, types::*};
use std::fmt;
use std::error;

#[derive(Debug)]
pub struct Error(GLenum);

macro_rules! check {
    ($on:expr, ($($id:ident),+)) => {
        match $on {
            $($id => {stringify!($id)}),+
            _ => "UNKNOWN",
        }
    };
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "OpenGL ES 2.0 error: {}",
            check!(
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
