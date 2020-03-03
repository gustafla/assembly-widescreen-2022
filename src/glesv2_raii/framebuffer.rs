use super::Renderbuffer;
use super::Texture;
use log::trace;
use opengles::prelude::*;
use std::collections::HashMap;
use std::error;
use std::fmt;

#[derive(Debug)]
pub struct Error(GLenum);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "Framebuffer error: {}",
            stringify_match!(
                self.0,
                (
                    GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT,
                    GL_FRAMEBUFFER_INCOMPLETE_DIMENSIONS,
                    GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT,
                    GL_FRAMEBUFFER_UNSUPPORTED
                )
            )
        )
    }
}

impl error::Error for Error {}

pub struct TextureAttachment {
    pub target: GLenum,
    pub texture: Texture,
    pub mipmap_level: GLint,
}

pub struct RenderbufferAttachment {
    pub renderbuffer: Renderbuffer,
}

pub struct Framebuffer {
    handle: GLuint,
    textures: HashMap<GLenum, TextureAttachment>,
    renderbuffers: Vec<RenderbufferAttachment>,
}

impl Framebuffer {
    pub fn new(
        texture_attachments: Option<Vec<(GLenum, TextureAttachment)>>,
        renderbuffer_attachments: Option<Vec<(GLenum, RenderbufferAttachment)>>,
    ) -> Result<Framebuffer, Error> {
        let handle = glesv2::gen_framebuffers(1)[0];
        trace!("Framebuffer {} created", handle);
        glesv2::bind_framebuffer(GL_FRAMEBUFFER, handle);

        let mut textures: HashMap<GLuint, TextureAttachment> = HashMap::new();
        if let Some(texture_attachments) = texture_attachments {
            for (name, attachment) in texture_attachments {
                glesv2::framebuffer_texture_2d(
                    GL_FRAMEBUFFER,
                    name,
                    attachment.target,
                    attachment.texture.handle(),
                    attachment.mipmap_level,
                );
                textures.insert(name, attachment);
            }
        }

        let mut renderbuffers: Vec<RenderbufferAttachment> = Vec::new();
        if let Some(renderbuffer_attachments) = renderbuffer_attachments {
            for (name, attachment) in renderbuffer_attachments {
                glesv2::framebuffer_renderbuffer(
                    GL_FRAMEBUFFER,
                    name,
                    GL_RENDERBUFFER,
                    attachment.renderbuffer.handle(),
                );
                renderbuffers.push(attachment);
            }
        }

        let status = glesv2::check_framebuffer_status(GL_FRAMEBUFFER);
        if status != GL_FRAMEBUFFER_COMPLETE {
            Err(Error(status))
        } else {
            Ok(Framebuffer {
                handle,
                textures,
                renderbuffers,
            })
        }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn texture_handle(&self, texture_attachment_name: GLenum) -> Option<GLuint> {
        match self.textures.get(&texture_attachment_name) {
            Some(texture_attachment) => Some(texture_attachment.texture.handle()),
            None => None,
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        trace!("Framebuffer {} dropped", self.handle());
        glesv2::delete_framebuffers(&[self.handle()]);
    }
}
