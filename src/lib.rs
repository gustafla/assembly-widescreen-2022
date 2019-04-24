use std::os::raw::c_char;

struct Scene {
    sync_get: Option<extern "C" fn(*const c_char) -> f64>,
}

static mut SCENE: Scene = Scene { sync_get: None };

#[no_mangle]
extern "C" fn scene_init(width: i32, height: i32, get: extern "C" fn(*const c_char) -> f64) -> i32 {
    unsafe {
        SCENE.sync_get = Some(get);
    }
    0
}

#[no_mangle]
extern "C" fn scene_deinit() {
}

#[no_mangle]
extern "C" fn scene_render(time: f64) {
    println!("Scene render called");
}

