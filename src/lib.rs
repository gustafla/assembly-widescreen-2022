#![allow(dead_code)]

mod glesv2_raii;
mod post;

use glesv2_raii::ResourceMapper;
use glesv2_raii::{Buffer, UniformValue};
use opengles::glesv2::{self, constants::*};
use post::Post;
use std::ffi::CString;
use std::os::raw::c_char;

pub struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    resolution: (i32, i32),
    resources: ResourceMapper,
    buffer: Buffer,
    bloom_pass: Post,
    blur_pass_x: Post,
    blur_pass_y: Post,
    post_pass: Post,
}

impl Scene {
    fn sync_get(&self, name: &str) -> f64 {
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

    // Create a buffer for test triangle
    let buffer = Buffer::new(GL_ARRAY_BUFFER);
    buffer.bind();
    glesv2::buffer_data(
        GL_ARRAY_BUFFER,
        &[-0.5f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0],
        GL_STATIC_DRAW,
    );

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        resources: ResourceMapper::new().unwrap_or_else(|e| log_and_panic(e)),
        buffer,
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
    let scene = Box::leak(scene);

    let program = scene
        .resources
        .program("./shader.vert ./shader.frag")
        .unwrap();

    // Test picture -------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.bloom_pass.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    scene.buffer.bind();
    let index_pos = program.attrib_location("a_Pos");
    glesv2::enable_vertex_attrib_array(index_pos);
    glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, 0, 0);

    glesv2::use_program(program.handle());

    glesv2::draw_arrays(GL_TRIANGLES, 0, 3);

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
            (
                "u_NoiseTime",
                UniformValue::Float(scene.sync_get("noise_time") as f32),
            ),
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
