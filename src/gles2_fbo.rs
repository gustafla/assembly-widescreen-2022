use opengles::glesv2::{self, GLenum, GLint, GLsizei, GLubyte, GLuint};

#[derive(Debug)]
pub enum Error {
    UnknownAttachment,
    UnavailableAttachment,
    UnbindableAttachment,
    IncompleteFramebufferAttachment,
    IncompleteFramebufferDimensions,
    FramebufferMissingAttachment,
    FramebufferUnsupported,
}

enum Attachment {
    Texture2D(GLuint),
    Renderbuffer(GLuint),
}

#[derive(Default)]
pub struct Fbo {
    frame_buffer: GLuint,
    depth_attachment: Option<Attachment>,
    stencil_attachment: Option<Attachment>,
    color_attachment: Option<Attachment>,
}

impl Fbo {
    fn ref_for_gl_enum(&mut self, attachment: GLenum) -> Result<&mut Option<Attachment>, Error> {
        match attachment {
            glesv2::GL_COLOR_ATTACHMENT0 => Ok(&mut self.color_attachment),
            glesv2::GL_DEPTH_ATTACHMENT => Ok(&mut self.depth_attachment),
            glesv2::GL_STENCIL_ATTACHMENT => Ok(&mut self.stencil_attachment),
            _ => Err(Error::UnknownAttachment),
        }
    }

    pub fn bind_attachment(&mut self, attachment: GLenum) -> Result<(), Error> {
        match self.ref_for_gl_enum(attachment)? {
            Some(Attachment::Texture2D(handle)) => {
                glesv2::bind_texture(glesv2::GL_TEXTURE_2D, *handle);
                Ok(())
            }
            Some(_) => Err(Error::UnbindableAttachment),
            None => Err(Error::UnavailableAttachment),
        }
    }

    pub fn bind(&self) {
        glesv2::bind_framebuffer(glesv2::GL_FRAMEBUFFER, self.frame_buffer);
    }

    pub fn bind_default() {
        glesv2::bind_framebuffer(glesv2::GL_FRAMEBUFFER, 0);
    }

    fn attachments(&self) -> Vec<(&Option<Attachment>, GLuint)> {
        vec![
            (&self.depth_attachment, glesv2::GL_DEPTH_ATTACHMENT),
            (&self.stencil_attachment, glesv2::GL_STENCIL_ATTACHMENT),
            (&self.color_attachment, glesv2::GL_COLOR_ATTACHMENT0),
        ]
    }
}

impl Drop for Fbo {
    fn drop(&mut self) {
        for attachment in self.attachments() {
            match attachment {
                (Some(Attachment::Texture2D(texture)), _) => glesv2::delete_textures(&[*texture]),
                (Some(Attachment::Renderbuffer(rb)), _) => glesv2::delete_renderbuffers(&[*rb]),
                _ => {}
            }
        }
        glesv2::delete_framebuffers(&[self.frame_buffer]);
        eprintln!("Fbo dropped");
    }
}

#[derive(Default)]
pub struct FboBuilder {
    fbo: Fbo,
}

impl FboBuilder {
    pub fn new() -> Self {
        FboBuilder::default()
    }

    pub fn add_render_buffer(
        mut self,
        format: GLenum,
        size: (GLsizei, GLsizei),
        attachment: GLenum,
    ) -> Result<Self, Error> {
        let target_att = self.fbo.ref_for_gl_enum(attachment)?;
        if let Some(_) = target_att {
            Err(Error::UnavailableAttachment)
        } else {
            let rbo = glesv2::gen_renderbuffers(1)[0];
            glesv2::bind_renderbuffer(glesv2::GL_RENDERBUFFER, rbo);
            glesv2::renderbuffer_storage(glesv2::GL_RENDERBUFFER, format, size.0, size.1);
            *target_att = Some(Attachment::Renderbuffer(rbo));
            Ok(self)
        }
    }

    pub fn add_texture2d(
        mut self,
        format: GLenum,
        size: (GLsizei, GLsizei),
        attachment: GLenum,
    ) -> Result<Self, Error> {
        let target_att = self.fbo.ref_for_gl_enum(attachment)?;
        if let Some(_) = target_att {
            Err(Error::UnavailableAttachment)
        } else {
            let tex = glesv2::gen_textures(1)[0];
            glesv2::bind_texture(glesv2::GL_TEXTURE_2D, tex);
            glesv2::tex_image_2d::<GLubyte>(
                glesv2::GL_TEXTURE_2D,
                0,
                format as GLint,
                size.0,
                size.1,
                0,
                format,
                glesv2::GL_UNSIGNED_BYTE,
                &[],
            );
            *target_att = Some(Attachment::Texture2D(tex));
            Ok(self)
        }
    }

    pub fn build(mut self) -> Result<Fbo, Error> {
        self.fbo.frame_buffer = glesv2::gen_framebuffers(1)[0];
        self.fbo.bind();

        for attachment in self.fbo.attachments().iter() {
            match attachment {
                (Some(Attachment::Texture2D(texture)), gl_enum) => glesv2::framebuffer_texture_2d(
                    glesv2::GL_FRAMEBUFFER,
                    *gl_enum,
                    glesv2::GL_TEXTURE_2D,
                    *texture,
                    0, // Mipmap level must be 0
                ),
                (Some(Attachment::Renderbuffer(rbo)), gl_enum) => glesv2::framebuffer_renderbuffer(
                    glesv2::GL_FRAMEBUFFER,
                    *gl_enum,
                    glesv2::GL_RENDERBUFFER,
                    *rbo,
                ),
                _ => {}
            };
        }

        let status = glesv2::check_framebuffer_status(glesv2::GL_FRAMEBUFFER);
        if status == glesv2::GL_FRAMEBUFFER_COMPLETE {
            Ok(self.fbo)
        } else if status == glesv2::GL_FRAMEBUFFER_INCOMPLETE_ATTACHMENT {
            Err(Error::IncompleteFramebufferAttachment)
        } else if status == glesv2::GL_FRAMEBUFFER_INCOMPLETE_DIMENSIONS {
            Err(Error::IncompleteFramebufferDimensions)
        } else if status == glesv2::GL_FRAMEBUFFER_INCOMPLETE_MISSING_ATTACHMENT {
            Err(Error::FramebufferMissingAttachment)
        } else {
            Err(Error::FramebufferUnsupported)
        }
    }
}
