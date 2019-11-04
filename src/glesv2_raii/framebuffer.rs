use super::Renderbuffer;
use super::Texture;
use log::trace;
use opengles::glesv2::{self, constants::*, types::*};
use std::collections::HashMap;

#[derive(Debug)]
pub enum Error {
    IncompleteAttachment,
    DimensionsMismatch,
    MissingAttachment,
    Unsupported,
}

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

        use Error::*;
        match glesv2::check_framebuffer_status(GL_FRAMEBUFFER) {
            GL_FRAMEBUFFER_COMPLETE => Ok(Framebuffer {
                handle,
                textures,
                renderbuffers,
            }),
            GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT => Err(IncompleteAttachment),
            GL_FRAMEBUFFER_INCOMPLETE_DIMENSIONS => Err(DimensionsMismatch),
            GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT => Err(MissingAttachment),
            _ => Err(Unsupported),
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
