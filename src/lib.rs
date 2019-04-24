use std::os::raw::c_char;

#[no_mangle]
extern "C" fn scene_init(width: i32, height: i32, get: extern "C" fn(*const c_char) -> f64) -> i32 {
    0
}

#[no_mangle]
extern "C" fn scene_deinit() {
}

#[no_mangle]
extern "C" fn scene_render(time: f64) {
}

