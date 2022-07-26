mod player;
mod renderer;
pub mod scene;
mod sync;

use color_space::Hsv;
use glam::*;
use include_dir::{include_dir, Dir};
pub use player::Player;
use rand::prelude::*;
pub use renderer::Renderer;
use scene::{Camera, Instance, Light, Model, Scene, VertexData};
pub use sync::DemoSync;

pub static RESOURCES_PATH: &str = "resources";
pub static RESOURCES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

fn cylinder_position(r: f32, u: f32, v: f32) -> Vec3 {
    let u = u * std::f32::consts::TAU;
    let x = r * u.cos();
    let y = v;
    let z = -r * u.sin();
    vec3(x, y, z)
}

fn trunk_segment(r0: f32, r1: f32, start: f32, end: f32, n: usize) -> VertexData {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut roughness = Vec::new();
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

        colors.extend(std::iter::repeat(Hsv::new(0., 0., 1.)).take(6));
        roughness.extend(std::iter::repeat(1.).take(6));

        /*texcoords.push(vec2(u0, v0));
        texcoords.push(vec2(u1, v0));
        texcoords.push(vec2(u1, v1));
        texcoords.push(vec2(u1, v1));
        texcoords.push(vec2(u0, v1));
        texcoords.push(vec2(u0, v0));*/
    }

    VertexData::from_triangles(positions, colors, roughness)
}

fn generate_tree(rng: &mut impl Rng, nu: usize, nv: usize) -> VertexData {
    let mut vertices = VertexData::default();

    for v in 0..nv {
        let v0 = v as f32;
        let v1 = (v + 1) as f32;
        vertices.push(trunk_segment(1. / v0, 1. / v1, v0, v1, nu));
    }

    vertices
}

fn generate_plane() -> VertexData {
    VertexData::from_triangles(
        vec![
            vec3(-0.5, 0., 0.5),
            vec3(0.5, 0., 0.5),
            vec3(0.5, 0., -0.5),
            vec3(0.5, 0., -0.5),
            vec3(-0.5, 0., -0.5),
            vec3(-0.5, 0., 0.5),
        ],
        std::iter::repeat(Hsv::new(0., 0., 1.)).take(6).collect(),
        std::iter::repeat(1.).take(6).collect(),
    )
}

const MODELS: usize = 2;

pub fn init(rng: &mut impl Rng) -> [Model; MODELS] {
    let models = [
        Model {
            vertices: generate_tree(rng, 8, 15),
        },
        Model {
            vertices: generate_plane(),
        },
    ];

    log::trace!("Models initialized");

    models
}

pub fn update(sync: &mut DemoSync) -> Scene<MODELS> {
    let mut test = Vec::new();
    for i in -10i32..=10 {
        let i = i as f32 / 10.;
        test.push(Instance {
            scale: Vec3::ONE * (2. + i),
            rotation: Quat::from_axis_angle(Vec3::Y, sync.get("rotation.y")),
            translation: vec3(i * 60., (i + sync.get("rotation.y")).sin() * 3., 0.),
        });
    }

    Scene {
        instances_by_model: [
            test,
            vec![Instance {
                scale: Vec3::ONE * 60.,
                rotation: Quat::IDENTITY,
                translation: vec3(0., 0., 0.),
            }],
        ],
        camera: Camera {
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
        },
        lights: vec![
            Light {
                coordinates: vec4(
                    sync.get("light:point.x"),
                    sync.get("light:point.y"),
                    sync.get("light:point.z"),
                    1.,
                ),
                color: Hsv::new(
                    sync.get("light:point.hue") as f64,
                    sync.get("light:point.sat") as f64,
                    sync.get("light:point.val") as f64,
                ),
            },
            Light {
                coordinates: vec4(0.1, -1., -0.1, 0.),
                color: Hsv::new(0., 0., 0.8),
            },
        ],
    }
}
