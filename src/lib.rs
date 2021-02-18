mod glesv2;
mod particle_system;
mod player;
mod render_pass;
mod sync;
mod terrain;

use glam::{Mat4, Quat, Vec2, Vec3};
pub use glesv2::{
    types::*, Framebuffer, Gles2, RcGl, Renderbuffer, RenderbufferAttachment, ResourceMapper,
    Texture, UniformValue,
};
use glutin::dpi::PhysicalSize;
use particle_system::{
    ParticleSpawner, ParticleSpawnerKind, ParticleSpawnerMethod, ParticleSystem,
};
pub use player::Player;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;
use render_pass::RenderPass;
use std::convert::TryFrom;
pub use sync::Sync;
use terrain::Terrain;
use thiserror::Error;

const NOISE_SCALE: u32 = 8;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ResourceLoading(#[from] glesv2::resource_mapper::Error),
}

struct DemoResizables {
    resolution: PhysicalSize<u32>,
    projection: Mat4,
    noise_texture: Texture,
    bloom_pass: RenderPass,
    blur_pass_x: RenderPass,
    blur_pass_y: RenderPass,
    post_pass: RenderPass,
}

impl DemoResizables {
    fn new(gl: RcGl, size: PhysicalSize<u32>) -> Self {
        Self {
            resolution: size,
            noise_texture: {
                let noise_texture = Texture::new(gl.clone(), glesv2::TEXTURE_2D);
                noise_texture.image::<u8>(
                    0,
                    glesv2::LUMINANCE,
                    i32::try_from(size.width / NOISE_SCALE).unwrap(),
                    i32::try_from(size.height / NOISE_SCALE).unwrap(),
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
                size.width as f32 / size.height as f32,
                0.1,
                1000.,
            ),
            bloom_pass: RenderPass::new(
                gl.clone(),
                size,
                "bloom.frag",
                Some(vec![(glesv2::DEPTH_ATTACHMENT, {
                    let renderbuffer = Renderbuffer::new(gl.clone());
                    renderbuffer.storage(
                        glesv2::DEPTH_COMPONENT16,
                        i32::try_from(size.width).unwrap(),
                        i32::try_from(size.height).unwrap(),
                    );
                    RenderbufferAttachment { renderbuffer }
                })]),
            ),
            blur_pass_x: RenderPass::new(gl.clone(), size, "two_pass_gaussian_blur.frag", None),
            blur_pass_y: RenderPass::new(gl.clone(), size, "two_pass_gaussian_blur.frag", None),
            post_pass: RenderPass::new(gl, size, "post.frag", None),
        }
    }
}

pub struct Demo {
    view: Mat4,
    resources: ResourceMapper,
    gl: RcGl,
    rng: XorShiftRng,
    particle_system: ParticleSystem,
    terrain: Terrain,
    resizables: DemoResizables,
}

impl Demo {
    pub fn resolution(&self) -> PhysicalSize<u32> {
        self.resizables.resolution
    }

    pub fn view(&self) -> Mat4 {
        self.view
    }

    pub fn projection(&self) -> Mat4 {
        self.resizables.projection
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.resizables = DemoResizables::new(self.gl.clone(), size);
    }

    pub fn new(resolution: PhysicalSize<u32>, gl: RcGl) -> Result<Self, Error> {
        gl.blend_func(glesv2::SRC_ALPHA, glesv2::ONE_MINUS_SRC_ALPHA);
        gl.enable(glesv2::CULL_FACE);
        gl.depth_func(glesv2::LESS);

        let particle_system = ParticleSystem::new(
            gl.clone(),
            ParticleSpawner::new(
                Vec3::new(0., 2., 0.),
                ParticleSpawnerKind::Box((-5., 0., -5.), (5., 5., 5.)),
                ParticleSpawnerMethod::Once(100000),
            ),
            30.,
            60,
            |pos, time| {
                Vec3::unit_y() * f32::sin(pos.x / 4. + time) * 0.6
                    + (Quat::from_axis_angle(Vec3::unit_y(), f32::atan2(pos.x, pos.z))
                        * Vec3::unit_x()
                        / Vec2::new(pos.x, pos.z).length()
                        * (pos.y + 2.))
                        * (5. - time).max(0.)
            },
        );

        let demo = Demo {
            view: Mat4::zero(),
            resources: ResourceMapper::new(gl.clone(), "resources")?,
            gl: gl.clone(),
            rng: XorShiftRng::seed_from_u64(98341),
            particle_system,
            terrain: Terrain::new(gl.clone(), 200, 200, |x, z| {
                (x * 0.2).sin() * 2. + (z * 0.4).sin() - 2.
            }),
            resizables: DemoResizables::new(gl, resolution),
        };

        log::trace!("demo created");

        Ok(demo)
    }

    pub fn render(&mut self, sync: &mut Sync) -> Result<(), glesv2::Error> {
        self.gl.viewport(
            0,
            0,
            i32::try_from(self.resolution().width).unwrap(),
            i32::try_from(self.resolution().height).unwrap(),
        );

        let cam_pos = Vec3::new(
            sync.get("cam:pos.x"),
            sync.get("cam:pos.y"),
            sync.get("cam:pos.z"),
        );
        self.view = Mat4::look_at_rh(
            cam_pos,
            Vec3::new(
                sync.get("cam:target.x"),
                sync.get("cam:target.y"),
                sync.get("cam:target.z"),
            ), // center
            Vec3::unit_y(),
        );

        self.resizables
            .bloom_pass
            .fbo
            .bind(glesv2::COLOR_BUFFER_BIT | glesv2::DEPTH_BUFFER_BIT);

        // Terrain and particle system ------------------------------------------------------------

        self.gl.enable(glesv2::DEPTH_TEST);
        self.gl.enable(glesv2::BLEND);

        let sim_time = sync.get("sim_time");
        let lightpos = self.particle_system.prepare(cam_pos, sim_time, 128);
        self.terrain.render(&self, lightpos);
        self.particle_system.render(&self);

        self.gl.disable(glesv2::BLEND);
        self.gl.disable(glesv2::DEPTH_TEST);

        // Bloom pass -----------------------------------------------------------------------------

        self.resizables.blur_pass_x.fbo.bind(0);
        self.resizables.bloom_pass.render(&self, &[], &[]);

        // X-blur pass ----------------------------------------------------------------------------

        self.resizables.blur_pass_y.fbo.bind(0);
        self.resizables.blur_pass_x.render(
            &self,
            &[],
            &[("u_BlurDirection", UniformValue::Vec2f(1., 0.))],
        );

        // Y-blur pass ----------------------------------------------------------------------------

        self.resizables.post_pass.fbo.bind(0);
        self.resizables.blur_pass_y.render(
            &self,
            &[],
            &[("u_BlurDirection", UniformValue::Vec2f(0., 1.))],
        );

        // Post pass ------------------------------------------------------------------------------

        // Generate noise
        let noise: Vec<u8> = (0..(self.resolution().width * self.resolution().height
            / NOISE_SCALE.pow(2)))
            .map(|_| self.rng.gen())
            .collect();

        // Upload noise to Texture
        self.resizables.noise_texture.sub_image::<u8>(
            0,
            0,
            0,
            i32::try_from(self.resolution().width / NOISE_SCALE).unwrap(),
            i32::try_from(self.resolution().height / NOISE_SCALE).unwrap(),
            glesv2::LUMINANCE,
            glesv2::UNSIGNED_BYTE,
            noise.as_slice(),
        );

        let noise_amount = UniformValue::Float(sync.get("noise_amount"));

        Framebuffer::bind_default(self.gl.clone(), 0);
        self.resizables.post_pass.render(
            &self,
            &[
                self.resizables
                    .bloom_pass
                    .fbo
                    .texture(glesv2::COLOR_ATTACHMENT0)
                    .unwrap(),
                &self.resizables.noise_texture,
            ],
            &[
                ("u_NoiseAmount", noise_amount),
                ("u_NoiseScale", UniformValue::Float(NOISE_SCALE as f32)),
                ("u_Beat", UniformValue::Float(sync.get_beat())),
            ],
        );

        glesv2::check(self.gl.clone())
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        log::trace!("demo dropped");
    }
}
