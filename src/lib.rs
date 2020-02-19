#![allow(dead_code)]

mod glesv2_raii;
mod particle_system;
mod post;

use cgmath::{Deg, Matrix4, Point3, Vector3};
use glesv2_raii::ResourceMapper;
use glesv2_raii::UniformValue;
use opengles::glesv2::{self, constants::*};
use particle_system::ParticleSystem;
use post::Post;
use std::ffi::CString;
use std::os::raw::c_char;

pub struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    pub resolution: (i32, i32),
    pub projection: [f32; 16],
    pub view: [f32; 16],
    pub resources: ResourceMapper,
    particle_system: ParticleSystem,
    bloom_pass: Post,
    blur_pass_x: Post,
    blur_pass_y: Post,
    post_pass: Post,
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

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        projection: *cgmath::perspective(Deg(60f32), w as f32 / h as f32, 0.1, 1000.).as_ref(),
        view: [0f32; 16],
        resources: ResourceMapper::new().unwrap_or_else(|e| log_and_panic(e)),
        particle_system: ParticleSystem::new(1. / 60., 1000),
        bloom_pass: Post::new(w, h, "./bloom.frag"),
        blur_pass_x: Post::new(w, h, "./blurx.frag"),
        blur_pass_y: Post::new(w, h, "./blury.frag"),
        post_pass: Post::new(w, h, "./post.frag"),
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
        Point3::new(time.sin() as f32 * 3., 0., time.cos() as f32 * 3.), // eye
        Point3::new(0., 0., 0.),                                         // center
        Vector3::unit_y(),
    )
    .as_ref();

    // Particle system ----------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.bloom_pass.fbo.handle());
    glesv2::clear_color(0.2, 0.2, 0.2, 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.particle_system.render(&scene, time as f32);

    // Bloom pass ---------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.blur_pass_x.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.bloom_pass.render(&scene.resources, &[], &[]);

    // X-blur pass --------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.blur_pass_y.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.blur_pass_x.render(
        &scene.resources,
        &[],
        &[(
            "u_Resolution",
            UniformValue::Vec2(scene.resolution.0 as f32, scene.resolution.1 as f32),
        )],
    );

    // Y-blur pass --------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.post_pass.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.blur_pass_y.render(
        &scene.resources,
        &[],
        &[(
            "u_Resolution",
            UniformValue::Vec2(scene.resolution.0 as f32, scene.resolution.1 as f32),
        )],
    );

    // Post pass ----------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, 0);
    scene.post_pass.render(
        &scene.resources,
        &[scene
            .bloom_pass
            .fbo
            .texture_handle(GL_COLOR_ATTACHMENT0)
            .unwrap()],
        &[
            ("u_NoiseTime", UniformValue::Float(time as f32)),
            (
                "u_NoiseAmount",
                UniformValue::Float(scene.sync_get("noise_amount") as f32),
            ),
            (
                "u_Resolution",
                UniformValue::Vec2(scene.resolution.0 as f32, scene.resolution.1 as f32),
            ),
        ],
    );

    glesv2_raii::check().unwrap_or_else(|e| log_and_panic(Box::new(e)));
}
