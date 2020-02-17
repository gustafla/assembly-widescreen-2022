#![allow(dead_code)]

mod glesv2_raii;
mod post;

use glesv2_raii::{Buffer, UniformValue};
use log::info;
use opengles::glesv2::{self, constants::*};
use post::Post;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use glesv2_raii::ResourceMapper;

pub struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    resolution: (i32, i32),
    resources: ResourceMapper,
    buffer: Buffer,
    post_pass: Post,
}

impl Scene {
    fn sync_get(&self, name: &str) -> f64 {
        let string = CString::new(name).unwrap();
        (self.sync_get_raw)(string.as_c_str().as_ptr())
    }
}

#[no_mangle]
extern "C" fn scene_init(w: i32, h: i32, get: extern "C" fn(*const c_char) -> f64) -> *mut c_void {
    simple_logger::init().unwrap();
    glesv2::viewport(0, 0, w, h);

    // Create a buffer for test triangle
    let buffer = Buffer::new();
    glesv2::bind_buffer(GL_ARRAY_BUFFER, buffer.handle());
    glesv2::buffer_data(
        GL_ARRAY_BUFFER,
        &[-0.5f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0],
        GL_STATIC_DRAW,
    );

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        resources: ResourceMapper::new().unwrap(),
        buffer,
        post_pass: Post::new(w, h, "post.frag"),
    });

    info!("scene created");

    Box::into_raw(scene) as *mut c_void
}

#[no_mangle]
extern "C" fn scene_deinit(data: *mut c_void) {
    info!("scene dropped");
    let _scene = unsafe { Box::from_raw(data as *mut Scene) };
}

#[no_mangle]
extern "C" fn scene_render(time: f64, data: *mut c_void) {
    let scene = Box::leak(unsafe { Box::from_raw(data as *mut Scene) });

    let program = scene.resources.program("./shader.vert ./shader.frag");

    // Test picture -------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, scene.post_pass.fbo.handle());
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(GL_COLOR_BUFFER_BIT);

    glesv2::bind_buffer(GL_ARRAY_BUFFER, scene.buffer.handle());
    let index_pos = program.attrib_location("a_Pos");
    glesv2::enable_vertex_attrib_array(index_pos);
    glesv2::vertex_attrib_pointer_offset(index_pos, 3, GL_FLOAT, false, 0, 0);

    glesv2::use_program(program.handle());

    glesv2::draw_arrays(GL_TRIANGLES, 0, 3);

    // Post pass ----------------------------------------------------------------------------------

    glesv2::bind_framebuffer(GL_FRAMEBUFFER, 0);
    scene.post_pass.render(
        &scene.resources,
        &[],
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

    glesv2_raii::check().unwrap();
}
