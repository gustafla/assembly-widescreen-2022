use std::os::raw::c_char;
use std::ffi::{CString, c_void};

struct Scene {
    sync_get_raw: extern "C" fn(*const c_char) -> f64,
    resolution: (i32, i32),
}

impl Scene {
    fn sync_get(&self, name: &str) -> f64 {
        let string = CString::new(name).unwrap();
        (self.sync_get_raw)(string.as_c_str().as_ptr())
    }
}

#[no_mangle]
extern "C" fn scene_init(w: i32, h: i32, get: extern "C" fn(*const c_char) -> f64) -> *mut c_void {
    let scene = Box::new(Scene {
        sync_get_raw: get,
        resolution: (w, h),
    });

    Box::into_raw(scene) as *mut c_void
}

#[no_mangle]
extern "C" fn scene_deinit(data: *mut c_void) {
    let _scene = unsafe {Box::from_raw(data as *mut Scene)};
}

#[no_mangle]
extern "C" fn scene_render(time: f64, data: *mut c_void) {
    use opengles::glesv2;

    let scene = Box::leak(unsafe {Box::from_raw(data as *mut Scene)});
    glesv2::clear_color(1., 0., 0., 1.);
    glesv2::clear(glesv2::GL_COLOR_BUFFER_BIT);
}

