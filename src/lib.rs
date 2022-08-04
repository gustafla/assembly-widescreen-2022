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
use scene::{Camera, CameraView, Instance, Light, Model, Scene, VertexData};
use simdnoise::*;
use std::time::Instant;
pub use sync::DemoSync;

pub static RESOURCES_PATH: &str = "resources";
pub static RESOURCES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

const PARTICLES_COUNT: usize = 2048;
const HEIGHTMAP_SIZE: usize = 1000;

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

fn load_bitmap(path: &str, width: usize) -> VertexData {
    #[cfg(debug_assertions)]
    let image_data = std::fs::read(std::path::PathBuf::from(RESOURCES_PATH).join(path)).unwrap();
    #[cfg(not(debug_assertions))]
    let image_data = RESOURCES_DIR.get_file(path).unwrap().contents();

    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut roughness = Vec::new();

    for (i, element) in image_data.iter().step_by(2).enumerate() {
        let u = i % width;
        let v = i / width;
        if *element > 127 {
            let u0 = u as f32;
            let v0 = v as f32;
            let u1 = u as f32 + 1.;
            let v1 = v as f32 + 1.;

            let p0 = vec3(u0, -v0, 0.);
            let p1 = vec3(u1, -v0, 0.);
            let p2 = vec3(u1, -v1, 0.);
            let p3 = vec3(u0, -v1, 0.);

            positions.extend_from_slice(&[p0, p1, p2, p2, p1, p0, p2, p3, p0, p0, p3, p2]);
            colors.extend(std::iter::repeat(Hsv::new(0., 0., 1.)).take(12));
            roughness.extend(std::iter::repeat(1.).take(12));
        }
    }

    VertexData::from_triangles(positions, colors, roughness)
}

struct Heightmap {
    nu: usize,
    nv: usize,
    values: Vec<f32>,
}

impl Heightmap {
    fn new(nu: usize, nv: usize, height: f32) -> Self {
        let values = NoiseBuilder::gradient_2d(nu, nv).generate_scaled(0., height);
        Self { nu, nv, values }
    }

    fn get(&self, u: usize, v: usize) -> f32 {
        self.values[u * self.nu + v]
    }

    fn dimensions(&self) -> (usize, usize) {
        (self.nu, self.nv)
    }
}

fn generate_terrain(nu: usize, nv: usize) -> (VertexData, Heightmap) {
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    let mut roughness = Vec::new();

    let (hu, hv) = (nu as f32 / 2., nv as f32 / 2.);

    let heightmap = Heightmap::new(nu, nv, 6.);

    for u in 0..(nu - 1) {
        for v in 0..(nv - 1) {
            let u0 = u;
            let u1 = u + 1;
            let v0 = v + 1;
            let v1 = v;

            positions.push(vec3(u0 as f32 - hu, heightmap.get(u0, v0), v0 as f32 - hv));
            positions.push(vec3(u1 as f32 - hu, heightmap.get(u1, v0), v0 as f32 - hv));
            positions.push(vec3(u1 as f32 - hu, heightmap.get(u1, v1), v1 as f32 - hv));
            positions.push(vec3(u1 as f32 - hu, heightmap.get(u1, v1), v1 as f32 - hv));
            positions.push(vec3(u0 as f32 - hu, heightmap.get(u0, v1), v1 as f32 - hv));
            positions.push(vec3(u0 as f32 - hu, heightmap.get(u0, v0), v0 as f32 - hv));

            colors.extend(std::iter::repeat(Hsv::new(0., 0., 1.)).take(6));
            roughness.extend(std::iter::repeat(1.).take(6));
        }
    }

    (
        VertexData::from_triangles(positions, colors, roughness),
        heightmap,
    )
}

struct ParticleSystem {
    velocities: [Vec3; PARTICLES_COUNT],
    rotation_axes: [Vec3; PARTICLES_COUNT],
    sizes: [f32; PARTICLES_COUNT],
    lives: [f32; PARTICLES_COUNT],
}

impl ParticleSystem {
    fn new() -> Self {
        Self {
            velocities: [Vec3::ZERO; PARTICLES_COUNT],
            rotation_axes: [Vec3::ZERO; PARTICLES_COUNT],
            sizes: [0.; PARTICLES_COUNT],
            lives: [0.; PARTICLES_COUNT],
        }
    }

    fn update(
        &mut self,
        rng: &mut impl Rng,
        dt: f32,
        instances: &mut [Instance],
        heightmap: &Heightmap,
    ) {
        for (i, instance) in instances.iter_mut().enumerate() {
            // Sample heightmap at particle xz coordinates
            let u = instance.translation.x as i32 + (heightmap.dimensions().0 as i32 / 2);
            let v = instance.translation.z as i32 + (heightmap.dimensions().1 as i32 / 2);
            let height = (u >= 0
                && v >= 0
                && u < heightmap.dimensions().0 as i32
                && v < heightmap.dimensions().1 as i32)
                .then(|| heightmap.get(u as usize, v as usize))
                .unwrap_or(0.);

            // Respawn underground particles
            if instance.translation.y < height - 1. {
                instance.translation = random_vec3(rng) * vec3(40., 1., 40.) + vec3(20., 20., 20.);
                self.velocities[i] = random_vec3(rng) * 0.1;
                self.lives[i] = 0.;
                self.sizes[i] = rng.gen_range(0.1..0.3);
                instance.scale = Vec3::splat(self.sizes[i]);
                self.rotation_axes[i] = random_vec3(rng).normalize();
            }

            // Life
            self.lives[i] += dt;

            // Gravity
            let gravity = 1.;
            self.velocities[i].y -= gravity * dt * self.sizes[i];

            // "Wind"
            let wind = vec3(
                (instance.translation.x * 0.1).sin() * 0.776
                    + instance.translation.y.sin() * 0.3
                    + (instance.translation.y * 2.43).sin() * 0.1,
                instance.translation.z.sin() * 0.2,
                instance.translation.y.sin() * 0.76 + (instance.translation.y * 1.43).sin() * 0.2,
            ) * 0.3
                + Vec3::X * 0.2;
            self.velocities[i] += dt * wind;

            // Terminal velocity
            let speed = self.velocities[i].length();
            self.velocities[i] /= speed.max(1.);

            // Output
            instance.translation += self.velocities[i] * dt;
            instance.rotation =
                Quat::from_axis_angle(self.rotation_axes[i], self.lives[i] * self.sizes[i]);
        }
    }
}

pub struct State {
    last_time: Instant,
    heightmap: Heightmap,
    particles: ParticleSystem,
    scene: Scene,
    greet_models_start: usize,
}

impl State {
    pub fn new(rng: &mut impl Rng) -> (State, Vec<Model>) {
        // Add leaf model for particle system
        let mut models = vec![Model {
            vertices: generate_leaf(Hsv::new(100., 0.5, 0.5), 0.3),
        }];

        let (vertices, heightmap) = generate_terrain(HEIGHTMAP_SIZE, HEIGHTMAP_SIZE);
        // Add terrain model
        models.push(Model { vertices });

        // Add tree models
        for _ in 0..10 {
            models.push(Model {
                vertices: generate_tree(rng, 6, 20, 1., Vec3::ZERO, Vec3::Y, 5, 100, 20.),
            });
        }

        // Add greets models from bitmaps
        let greet_models_start = models.len();
        models.push(Model {
            vertices: load_bitmap("mehu.raw", 32),
        });
        models.push(Model {
            vertices: load_bitmap("deliberate64.raw", 96),
        });
        models.push(Model {
            vertices: load_bitmap("epoch64.raw", 64),
        });
        models.push(Model {
            vertices: load_bitmap("faemiyah64.raw", 96),
        });
        models.push(Model {
            vertices: load_bitmap("ivory96.raw", 96),
        });
        models.push(Model {
            vertices: load_bitmap("jml96.raw", 96),
        });
        models.push(Model {
            vertices: load_bitmap("mercury64.raw", 64),
        });
        models.push(Model {
            vertices: load_bitmap("peisik64.raw", 64),
        });
        models.push(Model {
            vertices: load_bitmap("darksecond96.raw", 96),
        });
        models.push(Model {
            vertices: load_bitmap("assembly156.raw", 156),
        });

        log::trace!("Models initialized");

        // Add particle instances
        let mut particles = ParticleSystem::new();
        let mut instances_by_model = vec![vec![
            Instance {
                scale: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                translation: Vec3::ZERO,
            };
            PARTICLES_COUNT
        ]];
        // Pre-run to have nicer looking initial state
        for _ in 0..10000 {
            particles.update(rng, 0.01, &mut instances_by_model[0], &heightmap);
        }

        // Add terrain instance
        instances_by_model.push(vec![Instance {
            scale: Vec3::ONE,
            rotation: Quat::IDENTITY,
            translation: vec3(0., 0., 0.),
        }]);

        // Sprinkle random tree instances
        for _ in 1..=10 {
            let mut instances = Vec::with_capacity(20);
            let dims = heightmap.dimensions();
            let half = vec2(dims.0 as f32, dims.1 as f32) / 2.;
            for _ in 0..200 {
                let posrand = (rng.gen_range(0..dims.0), rng.gen_range(0..dims.1));
                let height = heightmap.get(posrand.0, posrand.1);
                let xz = vec2(posrand.0 as f32, posrand.1 as f32) - half;
                instances.push(Instance {
                    scale: Vec3::ONE + random_vec3(rng) * 0.1 + rng.gen_range(0f32..2.),
                    rotation: Quat::from_axis_angle(
                        Vec3::Y,
                        rng.gen::<f32>() * std::f32::consts::TAU,
                    ),
                    translation: vec3(xz.x, height, xz.y),
                });
            }
            instances_by_model.push(instances);
        }

        // Add empty vecs for greet models
        for _ in instances_by_model.len()..models.len() {
            instances_by_model.push(Vec::new());
        }

        let scene = Scene {
            instances_by_model,
            ambient: 0.,
            bloom_floor: 1.,
            bloom_amount: 1.,
            march_multiplier: 1.,
            world_triangles: 1.,
            lights: std::iter::repeat(Light::default()).take(2).collect(),
            camera: Camera::default(),
        };
        (
            Self {
                last_time: Instant::now(),
                heightmap,
                particles,
                scene,
                greet_models_start,
            },
            models,
        )
    }

    pub fn update(&mut self, rng: &mut impl Rng, sync: &mut DemoSync) -> &Scene {
        // Compute dt
        let now = Instant::now();
        let dt = now - self.last_time;
        self.last_time = now;

        // Update particles
        self.particles.update(
            rng,
            dt.as_secs_f32(),
            &mut self.scene.instances_by_model[0],
            &self.heightmap,
        );

        // Clear all greets text instances
        for instances in self.scene.instances_by_model[self.greet_models_start..].iter_mut() {
            instances.clear();
        }
        // Add a single greet text for the configured index
        let max_i = self.scene.instances_by_model.len() - 1;
        self.scene.instances_by_model
            [(self.greet_models_start + sync.get("greet:index") as usize).min(max_i)]
        .push(Instance {
            scale: sync.get_vec3("greet:scale", None),
            rotation: Quat::from_euler(
                EulerRot::XYZ,
                sync.get("greet:rotation.x"),
                sync.get("greet:rotation.y"),
                sync.get("greet:rotation.z"),
            ),
            translation: sync.get_vec3("greet:translation", None),
        });

        // Update camera
        let camera = sync.get("camera") as usize;
        let camstr = format!("camera{camera}");
        self.scene.camera = Camera {
            fov: sync.get(&[&camstr, "fov"].join(":")),
            position: sync.get_vec3(&[&camstr, "pos"].join(":"), None),
            view: if sync.get(&[&camstr, "view"].join(":")) < 1. {
                CameraView::PitchYawRoll(sync.get_vec3(&camstr, Some([":pitch", ":yaw", ":roll"])))
            } else {
                CameraView::Target(sync.get_vec3(&[&camstr, "target"].join(":"), None))
            },
        };

        // Update lights
        for (i, light) in self.scene.lights.iter_mut().enumerate() {
            let lightstr = format!("light{i}");
            *light = Light {
                coordinates: sync.get_vec4(&[&lightstr, "coord"].join(":"), None),
                color: sync.get_hsv(&lightstr, None),
            };
        }
        self.scene.ambient = sync.get("ambient");
        self.scene.bloom_floor = sync.get("bloom_floor");
        self.scene.bloom_amount = sync.get("bloom_amount");
        self.scene.march_multiplier = sync.get("march_multiplier");
        self.scene.world_triangles = sync.get("triangles");

        &self.scene
    }
}
