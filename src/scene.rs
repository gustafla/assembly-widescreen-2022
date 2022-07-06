use glam::Vec3;

pub struct Object {
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    color_hsv: (f32, f32, f32),
    translation: Vec3,
    scale: Vec3,
}

pub struct Camera {
    fov: f32,
    position: Vec3,
    target: Vec3,
}

pub struct Scene {
    objects: Vec<Object>,
    cameras: Vec<Camera>,
    bg_color_hsv: (f32, f32, f32),
}
