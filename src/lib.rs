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

fn random_vec3(rng: &mut impl Rng) -> Vec3 {
    vec3(rng.gen(), rng.gen(), rng.gen()) * 2. - 1.
}

fn generate_leaf(color: Hsv, roughness: f32) -> VertexData {
    VertexData::from_triangles(
        vec![
            vec3(-0.5, -0.5, 0.),
            vec3(0.5, -0.5, 0.),
            vec3(0., 0.5, 0.),
            vec3(0.5, -0.5, 0.),
            vec3(-0.5, -0.5, 0.),
            vec3(0., 0.5, 0.),
        ],
        std::iter::repeat(color).take(6).collect(),
        std::iter::repeat(roughness).take(6).collect(),
    )
}

fn generate_trunk_segment(r0: f32, r1: f32, start: Vec3, end: Vec3, n: usize) -> VertexData {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut roughness = Vec::new();
    for i in 0..n - 1 {
        let u0 = i as f32 / (n - 1) as f32;
        let u1 = (i + 1) as f32 / (n - 1) as f32;

        let p0 = cylinder_position(r0, u0, 0.) + start;
        let p1 = cylinder_position(r0, u1, 0.) + start;
        let p2 = cylinder_position(r1, u1, 0.) + end;
        let p3 = cylinder_position(r1, u0, 0.) + end;

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

fn generate_tree(
    rng: &mut impl Rng,
    nu: usize,
    nv: usize,
    thickness: f32,
    start: Vec3,
    direction: Vec3,
    mut branches: usize,
    leaves_per_segment: usize,
    light_scale: f64,
) -> VertexData {
    let mut vertices = VertexData::default();

    let mut segment = start + random_vec3(rng) * 0.1;
    let mut thicc = thickness;
    for i in 0..nv {
        let thicc2 = thicc - (thickness / nv as f32);
        let segment2 = segment + random_vec3(rng) * 0.1 + direction;
        vertices.push(generate_trunk_segment(thicc, thicc2, segment, segment2, nu));

        // Leaves
        let tip = i as f32 / nv as f32;
        let leaves = (leaves_per_segment as f32 * (tip - 0.5).max(0.) * 2.) as usize;
        for _ in 0..leaves {
            let position = segment + random_vec3(rng) * tip * 4.;
            let transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(0.5),
                Quat::from_axis_angle(random_vec3(rng), rng.gen::<f32>() * std::f32::consts::PI),
                position,
            );
            let deep = position.length() as f64 / light_scale;
            vertices.push(
                generate_leaf(Hsv::new(100. - deep * 40., 0.55, deep * 0.6), 0.3)
                    .transform(transform),
            );
        }

        // Branch
        if branches > 0 && rng.gen::<f32>() < tip {
            let direction = (direction + random_vec3(rng) + Vec3::Y * 0.2).normalize();
            branches -= 1;
            let nv = nv - rng.gen_range(i..nv) + rng.gen_range(0..3);
            vertices.push(generate_tree(
                rng,
                nu,
                nv,
                thicc / 1.2,
                segment,
                direction,
                branches,
                leaves_per_segment,
                light_scale,
            ));
        }

        segment = segment2;
        thicc = thicc2;
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

pub fn init(rng: &mut impl Rng) -> Vec<Model> {
    let mut models = vec![Model {
        vertices: generate_plane(),
    }];

    for _ in 0..10 {
        models.push(Model {
            vertices: generate_tree(rng, 6, 20, 1., Vec3::ZERO, Vec3::Y, 5, 100, 20.),
        });
    }

    log::trace!("Models initialized");

    models
}

pub fn update(sync: &mut DemoSync) -> Scene {
    let mut instances_by_model = vec![vec![Instance {
        scale: Vec3::ONE * 60.,
        rotation: Quat::IDENTITY,
        translation: vec3(0., 0., 0.),
    }]];

    for i in 1..=10 {
        let i = i as f32 / 10.;
        instances_by_model.push(vec![
            Instance {
                scale: Vec3::ONE,
                rotation: Quat::from_axis_angle(Vec3::Y, sync.get("rotation.y")),
                translation: vec3(i * 60., 0., 0.),
            },
            Instance {
                scale: Vec3::ONE,
                rotation: Quat::from_axis_angle(Vec3::Y, sync.get("rotation.y")),
                translation: vec3(-i * 60., 0., 0.),
            },
        ]);
    }

    Scene {
        instances_by_model,
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
