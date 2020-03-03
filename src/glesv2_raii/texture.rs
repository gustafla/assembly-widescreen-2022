use log::trace;
use opengles::prelude::*;

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

    pub fn sub_image<T>(
        target: GLenum,
        level: GLint,
        xoffset: GLint,
        yoffset: GLint,
        width: GLsizei,
        height: GLsizei,
        format: GLenum,
        type_: GLenum,
        buffer: &[T],
    ) {
        glesv2::tex_sub_image_2d(
            target, level, xoffset, yoffset, width, height, format, type_, buffer,
        );
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        trace!("Texture {} dropped", self.handle());
        glesv2::delete_textures(&[self.handle()]);
    }
}
