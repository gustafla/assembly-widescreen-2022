#![allow(dead_code)]

mod gles2_error;
mod gles2_shader;
mod gles2_fbo;

use opengles::glesv2;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;

struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    resolution: (i32, i32),
    program: glesv2::GLuint,
}

impl Scene {
    fn sync_get(&self, name: &str) -> f64 {
        let string = CString::new(name).unwrap();
        (self.sync_get_raw)(string.as_c_str().as_ptr())
    }
}

#[no_mangle]
extern "C" fn scene_init(w: i32, h: i32, get: extern "C" fn(*const c_char) -> f64) -> *mut c_void {
    glesv2::viewport(0, 0, w, h);

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        program: gles2_shader::link_program(&["shader.vert", "shader.frag"]).unwrap(),
    });

    Box::into_raw(scene) as *mut c_void
}

#[no_mangle]
extern "C" fn scene_deinit(data: *mut c_void) {
    let scene = unsafe { Box::from_raw(data as *mut Scene) };
    glesv2::delete_program(scene.program);
}

#[no_mangle]
extern "C" fn scene_render(time: f64, data: *mut c_void) {
    let scene = Box::leak(unsafe { Box::from_raw(data as *mut Scene) });

    glesv2::clear_color(f64::sin(time) as f32, 1., 0., 1.);
    glesv2::clear(glesv2::GL_COLOR_BUFFER_BIT);

    let vertices = [-0.5f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0];

    /*let buffer = glesv2::gen_buffers(1)[0];
    glesv2::bind_buffer(glesv2::GL_ARRAY_BUFFER, buffer);
    glesv2::buffer_data(glesv2::GL_ARRAY_BUFFER, &vertices, glesv2::GL_STATIC_DRAW);*/

    glesv2::use_program(scene.program);

    glesv2::vertex_attrib_pointer(0, 3, glesv2::GL_FLOAT, false, 0, &vertices);
    glesv2::enable_vertex_attrib_array(0);

    glesv2::draw_arrays(glesv2::GL_TRIANGLES, 0, 3);

    gles2_error::check().unwrap();
}
