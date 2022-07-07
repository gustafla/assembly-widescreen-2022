mod player;
mod renderer;
pub mod scene;
mod sync;

use glam::*;
use include_dir::{include_dir, Dir};
pub use player::Player;
pub use renderer::Renderer;
use scene::{Camera, Object, Scene};
pub use sync::DemoSync;

pub static RESOURCES_PATH: &str = "resources";
pub static RESOURCES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

pub struct Demo {}

impl Demo {
    pub fn new() -> Self {
        let demo = Demo {};

        log::trace!("demo created");

        demo
    }

    pub fn update(&mut self, sync: &mut DemoSync) -> Scene {
        Scene {
            objects: vec![Object {
                positions: vec![vec3(-1., -1., 0.), vec3(1., -1., 0.), vec3(0., 1., 0.)],
                normals: vec![vec3(0., 0., 1.), vec3(0., 0., 1.), vec3(0., 0., 1.)],
                color_hsv: (1., 1., 1.),
                scale: Vec3::ONE,
                rotation: Quat::from_rotation_y(sync.get("rotation.y")),
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
            bg_color_hsv: (1., 1., 0.5),
        }

        /*let to_resolution = to_resolution.into();

        glesv2::clear_color(0., 0., 0., 1.);

        // Scene ------------------------------------------------------------------------------

        glesv2::enable(glesv2::DEPTH_TEST);

        self.bloom_pass
            .fbo()
            .bind(glesv2::COLOR_BUFFER_BIT | glesv2::DEPTH_BUFFER_BIT);

        // Bloom pass -----------------------------------------------------------------------------

        self.post_pass.fbo().bind(0);
        self.bloom_pass.render(&self, &[], &[], None);

        // Post pass ------------------------------------------------------------------------------

        glesv2::Framebuffer::bind_default(glesv2::COLOR_BUFFER_BIT);

        // Generate noise
        let noise: Vec<u8> = (0..(self.resolution().width * self.resolution().height
            / NOISE_SCALE.pow(2)))
            .map(|_| self.rng.gen())
            .collect();

        // Upload noise to Texture
        self.noise_texture.sub_image::<u8>(
            0,
            0,
            0,
            i32::try_from(self.resolution().width / NOISE_SCALE).unwrap(),
            i32::try_from(self.resolution().height / NOISE_SCALE).unwrap(),
            glesv2::LUMINANCE,
            glesv2::UNSIGNED_BYTE,
            noise.as_slice(),
        );

        // Mipmap for blur
        self.post_pass
            .fbo()
            .texture(glesv2::COLOR_ATTACHMENT0)
            .unwrap()
            .generate_mipmaps();

        self.post_pass.render(
            &self,
            &[
                self.bloom_pass
                    .fbo()
                    .texture(glesv2::COLOR_ATTACHMENT0)
                    .unwrap(),
                &self.noise_texture,
            ],
            &[
                (
                    "u_NoiseAmount",
                    glesv2::UniformValue::Float(sync.get("noise_amount")),
                ),
                (
                    "u_NoiseScale",
                    glesv2::UniformValue::Float(NOISE_SCALE as f32),
                ),
                ("u_Beat", glesv2::UniformValue::Float(sync.get_beat())),
            ],
            Some(to_resolution),
        );

        glesv2::check()*/
    }
}

impl Drop for Demo {
    fn drop(&mut self) {
        log::trace!("demo dropped");
    }
}
