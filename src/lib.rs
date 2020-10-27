mod glesv2;
mod particle_system;
mod player;
mod render_pass;
mod terrain;

use cgmath::{Angle, Deg, Euler, InnerSpace, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};
pub use glesv2::{
    types::*, Framebuffer, Gles2, RcGl, Renderbuffer, RenderbufferAttachment, ResourceMapper,
    Texture, UniformValue,
};
use particle_system::{
    ParticleSpawner, ParticleSpawnerKind, ParticleSpawnerMethod, ParticleSystem,
};
pub use player::Player;
use rand::prelude::*;
use rand_xorshift::XorShiftRng;
use render_pass::RenderPass;
use terrain::Terrain;
use thiserror::Error;

const NOISE_SCALE: i32 = 8;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ResourceLoading(#[from] glesv2::resource_mapper::Error),
    #[error(transparent)]
    MusicPlayer(#[from] player::Error),
}

pub struct Demo {
    player: Player,
    pub resolution: (i32, i32),
    pub projection: [f32; 16],
    pub view: [f32; 16],
    pub resources: ResourceMapper,
    pub gl: RcGl,
    rng: XorShiftRng,
    noise_texture: Texture,
    particle_system: ParticleSystem,
    terrain: Terrain,
    bloom_pass: RenderPass,
    blur_pass_x: RenderPass,
    blur_pass_y: RenderPass,
    post_pass: RenderPass,
}

impl Demo {
    pub fn new(w: i32, h: i32, gl: RcGl) -> Result<Self, Error> {
        gl.viewport(0, 0, w, h);
        gl.blend_func(glesv2::SRC_ALPHA, glesv2::ONE_MINUS_SRC_ALPHA);
        gl.enable(glesv2::CULL_FACE);
        gl.depth_func(glesv2::LESS);

        let particle_system = ParticleSystem::new(
            gl.clone(),
            ParticleSpawner::new(
                Vector3::new(0., 2., 0.),
                ParticleSpawnerKind::Box((-5., 0., -5.), (5., 5., 5.)),
                ParticleSpawnerMethod::Once(100000),
            ),
            30.,
            60,
            |pos, time| {
                Vector3::unit_y() * f32::sin(pos.x / 4. + time) * 0.6
                    + (Quaternion::from(Euler {
                        x: Rad(0f32),
                        y: Angle::atan2(pos.x, pos.z),
                        z: Rad(0f32),
                    }) * Vector3::unit_x()
                        / Vector2::new(pos.x, pos.z).magnitude()
                        * (pos.y + 2.))
                        * (5. - time).max(0.)
            },
        );

        let noise_texture = Texture::new(gl.clone(), glesv2::TEXTURE_2D);
        noise_texture.image::<u8>(
            0,
            glesv2::LUMINANCE,
            w / NOISE_SCALE,
            h / NOISE_SCALE,
            glesv2::UNSIGNED_BYTE,
            None,
        );
        noise_texture.parameters(&[
            (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
            (glesv2::TEXTURE_WRAP_S, glesv2::REPEAT),
            (glesv2::TEXTURE_WRAP_T, glesv2::REPEAT),
        ]);

        let demo = Demo {
            player: Player::new("resources/music.ogg")?,
            resolution: (w, h),
            projection: *cgmath::perspective(Deg(60f32), w as f32 / h as f32, 0.1, 1000.).as_ref(),
            view: [0f32; 16],
            resources: ResourceMapper::new(gl.clone(), "resources")?,
            gl: gl.clone(),
            rng: XorShiftRng::seed_from_u64(98341),
            noise_texture,
            particle_system,
            terrain: Terrain::new(gl.clone(), 200, 200, |x, z| {
                (x * 0.2).sin() * 2. + (z * 0.4).sin() - 2.
            }),
            bloom_pass: RenderPass::new(
                gl.clone(),
                w,
                h,
                "bloom.frag",
                Some(vec![(glesv2::DEPTH_ATTACHMENT, {
                    let renderbuffer = Renderbuffer::new(gl.clone());
                    renderbuffer.storage(glesv2::DEPTH_COMPONENT16, w, h);
                    RenderbufferAttachment { renderbuffer }
                })]),
            ),
            blur_pass_x: RenderPass::new(gl.clone(), w, h, "two_pass_gaussian_blur.frag", None),
            blur_pass_y: RenderPass::new(gl.clone(), w, h, "two_pass_gaussian_blur.frag", None),
            post_pass: RenderPass::new(gl, w, h, "post.frag", None),
        };

        log::info!("demo created");

        Ok(demo)
    }

    pub fn render(&mut self) -> Result<(), glesv2::Error> {
        log::info!("{}", self.player.time_secs());
        let cam_pos = Point3::new(
            self.sync_get("cam:pos.x") as f32,
            self.sync_get("cam:pos.y") as f32,
            self.sync_get("cam:pos.z") as f32,
        );
        self.view = *Matrix4::look_at(
            cam_pos,
            Point3::new(
                self.sync_get("cam:target.x") as f32,
                self.sync_get("cam:target.y") as f32,
                self.sync_get("cam:target.z") as f32,
            ), // center
            Vector3::unit_y(),
        )
        .as_ref();

        self.bloom_pass
            .fbo
            .bind(glesv2::COLOR_BUFFER_BIT | glesv2::DEPTH_BUFFER_BIT);

        // Terrain and particle system ------------------------------------------------------------

        self.gl.enable(glesv2::DEPTH_TEST);
        self.gl.enable(glesv2::BLEND);

        let sim_time = self.sync_get("sim_time") as f32;
        let lightpos =
            self.particle_system
                .prepare(cam_pos.to_homogeneous().truncate(), sim_time, 128);
        self.terrain.render(&self, lightpos);
        self.particle_system.render(&self);

        self.gl.disable(glesv2::BLEND);
        self.gl.disable(glesv2::DEPTH_TEST);

        // Bloom pass -----------------------------------------------------------------------------

        self.blur_pass_x.fbo.bind(0);
        self.bloom_pass.render(&self, &[], &[]);

        // X-blur pass ----------------------------------------------------------------------------

        self.blur_pass_y.fbo.bind(0);
        self.blur_pass_x.render(
            &self,
            &[],
            &[("u_BlurDirection", UniformValue::Vec2f(1., 0.))],
        );

        // Y-blur pass ----------------------------------------------------------------------------

        self.post_pass.fbo.bind(0);
        self.blur_pass_y.render(
            &self,
            &[],
            &[("u_BlurDirection", UniformValue::Vec2f(0., 1.))],
        );

        // Post pass ------------------------------------------------------------------------------

        // Generate noise
        let noise: Vec<u8> = (0..(self.resolution.0 * self.resolution.1 / NOISE_SCALE.pow(2)))
            .map(|_| self.rng.gen())
            .collect();

        // Upload noise to Texture
        self.noise_texture.sub_image::<u8>(
            0,
            0,
            0,
            self.resolution.0 / NOISE_SCALE,
            self.resolution.1 / NOISE_SCALE,
            glesv2::LUMINANCE,
            glesv2::UNSIGNED_BYTE,
            noise.as_slice(),
        );

        let noise_amount = UniformValue::Float(self.sync_get("noise_amount") as f32);

        Framebuffer::bind_default(self.gl.clone(), 0);
        self.post_pass.render(
            &self,
            &[
                self.bloom_pass
                    .fbo
                    .texture(glesv2::COLOR_ATTACHMENT0)
                    .unwrap(),
                &self.noise_texture,
            ],
            &[
                ("u_NoiseAmount", noise_amount),
                ("u_NoiseScale", UniformValue::Float(NOISE_SCALE as f32)),
            ],
        );

        glesv2::check(self.gl.clone())
    }

    pub fn start(&self) -> Result<(), Error> {
        self.player.play()?;
        Ok(())
    }

    fn sync_get(&self, _: &str) -> f64 {
        // TODO Rocket impl
        0.1
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        log::info!("demo dropped");
    }
}
