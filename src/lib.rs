#![allow(dead_code)]

mod glesv2_raii;
mod particle_system;
mod render_pass;

use cgmath::{Angle, Deg, Euler, InnerSpace, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};
use glesv2_raii::{ResourceMapper, Texture, UniformValue};
use opengles::glesv2::{self, constants::*, types::*};
use particle_system::{
    ParticleSpawner, ParticleSpawnerKind, ParticleSpawnerMethod, ParticleSystem,
};
use rand::prelude::*;
use rand_xorshift::XorShiftRng;
use render_pass::RenderPass;
use std::ffi::CString;
use std::os::raw::c_char;

const NOISE_SCALE: i32 = 8;

pub struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    pub resolution: (i32, i32),
    pub projection: [f32; 16],
    pub view: [f32; 16],
    pub resources: ResourceMapper,
    rng: XorShiftRng,
    particle_system: ParticleSystem,
    noise_texture: Texture,
    bloom_pass: RenderPass,
    blur_pass_x: RenderPass,
    blur_pass_y: RenderPass,
    post_pass: RenderPass,
}

impl Scene {
    pub fn sync_get(&self, name: &str) -> f64 {
        let string = CString::new(name).unwrap();
        (self.sync_get_raw)(string.as_c_str().as_ptr())
    }
}

fn log_and_panic(error: Box<dyn std::error::Error>) -> ! {
    log::error!("{}", error);
    let mut source = error.source();
    while let Some(e) = source {
        log::error!("Caused by {}", e);
        source = e.source();
    }
    panic!();
}

#[no_mangle]
extern "C" fn scene_init(w: i32, h: i32, get: extern "C" fn(*const c_char) -> f64) -> Box<Scene> {
    simple_logger::init().unwrap_or_else(|e| panic!("Failed to initialize logger\n{}", e));
    glesv2::viewport(0, 0, w, h);

    let particle_system = ParticleSystem::new(
        ParticleSpawner::new(
            ParticleSpawnerKind::Box((-5., -5., -5.), (5., 5., 5.)),
            ParticleSpawnerMethod::Once(1000000),
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

    let noise_texture = Texture::new();
    glesv2::bind_texture(GL_TEXTURE_2D, noise_texture.handle());
    Texture::image::<u8>(
        GL_TEXTURE_2D,
        0,
        GL_LUMINANCE,
        w / NOISE_SCALE,
        h / NOISE_SCALE,
        GL_UNSIGNED_BYTE,
        &[],
    );
    glesv2::tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST as GLint);
    glesv2::tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST as GLint);
    glesv2::tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_REPEAT as GLint);
    glesv2::tex_parameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_REPEAT as GLint);

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        projection: *cgmath::perspective(Deg(60f32), w as f32 / h as f32, 0.1, 1000.).as_ref(),
        view: [0f32; 16],
        resources: ResourceMapper::new().unwrap_or_else(|e| log_and_panic(e)),
        rng: XorShiftRng::seed_from_u64(98341),
        particle_system,
        noise_texture,
        bloom_pass: RenderPass::new(w, h, "./bloom.frag"),
        blur_pass_x: RenderPass::new(w, h, "./two_pass_gaussian_blur.frag"),
        blur_pass_y: RenderPass::new(w, h, "./two_pass_gaussian_blur.frag"),
        post_pass: RenderPass::new(w, h, "./post.frag"),
    });

    log::info!("scene created");

    scene
}

#[no_mangle]
extern "C" fn scene_deinit(_: Box<Scene>) {
    log::info!("scene dropped");
}

#[no_mangle]
extern "C" fn scene_render(time: f64, scene: Box<Scene>) {
    let mut scene = Box::leak(scene);

    scene.view = *Matrix4::look_at(
        Point3::new(
            scene.sync_get("cam:pos.x") as f32,
            scene.sync_get("cam:pos.y") as f32,
            scene.sync_get("cam:pos.z") as f32,
        ), // eye
        Point3::new(
            scene.sync_get("cam:target.x") as f32,
            scene.sync_get("cam:target.y") as f32,
            scene.sync_get("cam:target.z") as f32,
        ), // center
        Vector3::unit_y(),
    )
    .as_ref();

    glesv2::enable(GL_BLEND);
    glesv2::blend_func(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

    // Particle system ----------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.bloom_pass.fbo.handle());
    glesv2::clear_color(0., 0., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene
        .particle_system
        .render(&scene, scene.sync_get("sim_time") as f32);

    // Bloom pass ---------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.blur_pass_x.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.bloom_pass.render(&scene, &[], &[]);

    // X-blur pass --------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.blur_pass_y.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.blur_pass_x.render(
        &scene,
        &[],
        &[("u_BlurDirection", UniformValue::Vec2(1., 0.))],
    );

    // Y-blur pass --------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.post_pass.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.blur_pass_y.render(
        &scene,
        &[],
        &[("u_BlurDirection", UniformValue::Vec2(0., 1.))],
    );

    // Post pass ----------------------------------------------------------------------------------

    // Generate noise
    let noise: Vec<u8> = (0..(scene.resolution.0 * scene.resolution.1 / NOISE_SCALE.pow(2)))
        .map(|_| scene.rng.gen())
        .collect();

    // Upload noise to Texture
    glesv2::bind_texture(GL_TEXTURE_2D, scene.noise_texture.handle());
    Texture::sub_image::<u8>(
        GL_TEXTURE_2D,
        0,
        0,
        0,
        scene.resolution.0 / NOISE_SCALE,
        scene.resolution.1 / NOISE_SCALE,
        GL_LUMINANCE,
        GL_UNSIGNED_BYTE,
        noise.as_slice(),
    );

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, 0);
    scene.post_pass.render(
        &scene,
        &[
            scene
                .bloom_pass
                .fbo
                .texture_handle(GL_COLOR_ATTACHMENT0)
                .unwrap(),
            scene.noise_texture.handle(),
        ],
        &[
            (
                "u_NoiseAmount",
                UniformValue::Float(scene.sync_get("noise_amount") as f32),
            ),
            ("u_NoiseScale", UniformValue::Float(NOISE_SCALE as f32)),
        ],
    );

    glesv2_raii::check().unwrap_or_else(|e| log_and_panic(Box::new(e)));
}
