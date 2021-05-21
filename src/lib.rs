mod city;
pub mod glesv2;
mod model;
mod particle_system;
mod player;
mod render_pass;
mod resolution;
mod shader_quad;
mod sync;

use glam::{Mat4, Quat, Vec2, Vec3};
//use particle_system::{
//    ParticleSpawner, ParticleSpawnerKind, ParticleSpawnerMethod, ParticleSystem,
//};
use city::City;
pub use model::Model;
pub use player::Player;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;
use render_pass::RenderPass;
pub use resolution::Resolution;
use shader_quad::ShaderQuad;
use std::convert::TryFrom;
pub use sync::DemoSync;
use thiserror::Error;

const NOISE_SCALE: u32 = 8;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ResourceLoading(#[from] glesv2::resource_mapper::Error),
    #[error(transparent)]
    Framebuffer(#[from] glesv2::framebuffer::Error),
}

pub struct Demo {
    resources: glesv2::ResourceMapper,
    rng: XorShiftRng,
    shadow_fbo: glesv2::Framebuffer,
    //particle_system: ParticleSystem,
    sky: ShaderQuad,
    city: City,
    resolution: Resolution,
    projection: Mat4,
    noise_texture: glesv2::Texture,
    bloom_pass: RenderPass,
    post_pass: RenderPass,
}

impl Demo {
    pub fn resolution(&self) -> Resolution {
        self.resolution
    }

    pub fn new(resolution: impl Into<Resolution>) -> Result<Self, Error> {
        let resolution = resolution.into();
        let mut rng = XorShiftRng::seed_from_u64(98341);

        glesv2::blend_func(glesv2::SRC_ALPHA, glesv2::ONE_MINUS_SRC_ALPHA);
        glesv2::enable(glesv2::CULL_FACE);
        glesv2::depth_func(glesv2::LESS);

        //let particle_system = ParticleSystem::new(
        //    ParticleSpawner::new(
        //        Vec3::new(0., 2., 0.),
        //        ParticleSpawnerKind::Box((-5., 0., -5.), (5., 5., 5.)),
        //        ParticleSpawnerMethod::Once(1000),
        //    ),
        //    30.,
        //    60,
        //    |pos, time| {
        //        Vec3::unit_y() * f32::sin(pos.x / 4. + time) * 0.6
        //            + (Quat::from_axis_angle(Vec3::unit_y(), f32::atan2(pos.x, pos.z))
        //                * Vec3::unit_x()
        //                / Vec2::new(pos.x, pos.z).length()
        //                * (pos.y + 2.))
        //                * (5. - time).max(0.)
        //    },
        //);

        let city = City::new(&mut rng, 20);

        let demo = Demo {
            resources: glesv2::ResourceMapper::new("resources")?,
            rng,
            shadow_fbo: {
                let depth_texture = glesv2::Texture::new(glesv2::TEXTURE_2D);
                // DEPTH_COMPONENT textures are not vanilla ES 2.0 but let's try if it works
                depth_texture.image::<u8>(
                    0,
                    glesv2::DEPTH_COMPONENT,
                    2048,
                    2048,
                    glesv2::FLOAT,
                    None,
                );
                depth_texture.parameters(&[
                    (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
                    (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
                    (glesv2::TEXTURE_WRAP_S, glesv2::CLAMP_TO_EDGE),
                    (glesv2::TEXTURE_WRAP_T, glesv2::CLAMP_TO_EDGE),
                ]);
                glesv2::Framebuffer::new(
                    Some(vec![(
                        glesv2::DEPTH_ATTACHMENT,
                        glesv2::TextureAttachment {
                            target: glesv2::TEXTURE_2D,
                            texture: depth_texture,
                            mipmap_level: 0,
                        },
                    )]),
                    None,
                )?
            },
            //particle_system,
            sky: ShaderQuad::new(resolution, "sky.frag"),
            city,
            resolution,
            noise_texture: {
                let noise_texture = glesv2::Texture::new(glesv2::TEXTURE_2D);
                noise_texture.image::<u8>(
                    0,
                    glesv2::LUMINANCE,
                    i32::try_from(resolution.width / NOISE_SCALE).unwrap(),
                    i32::try_from(resolution.height / NOISE_SCALE).unwrap(),
                    glesv2::UNSIGNED_BYTE,
                    None,
                );
                noise_texture.parameters(&[
                    (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
                    (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
                    (glesv2::TEXTURE_WRAP_S, glesv2::REPEAT),
                    (glesv2::TEXTURE_WRAP_T, glesv2::REPEAT),
                ]);
                noise_texture
            },
            projection: Mat4::perspective_rh_gl(
                60. * (std::f32::consts::PI / 180.),
                resolution.width as f32 / resolution.height as f32,
                0.1,
                1000.,
            ),
            bloom_pass: RenderPass::new(
                resolution,
                "bloom.frag",
                Some(vec![(glesv2::DEPTH_ATTACHMENT, {
                    let renderbuffer = glesv2::Renderbuffer::new();
                    renderbuffer.storage(
                        glesv2::DEPTH_COMPONENT16,
                        i32::try_from(resolution.width).unwrap(),
                        i32::try_from(resolution.height).unwrap(),
                    );
                    glesv2::RenderbufferAttachment { renderbuffer }
                })]),
            ),
            post_pass: {
                let pass = RenderPass::new(resolution, "post.frag", None);
                let fbo = pass.fbo();
                let tex = fbo.texture(glesv2::COLOR_ATTACHMENT0).unwrap();
                tex.parameters(&[(glesv2::TEXTURE_MIN_FILTER, glesv2::LINEAR_MIPMAP_NEAREST)]);
                pass
            },
        };

        log::trace!("demo created");

        Ok(demo)
    }

    pub fn render(
        &mut self,
        sync: &mut DemoSync,
        to_resolution: impl Into<Resolution>,
    ) -> Result<(), glesv2::Error> {
        let to_resolution = to_resolution.into();

        glesv2::clear_color(0., 0., 0., 1.);
        glesv2::viewport(
            0,
            0,
            i32::try_from(self.resolution().width).unwrap(),
            i32::try_from(self.resolution().height).unwrap(),
        );

        let view = Mat4::from_euler(
            glam::EulerRot::ZXY,
            sync.get("cam:roll") * std::f32::consts::PI,
            sync.get("cam:pitch") * std::f32::consts::PI,
            sync.get("cam:yaw") * std::f32::consts::PI,
        ) * Mat4::from_translation(-Vec3::new(
            sync.get("cam:pos.x"),
            sync.get("cam:pos.y"),
            sync.get("cam:pos.z"),
        ));

        // 3D models ------------------------------------------------------------------------------

        glesv2::enable(glesv2::DEPTH_TEST);

        // Render depth from the sun's perspective
        // Generate sun's matrix etc
        self.shadow_fbo.bind(glesv2::DEPTH_BUFFER_BIT);
        // TODO render the city but with the sun's matrices

        // Render actual image
        self.bloom_pass
            .fbo()
            .bind(glesv2::COLOR_BUFFER_BIT | glesv2::DEPTH_BUFFER_BIT);

        //let sim_time = sync.get("sim_time");
        //self.particle_system.prepare(sim_time, cam_pos);
        self.sky.render(&self, &[], &[], None);
        self.city.render(&self, sync, &self.projection, &view);
        //self.particle_system.render(&self);

        // Bloom pass -----------------------------------------------------------------------------

        self.post_pass.fbo().bind(0);
        self.bloom_pass.render(&self, &[], &[], None);

        // Post pass ------------------------------------------------------------------------------

        glesv2::Framebuffer::bind_default(glesv2::COLOR_BUFFER_BIT);

        // Generate noise
        let noise: Vec<u8> = (0..(self.resolution().width * self.resolution().height
            / NOISE_SCALE.pow(2)))
            .map(|_| self.rng.gen())
            .collect();

        // Upload noise to Texture
        self.noise_texture.sub_image::<u8>(
            0,
            0,
            0,
            i32::try_from(self.resolution().width / NOISE_SCALE).unwrap(),
            i32::try_from(self.resolution().height / NOISE_SCALE).unwrap(),
            glesv2::LUMINANCE,
            glesv2::UNSIGNED_BYTE,
            noise.as_slice(),
        );

        // Mipmap for blur
        self.post_pass
            .fbo()
            .texture(glesv2::COLOR_ATTACHMENT0)
            .unwrap()
            .generate_mipmaps();

        self.post_pass.render(
            &self,
            &[
                self.bloom_pass
                    .fbo()
                    .texture(glesv2::COLOR_ATTACHMENT0)
                    .unwrap(),
                &self.noise_texture,
            ],
            &[
                (
                    "u_NoiseAmount",
                    glesv2::UniformValue::Float(sync.get("noise_amount")),
                ),
                (
                    "u_NoiseScale",
                    glesv2::UniformValue::Float(NOISE_SCALE as f32),
                ),
                ("u_Beat", glesv2::UniformValue::Float(sync.get_beat())),
            ],
            Some(to_resolution),
        );

        glesv2::check()
    }

    #[cfg(debug_assertions)]
    pub fn reload(&mut self) -> Result<(), Error> {
        self.resources = glesv2::ResourceMapper::new("resources")?;
        Ok(())
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        log::trace!("demo dropped");
    }
}
