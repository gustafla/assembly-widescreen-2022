mod player;
mod renderer;
mod sync;

use include_dir::{include_dir, Dir};
pub use player::Player;
pub use renderer::Renderer;
pub use sync::DemoSync;

pub static RESOURCES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/resources");

pub struct Demo {}

impl Demo {
    pub fn new() -> Self {
        let demo = Demo {};

        log::trace!("demo created");

        demo
    }

    pub fn update(&mut self, _sync: &mut DemoSync) {
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
