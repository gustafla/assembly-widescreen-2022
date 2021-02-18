use crate::glesv2::{
    self, types::*, Framebuffer, RcGl, RenderbufferAttachment, Texture, TextureAttachment,
    UniformValue,
};
use crate::Demo;
use glutin::dpi::PhysicalSize;
use std::convert::TryFrom;

pub struct RenderPass {
    gl: RcGl,
    pub fbo: Framebuffer,
    shader_path: String,
    resolution: PhysicalSize<f32>,
}

impl RenderPass {
    pub fn new(
        gl: RcGl,
        resolution: PhysicalSize<u32>,
        frag_path: &str,
        renderbuffers: Option<Vec<(GLenum, RenderbufferAttachment)>>,
    ) -> Self {
        let fbo_texture = Texture::new(gl.clone(), glesv2::TEXTURE_2D);
        fbo_texture.image::<u8>(
            0,
            glesv2::RGB,
            i32::try_from(resolution.width).unwrap(),
            i32::try_from(resolution.height).unwrap(),
            glesv2::UNSIGNED_BYTE,
            None,
        );
        fbo_texture.parameters(&[
            (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_WRAP_S, glesv2::CLAMP_TO_EDGE),
            (glesv2::TEXTURE_WRAP_T, glesv2::CLAMP_TO_EDGE),
        ]);

        let fbo = Framebuffer::new(
            gl.clone(),
            Some(vec![(
                glesv2::COLOR_ATTACHMENT0,
                TextureAttachment {
                    target: glesv2::TEXTURE_2D,
                    texture: fbo_texture,
                    mipmap_level: 0,
                },
            )]),
            renderbuffers,
        )
        .unwrap();

        Self {
            gl,
            fbo,
            shader_path: format!("shader.vert {}", frag_path),
            resolution: PhysicalSize::new(resolution.width as f32, resolution.height as f32),
        }
    }

    pub fn render(
        &self,
        demo: &Demo,
        textures: &[&Texture],
        uniforms: &[(&str, UniformValue)],
        to_size: Option<PhysicalSize<u32>>,
    ) {
        let program = demo.resources.program(&self.shader_path).unwrap();

        let mut uniforms: Vec<(GLint, UniformValue)> = uniforms
            .iter()
            .map(|(loc, val)| (program.uniform_location(loc).unwrap(), *val))
            .collect();

        uniforms.push((
            program.uniform_location("u_InputSampler0").unwrap(),
            UniformValue::Int(0),
        ));

        self.fbo.texture(glesv2::COLOR_ATTACHMENT0).unwrap().bind(0);
        for (i, texture) in textures.iter().enumerate() {
            let i = i + 1;
            texture.bind(i as GLuint);
            uniforms.push((
                program
                    .uniform_location(&format!("u_InputSampler{}", i))
                    .unwrap(),
                UniformValue::Int(i as GLint),
            ));
        }

        if let Some(loc) = program.uniform_location("u_Resolution") {
            uniforms.push((
                loc,
                UniformValue::Vec2f(self.resolution.width, self.resolution.height),
            ));
        }

        program.bind(Some(&uniforms));

        // Generate a quad
        let mut left = -1.;
        let mut right = 1.;
        let mut down = -1.;
        let mut up = 1.;

        // Letterbox the aspect ratio difference
        if let Some(to_size) = to_size {
            self.gl.viewport(
                0,
                0,
                i32::try_from(to_size.width).unwrap(),
                i32::try_from(to_size.height).unwrap(),
            );

            let from_w = self.resolution.width;
            let from_h = self.resolution.height;
            let to_w = to_size.width as f32;
            let to_h = to_size.height as f32;
            let from_aspect_ratio = from_w / from_h;
            let to_aspect_ratio = to_w / to_h;
            let h_scale = from_h / to_h;
            let w_scale = from_w / to_w;

            if from_aspect_ratio < to_aspect_ratio {
                right = w_scale / h_scale;
                left = -right;
            } else {
                up = h_scale / w_scale;
                down = -up;
            };
        }

        #[rustfmt::skip]
        let quad = [
            left, down, 0., 0., 0.,
            right, down, 0., 1., 0.,
            right, up, 0., 1., 1.,
            left, down, 0., 0., 0.,
            right, up, 0., 1., 1.,
            left, up, 0., 0., 1.,
        ];

        let index_pos = program.attrib_location("a_Pos").unwrap() as GLuint;
        let index_tex_coord = program.attrib_location("a_TexCoord").unwrap() as GLuint;
        let stride = (std::mem::size_of::<f32>() * 5) as GLint;
        unsafe {
            self.gl.BindBuffer(glesv2::ARRAY_BUFFER, 0);
            self.gl.EnableVertexAttribArray(index_pos);
            self.gl.VertexAttribPointer(
                index_pos,
                3,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                quad.as_ptr() as *const GLvoid,
            );
            self.gl.EnableVertexAttribArray(index_tex_coord);
            self.gl.VertexAttribPointer(
                index_tex_coord,
                2,
                glesv2::FLOAT,
                glesv2::FALSE,
                stride,
                (quad.as_ptr().add(3)) as *const GLvoid,
            );

            self.gl.DrawArrays(glesv2::TRIANGLES, 0, 6);
        }
    }
}
