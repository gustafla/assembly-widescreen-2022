use opengles::glesv2::{self, constants::*, types::*};
use log::trace;

pub struct Texture(GLuint);

impl Texture {
    pub fn new() -> Texture {
        let handle = glesv2::gen_textures(1)[0];
        trace!("Texture {} created", handle);
        Texture(handle)
    }

    pub fn handle(&self) -> GLuint {
        self.0
    }

    pub fn set_filters(target: GLenum, param: GLenum) {
        glesv2::tex_parameteri(target, GL_TEXTURE_MIN_FILTER, param as GLint);
        glesv2::tex_parameteri(target, GL_TEXTURE_MAG_FILTER, param as GLint);
    }

    pub fn image<T>(
        target: GLenum,
        level: GLint,
        format: GLenum,
        width: GLsizei,
        height: GLsizei,
        type_: GLenum,
        buffer: &[T],
    ) {
        glesv2::tex_image_2d(
            target,
            level,
            format as GLint,
            width,
            height,
            0,
            format,
            type_,
            buffer,
        );
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        trace!("Texture {} dropped", self.handle());
        glesv2::delete_textures(&[self.handle()]);
    }
}
