mod player;
mod renderer;
pub mod scene;
mod sync;

use glam::*;
use include_dir::{include_dir, Dir};
pub use player::Player;
pub use renderer::Renderer;
use scene::{Camera, Model, Object, Scene, VertexData};
pub use sync::DemoSync;

use crate::scene::{Srgbu8, Texture};

pub static RESOURCES_PATH: &str = "resources";
pub static RESOURCES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

fn cylinder_position(r: f32, u: f32, v: f32) -> Vec3 {
    let u = u * std::f32::consts::TAU;
    let x = r * u.cos();
    let y = v;
    let z = r * u.sin();
    vec3(x, y, z)
}

fn trunk_segment(r0: f32, r1: f32, start: f32, end: f32, n: usize) -> VertexData {
    let mut positions = Vec::new();
    let mut texcoords = Vec::new();
    for i in 0..n - 1 {
        let u0 = i as f32 / (n - 1) as f32;
        let v0 = start;
        let u1 = (i + 1) as f32 / (n - 1) as f32;
        let v1 = end;

        let p0 = cylinder_position(r0, u0, v0);
        let p1 = cylinder_position(r0, u1, v0);
        let p2 = cylinder_position(r1, u1, v1);
        let p3 = cylinder_position(r1, u0, v1);

        positions.push(p0);
        positions.push(p1);
        positions.push(p2);
        positions.push(p2);
        positions.push(p3);
        positions.push(p0);

        texcoords.push(vec2(u0, v0));
        texcoords.push(vec2(u1, v0));
        texcoords.push(vec2(u1, v1));
        texcoords.push(vec2(u1, v1));
        texcoords.push(vec2(u0, v1));
        texcoords.push(vec2(u0, v0));
    }

    VertexData::from_triangles(positions, texcoords)
}

fn generate_tree() -> VertexData {
    let mut vertices = VertexData::default();

    vertices.push(trunk_segment(1., 0.5, 0., 1., 8));
    vertices.push(trunk_segment(0.5, 0.1, 1., 2., 8));
    vertices.push(trunk_segment(0.1, 0.0, 2., 2.1, 8));

    vertices
}

pub fn init() -> Vec<Model> {
    let mut models = Vec::new();

    models.push(Model {
        vertices: generate_tree(),
        albedo: Texture {
            width: 1,
            height: 1,
            data: vec![Srgbu8([255, 255, 255])],
        },
        normal: Texture {
            width: 1,
            height: 1,
            data: vec![vec3(0., 0., 1.)],
        },
        roughness: Texture {
            width: 1,
            height: 1,
            data: vec![0.],
        },
        ao: Texture {
            width: 1,
            height: 1,
            data: vec![0.],
        },
    });

    log::trace!("Models initialized");

    models
}

pub fn update(sync: &mut DemoSync) -> Scene {
    Scene {
        objects: vec![Object {
            model: 0,
            scale: Vec3::ONE,
            rotation: Quat::from_rotation_x(sync.get("rotation.x")),
            translation: Vec3::ZERO,
        }],
        cameras: vec![Camera {
            fov: sync.get("camera0:fov"),
            position: vec3(
                sync.get("camera0:pos.x"),
                sync.get("camera0:pos.y"),
                sync.get("camera0:pos.z"),
            ),
            target: vec3(
                sync.get("camera0:target.x"),
                sync.get("camera0:target.y"),
                sync.get("camera0:target.z"),
            ),
        }],
    }
}
