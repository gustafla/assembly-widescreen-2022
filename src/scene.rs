use glam::Vec3;

pub struct Object {
    tris: Vec<[f32; 3]>,
    color_hsv: (f32, f32, f32),
}

pub struct Camera {
    fov: f32,
    position: Vec3,
    direction: Vec3,
}

pub struct Scene {
    objects: Vec<Object>,
    cameras: Vec<Camera>,
    bg_color_hsv: (f32, f32, f32),
}
