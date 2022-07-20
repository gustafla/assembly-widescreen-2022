use glam::*;

#[derive(Default)]
pub struct VertexData {
    pub positions: Vec<Vec3>,
    pub color_roughness: Vec<Vec4>,
    pub normals: Vec<Vec3>,
}

impl VertexData {
    pub fn from_triangles(positions: Vec<Vec3>, color_roughness: Vec<Vec4>) -> Self {
        assert!(positions.len() == color_roughness.len());
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
            let pos0 = positions[i + 0];
            let pos1 = positions[i + 1];
            let pos2 = positions[i + 2];

            let edge0 = pos1 - pos0;
            let edge1 = pos2 - pos0;
            let normal = (edge0.cross(edge1)).normalize();

            normals.extend(std::iter::repeat(normal).take(3));
        }

        Self {
            positions,
            color_roughness,
            normals,
        }
    }

    pub fn push(&mut self, data: VertexData) {
        self.positions.extend_from_slice(&data.positions);
        self.color_roughness
            .extend_from_slice(&data.color_roughness);
        self.normals.extend_from_slice(&data.normals);
    }
}

pub struct Model {
    pub vertices: VertexData,
}

pub struct Instance {
    pub scale: Vec3,
    pub rotation: Quat,
    pub translation: Vec3,
}

pub struct Camera {
    pub fov: f32,
    pub position: Vec3,
    pub target: Vec3,
}

pub struct Scene<const M: usize> {
    pub instances_by_model: [Vec<Instance>; M],
    pub camera: Camera,
}
