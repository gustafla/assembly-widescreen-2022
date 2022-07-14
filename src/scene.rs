use bytemuck::{Pod, Zeroable};
use glam::*;

pub trait Texel: Pod + Zeroable {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Srgbu8(pub [u8; 3]);
impl Texel for Srgbu8 {}
impl Texel for Vec3 {}
impl Texel for f32 {}

pub struct Texture<F: Texel> {
    pub width: u32,
    pub height: u32,
    pub data: Vec<F>,
}

#[derive(Default)]
pub struct VertexData {
    pub positions: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub tangents: Vec<Vec3>,
    pub normals: Vec<Vec3>,
}

impl VertexData {
    pub fn push(&mut self, data: VertexData) {
        self.positions.copy_from_slice(&data.positions);
        self.normals.copy_from_slice(&data.normals);
        self.tangents.copy_from_slice(&data.tangents);
        self.texcoords.copy_from_slice(&data.texcoords);
    }
}

pub struct Model {
    pub vertices: VertexData,
    pub albedo: Texture<Srgbu8>,
    pub normal: Texture<Vec3>,
    pub roughness: Texture<f32>,
    pub ao: Texture<f32>,
}

pub struct Object {
    pub model: usize,
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
    pub objects: Vec<Object>, // Only one supported in Renderer
    pub cameras: Vec<Camera>, // Only one supported in Renderer
}
