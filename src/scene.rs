use color_space::Hsv;
use glam::*;

#[derive(Default)]
pub struct VertexData {
    pub positions: Vec<Vec3>,
    pub colors: Vec<Hsv>,
    pub roughness: Vec<f32>,
    pub normals: Vec<Vec3>,
}

impl VertexData {
    pub fn from_triangles(positions: Vec<Vec3>, colors: Vec<Hsv>, roughness: Vec<f32>) -> Self {
        assert!(positions.len() == colors.len());
        assert!(positions.len() == roughness.len());
        assert!(positions.len() % 3 == 0);

        // Compute texcoord-aligned tangents and bitangents
        /*let mut tangents = Vec::with_capacity(positions.len());
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

            let tangent = ((dtxc1.y * edge0 - dtxc0.y * edge1) * f).normalize();
            tangents.extend(std::iter::repeat(tangent).take(3));
            let bitangent = ((dtxc0.x * edge1 - dtxc1.x * edge0) * f).normalize();
            bitangents.extend(std::iter::repeat(bitangent).take(3));
        }*/
        let mut normals = Vec::with_capacity(positions.len());
        for i in (0..positions.len()).step_by(3) {
            let pos0 = positions[i];
            let pos1 = positions[i + 1];
            let pos2 = positions[i + 2];

            let edge0 = pos1 - pos0;
            let edge1 = pos2 - pos0;
            let normal = (edge0.cross(edge1)).normalize();

            normals.extend(std::iter::repeat(normal).take(3));
        }

        Self {
            positions,
            colors,
            roughness,
            normals,
        }
    }

    pub fn push(&mut self, data: VertexData) {
        self.positions.extend_from_slice(&data.positions);
        self.colors.extend_from_slice(&data.colors);
        self.roughness.extend_from_slice(&data.roughness);
        self.normals.extend_from_slice(&data.normals);
    }

    pub fn transform(mut self, transformation: Mat4) -> Self {
        for x in self.positions.iter_mut() {
            *x = transformation.transform_point3(*x);
        }
        let normal_transformation = transformation.inverse().transpose();
        for x in self.normals.iter_mut() {
            *x = normal_transformation.transform_vector3(*x);
        }
        self
    }
}

pub struct Model {
    pub vertices: VertexData,
}

#[derive(Clone)]
pub struct Instance {
    pub scale: Vec3,
    pub rotation: Quat,
    pub translation: Vec3,
}

#[derive(Clone, Default)]
pub struct Light {
    pub coordinates: Vec4,
    pub color: Hsv,
}

pub enum CameraView {
    Target(Vec3),
    PitchYawRoll(Vec3),
}

pub struct Camera {
    pub fov: f32,
    pub position: Vec3,
    pub view: CameraView,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 1.,
            position: vec3(0., 10., 10.),
            view: CameraView::Target(Vec3::ZERO),
        }
    }
}

pub struct Scene {
    pub instances_by_model: Vec<Vec<Instance>>,
    pub ambient: f32,
    pub lights: Vec<Light>,
    pub camera: Camera,
}
