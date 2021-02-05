#[cfg(debug_assertions)]
mod fps_counter;
mod glesv2;
mod particle_system;
mod player;
mod render_pass;
mod sync;
mod terrain;

#[cfg(debug_assertions)]
use fps_counter::FpsCounter;
use glam::{Mat4, Quat, Vec2, Vec3};
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
pub use sync::Sync;
use terrain::Terrain;
use thiserror::Error;

const NOISE_SCALE: i32 = 8;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ResourceLoading(#[from] glesv2::resource_mapper::Error),
}

pub struct Demo {
    #[cfg(debug_assertions)]
    fps_counter: FpsCounter,
    pub resolution: (i32, i32),
    pub projection: Mat4,
    pub view: Mat4,
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
            #[cfg(debug_assertions)]
            fps_counter: FpsCounter::new(),
            resolution: (w, h),
            projection: Mat4::perspective_rh_gl(
                60. * (std::f32::consts::PI / 180.),
                w as f32 / h as f32,
                0.1,
                1000.,
            ),
            view: Mat4::zero(),
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

        log::trace!("demo created");

        Ok(demo)
    }

    pub fn render(&mut self, sync: &mut Sync) -> Result<(), glesv2::Error> {
        #[cfg(debug_assertions)]
        if let Some(fps) = self.fps_counter.tick() {
            log::info!("{} FPS", fps);
        }

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

        self.bloom_pass
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

        let noise_amount = UniformValue::Float(sync.get("noise_amount"));

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
                ("u_FftBass", UniformValue::Float(sync.get_fft(50..250))),
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
