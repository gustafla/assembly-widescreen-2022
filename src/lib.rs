mod glesv2;
mod particle_system;
mod render_pass;
mod terrain;

use cgmath::{Angle, Deg, Euler, InnerSpace, Matrix4, Point3, Quaternion, Rad, Vector2, Vector3};
pub use glesv2::{
    types::*, Framebuffer, Gles2, RcGl, Renderbuffer, RenderbufferAttachment, ResourceMapper,
    Texture, UniformValue,
};
use particle_system::{
    ParticleSpawner, ParticleSpawnerKind, ParticleSpawnerMethod, ParticleSystem,
};
use rand::prelude::*;
use rand_xorshift::XorShiftRng;
use render_pass::RenderPass;
use std::collections::HashMap;
use std::ffi::CString;
use std::os::raw::{c_char, c_void};
use std::rc::Rc;
use terrain::Terrain;

const NOISE_SCALE: i32 = 8;

pub struct Scene {
    tracks: HashMap<Rc<CString>, *const c_void>,
    sync_get_track_raw: extern "C" fn(*const c_char) -> *const c_void,
    sync_get_value_raw: extern "C" fn(*const c_void) -> f64,
    pub resolution: (i32, i32),
    pub projection: [f32; 16],
    pub view: [f32; 16],
    pub resources: ResourceMapper,
    pub gl: RcGl,
    rng: XorShiftRng,
    noise_texture: Texture,
    particle_system: ParticleSystem,
    terrain: Terrain,
    bloom_pass: RenderPass,
    blur_pass_x: RenderPass,
    blur_pass_y: RenderPass,
    post_pass: RenderPass,
}

impl Scene {
    pub fn sync_get(&mut self, name: &str) -> f64 {
        let string = Rc::new(CString::new(name).unwrap());
        let get = self.sync_get_track_raw;
        let track = self.tracks.entry(string.clone()).or_insert_with(|| {
            log::trace!("Calling get track {:?}", string);
            get(string.as_ptr())
        });
        (self.sync_get_value_raw)(*track)
    }
}

fn log_and_panic(error: Box<dyn std::error::Error>) -> ! {
    log::error!("{}", error);
    let mut source = error.source();
    while let Some(e) = source {
        log::error!("Caused by {}", e);
        source = e.source();
    }
    panic!();
}

#[no_mangle]
extern "C" fn scene_init(
    w: i32,
    h: i32,
    sync_get_track_raw: extern "C" fn(*const c_char) -> *const c_void,
    sync_get_value_raw: extern "C" fn(*const c_void) -> f64,
) -> Box<Scene> {
    simple_logger::init().unwrap_or_else(|e| panic!("Failed to initialize logger\n{}", e));

    let gl = RcGl::new();

    unsafe {
        gl.Viewport(0, 0, w, h);
        gl.BlendFunc(glesv2::SRC_ALPHA, glesv2::ONE_MINUS_SRC_ALPHA);
        gl.Enable(glesv2::CULL_FACE);
        gl.DepthFunc(glesv2::LESS);
    }

    let particle_system = ParticleSystem::new(
        gl.clone(),
        ParticleSpawner::new(
            Vector3::new(0., 2., 0.),
            ParticleSpawnerKind::Box((-5., 0., -5.), (5., 5., 5.)),
            ParticleSpawnerMethod::Once(100000),
        ),
        30.,
        60,
        |pos, time| {
            Vector3::unit_y() * f32::sin(pos.x / 4. + time) * 0.6
                + (Quaternion::from(Euler {
                    x: Rad(0f32),
                    y: Angle::atan2(pos.x, pos.z),
                    z: Rad(0f32),
                }) * Vector3::unit_x()
                    / Vector2::new(pos.x, pos.z).magnitude()
                    * (pos.y + 2.))
                    * (5. - time).max(0.)
        },
    );

    let noise_texture = Texture::new(gl.clone(), glesv2::TEXTURE_2D);
    noise_texture.image::<u8>(
        0,
        glesv2::LUMINANCE,
        w / NOISE_SCALE,
        h / NOISE_SCALE,
        glesv2::UNSIGNED_BYTE,
        None,
    );
    noise_texture.parameters(&[
        (glesv2::TEXTURE_MIN_FILTER, glesv2::NEAREST),
        (glesv2::TEXTURE_MAG_FILTER, glesv2::NEAREST),
        (glesv2::TEXTURE_WRAP_S, glesv2::REPEAT),
        (glesv2::TEXTURE_WRAP_T, glesv2::REPEAT),
    ]);

    let scene = Box::new(Scene {
        tracks: HashMap::new(),
        sync_get_track_raw,
        sync_get_value_raw,
        resolution: (w, h),
        projection: *cgmath::perspective(Deg(60f32), w as f32 / h as f32, 0.1, 1000.).as_ref(),
        view: [0f32; 16],
        resources: ResourceMapper::new(gl.clone()).unwrap_or_else(|e| log_and_panic(e)),
        gl: gl.clone(),
        rng: XorShiftRng::seed_from_u64(98341),
        noise_texture,
        particle_system,
        terrain: Terrain::new(gl.clone(), 200, 200, |x, z| {
            (x * 0.2).sin() * 2. + (z * 0.4).sin() - 2.
        }),
        bloom_pass: RenderPass::new(
            gl.clone(),
            w,
            h,
            "./bloom.frag",
            Some(vec![(glesv2::DEPTH_ATTACHMENT, {
                let renderbuffer = Renderbuffer::new(gl.clone());
                renderbuffer.storage(glesv2::DEPTH_COMPONENT16, w, h);
                RenderbufferAttachment { renderbuffer }
            })]),
        ),
        blur_pass_x: RenderPass::new(gl.clone(), w, h, "./two_pass_gaussian_blur.frag", None),
        blur_pass_y: RenderPass::new(gl.clone(), w, h, "./two_pass_gaussian_blur.frag", None),
        post_pass: RenderPass::new(gl.clone(), w, h, "./post.frag", None),
    });

    log::info!("scene created");

    scene
}

#[no_mangle]
extern "C" fn scene_deinit(_: Box<Scene>) {
    log::info!("scene dropped");
}

#[no_mangle]
extern "C" fn scene_render(_time: f64, scene: Box<Scene>) {
    let mut scene = Box::leak(scene);

    let cam_pos = Point3::new(
        scene.sync_get("cam:pos.x") as f32,
        scene.sync_get("cam:pos.y") as f32,
        scene.sync_get("cam:pos.z") as f32,
    );
    scene.view = *Matrix4::look_at(
        cam_pos,
        Point3::new(
            scene.sync_get("cam:target.x") as f32,
            scene.sync_get("cam:target.y") as f32,
            scene.sync_get("cam:target.z") as f32,
        ), // center
        Vector3::unit_y(),
    )
    .as_ref();

    scene
        .bloom_pass
        .fbo
        .bind(glesv2::COLOR_BUFFER_BIT | glesv2::DEPTH_BUFFER_BIT);

    // Terrain and particle system ----------------------------------------------------------------

    unsafe {
        scene.gl.Enable(glesv2::DEPTH_TEST);
        scene.gl.Enable(glesv2::BLEND);
    }

    let sim_time = scene.sync_get("sim_time") as f32;
    let lightpos =
        scene
            .particle_system
            .prepare(cam_pos.to_homogeneous().truncate(), sim_time, 128);
    scene.terrain.render(&scene, lightpos);

    scene.particle_system.render(&scene);

    unsafe {
        scene.gl.Disable(glesv2::BLEND);
        scene.gl.Disable(glesv2::DEPTH_TEST);
    }

    // Bloom pass ---------------------------------------------------------------------------------

    scene.blur_pass_x.fbo.bind(0);
    scene.bloom_pass.render(&scene, &[], &[]);

    // X-blur pass --------------------------------------------------------------------------------

    scene.blur_pass_y.fbo.bind(0);
    scene.blur_pass_x.render(
        &scene,
        &[],
        &[("u_BlurDirection", UniformValue::Vec2f(1., 0.))],
    );

    // Y-blur pass --------------------------------------------------------------------------------

    scene.post_pass.fbo.bind(0);
    scene.blur_pass_y.render(
        &scene,
        &[],
        &[("u_BlurDirection", UniformValue::Vec2f(0., 1.))],
    );

    // Post pass ----------------------------------------------------------------------------------

    // Generate noise
    let noise: Vec<u8> = (0..(scene.resolution.0 * scene.resolution.1 / NOISE_SCALE.pow(2)))
        .map(|_| scene.rng.gen())
        .collect();

    // Upload noise to Texture
    scene.noise_texture.sub_image::<u8>(
        0,
        0,
        0,
        scene.resolution.0 / NOISE_SCALE,
        scene.resolution.1 / NOISE_SCALE,
        glesv2::LUMINANCE,
        glesv2::UNSIGNED_BYTE,
        noise.as_slice(),
    );

    let noise_amount = UniformValue::Float(scene.sync_get("noise_amount") as f32);

    Framebuffer::bind_default(scene.gl.clone(), 0);
    scene.post_pass.render(
        &scene,
        &[
            scene
                .bloom_pass
                .fbo
                .texture(glesv2::COLOR_ATTACHMENT0)
                .unwrap(),
            &scene.noise_texture,
        ],
        &[
            ("u_NoiseAmount", noise_amount),
            ("u_NoiseScale", UniformValue::Float(NOISE_SCALE as f32)),
        ],
    );

    glesv2::check(scene.gl.clone()).unwrap_or_else(|e| log_and_panic(Box::new(e)));
}
