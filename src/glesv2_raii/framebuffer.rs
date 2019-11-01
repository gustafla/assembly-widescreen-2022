use super::Renderbuffer;
use super::Texture;
use opengles::glesv2::*;
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

pub enum AttachmentKind {
    Texture(TextureAttachment),
    Renderbuffer(RenderbufferAttachment),
}

pub struct Attachment {
    pub name: GLenum,
    pub kind: AttachmentKind,
}

pub struct Framebuffer {
    handle: GLuint,
    attachments: HashMap<GLenum, AttachmentKind>,
}

impl Framebuffer {
    pub fn new(attachments: Vec<Attachment>) -> Result<Framebuffer, Error> {
        let handle = gen_framebuffers(1)[0];
        bind_framebuffer(GL_FRAMEBUFFER, handle);

        let mut attach_map = HashMap::new();
        for attachment in attachments {
            match &attachment.kind {
                AttachmentKind::Texture(texture_attachment) => {
                    framebuffer_texture_2d(
                        GL_FRAMEBUFFER,
                        attachment.name,
                        texture_attachment.target,
                        texture_attachment.texture.handle(),
                        texture_attachment.mipmap_level,
                    );
                }
                AttachmentKind::Renderbuffer(renderbuffer_attachment) => {
                    framebuffer_renderbuffer(
                        GL_FRAMEBUFFER,
                        attachment.name,
                        GL_RENDERBUFFER,
                        renderbuffer_attachment.renderbuffer.handle(),
                    );
                }
            }
            attach_map.insert(attachment.name, attachment.kind);
        }

        use Error::*;
        match check_framebuffer_status(GL_FRAMEBUFFER) {
            GL_FRAMEBUFFER_COMPLETE => Ok(Framebuffer {
                handle,
                attachments: attach_map,
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

    pub fn attachment_handle(&self, attachment_name: GLenum) -> Option<GLuint> {
        match self.attachments.get(&attachment_name) {
            Some(attachment_kind) => match attachment_kind {
                AttachmentKind::Texture(texture_attachment) => {
                    Some(texture_attachment.texture.handle())
                }
                AttachmentKind::Renderbuffer(renderbuffer_attachment) => {
                    Some(renderbuffer_attachment.renderbuffer.handle())
                }
            },
            None => None,
        }
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        delete_framebuffers(&[self.handle()]);
    }
}
