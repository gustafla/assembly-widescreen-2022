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
    pub bitangents: Vec<Vec3>,
}

impl VertexData {
    pub fn from_triangles(positions: Vec<Vec3>, texcoords: Vec<Vec2>) -> Self {
        assert!(positions.len() == texcoords.len());
        assert!(positions.len() % 3 == 0);

        // Compute texcoord-aligned tangents and bitangents
        let mut tangents = Vec::with_capacity(positions.len());
        let mut bitangents = Vec::with_capacity(positions.len());
        for i in (0..positions.len()).step_by(3) {
            let pos0 = positions[i + 0];
            let pos1 = positions[i + 1];
            let pos2 = positions[i + 2];
            let txc0 = texcoords[i + 0];
            let txc1 = texcoords[i + 1];
            let txc2 = texcoords[i + 2];

            let edge0 = pos1 - pos0;
            let edge1 = pos2 - pos0;
            let dtxc0 = txc1 - txc0;
            let dtxc1 = txc2 - txc0;

            let f = 1. / (dtxc0.x * dtxc1.y - dtxc1.x * dtxc0.y);

            let tangent = vec3(
                f * (dtxc1.y * edge0.x - dtxc0.y * edge1.x),
                f * (dtxc1.y * edge0.y - dtxc0.y * edge1.y),
                f * (dtxc1.y * edge0.z - dtxc0.y * edge1.z),
            );
            tangents.extend(std::iter::repeat(tangent).take(3));
            let bitangent = vec3(
                f * (-dtxc1.x * edge0.x + dtxc0.x * edge1.x),
                f * (-dtxc1.x * edge0.y + dtxc0.x * edge1.y),
                f * (-dtxc1.x * edge0.z + dtxc0.x * edge1.z),
            );
            bitangents.extend(std::iter::repeat(bitangent).take(3));
        }

        Self {
            positions,
            texcoords,
            tangents,
            bitangents,
        }
    }

    pub fn push(&mut self, data: VertexData) {
        self.positions.copy_from_slice(&data.positions);
        self.texcoords.copy_from_slice(&data.texcoords);
        self.tangents.copy_from_slice(&data.tangents);
        self.bitangents.copy_from_slice(&data.bitangents);
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
