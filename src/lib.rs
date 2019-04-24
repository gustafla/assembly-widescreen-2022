use std::os::raw::c_char;
use std::ffi::CString;

struct Scene {
    sync_get_raw: Option<extern "C" fn(*const c_char) -> f64>,
}

static mut SCENE: Scene = Scene { sync_get_raw: None };

#[no_mangle]
extern "C" fn scene_init(width: i32, height: i32, get: extern "C" fn(*const c_char) -> f64) -> i32 {
    unsafe {
        SCENE.sync_get_raw = Some(get);
    }
    0
}

#[no_mangle]
extern "C" fn scene_deinit() {
}

fn sync_get(name: &str) -> f64 {
    let string = CString::new(name).unwrap();
    unsafe {
        SCENE.sync_get_raw.unwrap()(string.as_c_str().as_ptr())
    }
}

#[no_mangle]
extern "C" fn scene_render(time: f64) {
    println!("Value of test is {}", sync_get("test"));
}

