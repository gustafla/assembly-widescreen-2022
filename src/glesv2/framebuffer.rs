use super::{types::*, RcGl, Renderbuffer, Texture};
use log::trace;
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
                    FRAMEBUFFER_INCOMPLETE_ATTACHMENT,
                    FRAMEBUFFER_INCOMPLETE_DIMENSIONS,
                    FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT,
                    FRAMEBUFFER_UNSUPPORTED
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
    gl: RcGl,
    handle: GLuint,
    textures: HashMap<GLenum, TextureAttachment>,
    _renderbuffers: Vec<RenderbufferAttachment>,
}

impl Framebuffer {
    pub fn new(
        gl: RcGl,
        texture_attachments: Option<Vec<(GLenum, TextureAttachment)>>,
        renderbuffer_attachments: Option<Vec<(GLenum, RenderbufferAttachment)>>,
    ) -> Result<Framebuffer, Error> {
        let mut handle = 0;
        unsafe {
            gl.GenFramebuffers(1, &mut handle);
            trace!("Framebuffer {} created", handle);
            gl.BindFramebuffer(super::FRAMEBUFFER, handle);
        }

        let mut textures: HashMap<GLuint, TextureAttachment> = HashMap::new();
        if let Some(texture_attachments) = texture_attachments {
            for (name, attachment) in texture_attachments {
                unsafe {
                    gl.FramebufferTexture2D(
                        super::FRAMEBUFFER,
                        name,
                        attachment.target,
                        attachment.texture.handle(),
                        attachment.mipmap_level,
                    );
                }
                textures.insert(name, attachment);
            }
        }

        let mut _renderbuffers: Vec<RenderbufferAttachment> = Vec::new();
        if let Some(renderbuffer_attachments) = renderbuffer_attachments {
            for (name, attachment) in renderbuffer_attachments {
                unsafe {
                    gl.FramebufferRenderbuffer(
                        super::FRAMEBUFFER,
                        name,
                        super::RENDERBUFFER,
                        attachment.renderbuffer.handle(),
                    );
                }
                _renderbuffers.push(attachment);
            }
        }

        let status = unsafe { gl.CheckFramebufferStatus(super::FRAMEBUFFER) };
        if status != super::FRAMEBUFFER_COMPLETE {
            Err(Error(status))
        } else {
            Ok(Framebuffer {
                gl,
                handle,
                textures,
                _renderbuffers,
            })
        }
    }

    pub fn handle(&self) -> GLuint {
        self.handle
    }

    pub fn bind(&self, clear_flags: GLbitfield) {
        unsafe {
            self.gl.BindFramebuffer(super::FRAMEBUFFER, self.handle());
            self.gl.Clear(clear_flags);
        }
    }

    pub fn bind_default(gl: RcGl, clear_flags: GLbitfield) {
        unsafe {
            gl.BindFramebuffer(super::FRAMEBUFFER, 0);
            gl.Clear(clear_flags);
        }
    }

    pub fn texture(&self, texture_attachment_name: GLenum) -> Option<&Texture> {
        match self.textures.get(&texture_attachment_name) {
            Some(texture_attachment) => Some(&texture_attachment.texture),
            None => None,
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        trace!("Framebuffer {} dropped", self.handle());
        unsafe {
            self.gl.DeleteFramebuffers(1, &self.handle());
        }
    }
}
