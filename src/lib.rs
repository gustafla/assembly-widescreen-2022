#![allow(dead_code)]

mod gles2_buffer;
mod gles2_error;
mod gles2_fbo;
mod gles2_shader;

use gles2_buffer::{ArrayBuffer, Buffer};
use gles2_fbo::{Fbo, FboBuilder};
use opengles::glesv2::{self, GLint, GLuint};
use std::ffi::{c_void, CString};
use std::os::raw::c_char;

struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    resolution: (i32, i32),
    program: glesv2::GLuint,
    buffer: ArrayBuffer,
    post_fbo: Fbo,
    post_program: glesv2::GLuint,
    post_buffer: ArrayBuffer,
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

    let post_buffer = ArrayBuffer::new().static_data(&[
        -1f32, -1., 0., 0., 0., 1., -1., 0., 1., 0., 1., 1., 0., 1., 1., -1., -1., 0., 0., 0., 1.,
        1., 0., 1., 1., -1., 1., 0., 0., 1.,
    ]);

    let buffer =
        ArrayBuffer::new().static_data(&[-0.5f32, -0.5, 0.0, 0.5, -0.5, 0.0, 0.0, 0.5, 0.0]);

    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
        program: gles2_shader::link_program(&["shader.vert", "shader.frag"]).unwrap(),
        buffer,
        post_fbo: FboBuilder::new()
            .add_texture2d(glesv2::GL_RGB, (w, h), glesv2::GL_COLOR_ATTACHMENT0)
            .unwrap()
            .build()
            .unwrap(),
        post_program: gles2_shader::link_program(&["shader.vert", "post.frag"]).unwrap(),
        post_buffer,
    });

    eprintln!("scene created");

    Box::into_raw(scene) as *mut c_void
}

#[no_mangle]
extern "C" fn scene_deinit(data: *mut c_void) {
    let scene = unsafe { Box::from_raw(data as *mut Scene) };
    glesv2::delete_program(scene.program);
    glesv2::delete_program(scene.post_program);
    eprintln!("scene deinit finished");
}

#[no_mangle]
extern "C" fn scene_render(time: f64, data: *mut c_void) {
    let scene = Box::leak(unsafe { Box::from_raw(data as *mut Scene) });

    // Test picture -------------------------------------------------------------------------------

    scene.buffer.bind();
    glesv2::use_program(scene.program);

    scene.post_fbo.bind();
    glesv2::clear_color(f32::sin(time as f32), 1., 0., 1.);
    glesv2::clear(glesv2::GL_COLOR_BUFFER_BIT);

    let index_pos = glesv2::get_attrib_location(scene.program, "a_Pos") as GLuint;
    glesv2::enable_vertex_attrib_array(index_pos);
    glesv2::vertex_attrib_pointer_offset(index_pos, 3, glesv2::GL_FLOAT, false, 0, 0);

    glesv2::draw_arrays(glesv2::GL_TRIANGLES, 0, 3);

    // Post pass ----------------------------------------------------------------------------------

    scene.post_buffer.bind();
    glesv2::use_program(scene.post_program);

    Fbo::bind_default();
    scene
        .post_fbo
        .bind_attachment(glesv2::GL_COLOR_ATTACHMENT0)
        .unwrap();
    glesv2::uniform1i(
        glesv2::get_uniform_location(scene.post_program, "u_InputSampler"),
        0,
    );

    let index_pos = glesv2::get_attrib_location(scene.post_program, "a_Pos") as GLuint;
    let index_tex_coord = glesv2::get_attrib_location(scene.post_program, "a_TexCoord") as GLuint;
    let stride = (std::mem::size_of::<f32>() * 5) as GLint;
    glesv2::enable_vertex_attrib_array(index_pos);
    glesv2::vertex_attrib_pointer_offset(index_pos, 3, glesv2::GL_FLOAT, false, stride, 0);
    glesv2::enable_vertex_attrib_array(index_tex_coord);
    glesv2::vertex_attrib_pointer_offset(
        index_tex_coord,
        2,
        glesv2::GL_FLOAT,
        false,
        stride,
        std::mem::size_of::<f32>() as GLuint * 3,
    );

    glesv2::draw_arrays(glesv2::GL_TRIANGLES, 0, 6);
    glesv2::bind_buffer(glesv2::GL_ARRAY_BUFFER, 0);

    gles2_error::check().unwrap();
}
