use glam::*;

pub struct Object {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub color_hsv: (f32, f32, f32), // Not implemented
    pub scale: Vec3,
    pub rotation: Quat,
    pub translation: Vec3,
}

pub struct Camera {
    pub fov: f32,
    pub position: Vec3,
    pub target: Vec3,
}

pub struct Scene {
    pub objects: Vec<Object>,          // Only one supported in Renderer
    pub cameras: Vec<Camera>,          // Only one supported in Renderer
    pub bg_color_hsv: (f32, f32, f32), // Not implemented
}
