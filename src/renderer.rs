mod pass;
mod screen_quad;

use crate::scene;
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use color_space::{Hsv, Rgb};
use glam::*;
use pass::Pass;
use rand::prelude::*;
use rand_xoshiro::Xoshiro128Plus;
use screen_quad::ScreenQuad;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

const MAX_LIGHTS: usize = 8;
const POST_NOISE_SIZE: u32 = 128;
const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const PASS_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const PASS_TEXTURES: usize = 3;

struct NormRgb(Vec3);
impl From<Hsv> for NormRgb {
    fn from(hsv: Hsv) -> Self {
        let rgb = Rgb::from(hsv);
        NormRgb(vec3(rgb.r as f32, rgb.g as f32, rgb.b as f32) / 255.)
    }
}

impl From<NormRgb> for Vec3 {
    fn from(nrg: NormRgb) -> Self {
        nrg.0
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Light {
    coordinates: Vec4,
    rgb_intensity: Vec3,
    _pad: f32,
}

impl Default for Light {
    fn default() -> Self {
        Light {
            coordinates: Vec4::ONE,
            rgb_intensity: Vec3::ZERO,
            _pad: 0.,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct RenderUniforms {
    view_projection_mat: Mat4,
    inverse_view_projection_mat: Mat4,
    camera_position: Vec4,
    lights: [Light; MAX_LIGHTS],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PostUniforms {
    screen_size: Vec2,
    post_noise_size: Vec2,
    bloom_offset: Vec2,
    bloom_sample_bias: f32,
    _pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: Vec4,
    color_roughness: Vec4,
    normal: Vec4,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0=>Float32x4, 1=>Float32x4, 2=>Float32x4];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Instance {
    model: Mat4,
    normal: Mat4,
}

impl Instance {
    const ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![8=>Float32x4, 9=>Float32x4, 10=>Float32x4, 11=>Float32x4, 12=>Float32x4, 13=>Float32x4, 14=>Float32x4, 15=>Float32x4];
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

#[derive(Debug)]
struct Model {
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
}

pub struct Renderer {
    rng: Xoshiro128Plus,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    uniform_bind_group: wgpu::BindGroup,
    render_uniform_buffer: wgpu::Buffer,
    post_uniform_buffer: wgpu::Buffer,
    internal_size: PhysicalSize<u32>,
    models: Vec<Model>,
    instance_buffer: wgpu::Buffer,
    pass_quad: ScreenQuad,
    surface_quad: ScreenQuad,
    post_noise_texture: wgpu::Texture,
    depth_texture: wgpu::TextureView,
    rgba_textures: [wgpu::TextureView; PASS_TEXTURES],
    light_pass: Pass,
    bloom_x_pass: Pass,
    bloom_y_pass: Pass,
    post_pass: Pass,
    output_pass: Pass,
}

impl Renderer {
    fn get_shader(path: &str) -> wgpu::ShaderModuleDescriptor<'_> {
        #[cfg(debug_assertions)]
        {
            wgpu::ShaderModuleDescriptor {
                label: Some(path),
                source: wgpu::ShaderSource::Wgsl(
                    std::fs::read_to_string(
                        std::path::PathBuf::from(crate::RESOURCES_PATH).join(path),
                    )
                    .unwrap()
                    .into(),
                ),
            }
        }
        #[cfg(not(debug_assertions))]
        {
            wgpu::ShaderModuleDescriptor {
                label: Some(path),
                source: wgpu::ShaderSource::Wgsl(
                    crate::RESOURCES_DIR
                        .get_file(path)
                        .unwrap()
                        .contents_utf8()
                        .unwrap()
                        .into(),
                ),
            }
        }
    }

    fn load_model(device: &wgpu::Device, model: scene::Model) -> Model {
        let vert = model.vertices;
        let vertices: Vec<Vertex> = vert
            .positions
            .into_iter()
            .zip(vert.colors.into_iter())
            .zip(vert.roughness.into_iter())
            .zip(vert.normals.into_iter())
            .map(|(((position, color), roughness), normal)| Vertex {
                position: Vec4::from((position, 1.)),
                color_roughness: Vec4::from((Vec3::from(NormRgb::from(color)), roughness)),
                normal: Vec4::from((normal, 0.)),
            })
            .collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Object Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Model {
            vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }

    pub async fn new(
        internal_size: PhysicalSize<u32>,
        window: &Window,
        models: Vec<scene::Model>,
        rng: Xoshiro128Plus,
    ) -> Result<Self> {
        // Init & surface -------------------------------------------------------------------------

        let surface_size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .context("Cannot find a graphics adapter")?;

        let adapter_name = adapter.get_info().name;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device Descriptor"),
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .context(format!("Failed to initialize {}", adapter_name))?;
        log::info!("Created device on adapter {}", adapter_name);

        let surface_format = surface
            .get_supported_formats(&adapter)
            .into_iter()
            .next()
            .context(format!(
                "No surface format available for adapter {}",
                adapter_name
            ))?;
        if surface_format.describe().srgb {
            log::info!("Preferred surface is sRGB");
        } else {
            log::warn!("Preferred surface is not sRGB!");
        }
        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: surface_size.width,
            height: surface_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        // Common Resources -----------------------------------------------------------------------

        let render_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Render Uniform Buffer"),
            size: std::mem::size_of::<RenderUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let post_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Post Uniform Buffer"),
            size: std::mem::size_of::<PostUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sampler = (
            wgpu::SamplerBindingType::NonFiltering,
            &device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Pass Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
        );

        let filtering_sampler = (
            wgpu::SamplerBindingType::Filtering,
            &device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Pass Sampler"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
        );

        let repeating_sampler = (
            wgpu::SamplerBindingType::NonFiltering,
            &device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Pass Sampler"),
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }),
        );

        let size = wgpu::Extent3d {
            width: internal_size.width,
            height: internal_size.height,
            depth_or_array_layers: 1,
        };

        let depth_texture = device
            .create_texture(&wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_TEXTURE_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Depth Texture"),
            })
            .create_view(&wgpu::TextureViewDescriptor::default());

        let rgba_textures: [wgpu::TextureView; PASS_TEXTURES] = (0..PASS_TEXTURES)
            .map(|_| {
                device
                    .create_texture(&wgpu::TextureDescriptor {
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: PASS_TEXTURE_FORMAT,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                        label: Some("Pass Texture"),
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            })
            .collect::<Vec<wgpu::TextureView>>()
            .try_into()
            .unwrap();

        let post_noise_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Post Noise Texture"),
            size: wgpu::Extent3d {
                width: POST_NOISE_SIZE,
                height: POST_NOISE_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        });

        // Scene ----------------------------------------------------------------------------------

        let shader = device.create_shader_module(Self::get_shader("defer.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: render_uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), Instance::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[
                    Some(wgpu::ColorTargetState {
                        format: PASS_TEXTURE_FORMAT,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: PASS_TEXTURE_FORMAT,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                ],
            }),
            multiview: None,
        });

        let models = models
            .into_iter()
            .map(|m| Self::load_model(&device, m))
            .collect();

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: std::mem::size_of::<Instance>() as u64 * 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Passes ---------------------------------------------------------------------------------

        // textures depth + (0, 1) -> (2)
        let light_pass = Pass::new(
            &device,
            &render_uniform_buffer,
            sampler,
            Some(&depth_texture),
            &[&rgba_textures[0], &rgba_textures[1]],
            vec![pass::Target::Texture(2)],
            Self::get_shader("light.wgsl"),
        );

        // textures (2) -> (0)
        let bloom_x_pass = Pass::new(
            &device,
            &post_uniform_buffer,
            sampler,
            None,
            &[&rgba_textures[2]],
            vec![pass::Target::Texture(0)],
            Self::get_shader("bloom.wgsl"),
        );

        // textures (0) -> (1)
        let bloom_y_pass = Pass::new(
            &device,
            &post_uniform_buffer,
            sampler,
            None,
            &[&rgba_textures[0]],
            vec![pass::Target::Texture(1)],
            Self::get_shader("bloom.wgsl"),
        );

        // textures (2, 1) + post noise -> (0)
        let post_pass = Pass::new(
            &device,
            &post_uniform_buffer,
            repeating_sampler,
            None,
            &[
                &rgba_textures[2],
                &rgba_textures[1],
                &post_noise_texture.create_view(&wgpu::TextureViewDescriptor::default()),
            ],
            vec![pass::Target::Texture(0)],
            Self::get_shader("post.wgsl"),
        );

        // textures (0) -> surface
        let output_pass = Pass::new(
            &device,
            &post_uniform_buffer,
            filtering_sampler,
            None,
            &[&rgba_textures[0]],
            vec![pass::Target::Surface(surface_format)],
            Self::get_shader("output.wgsl"),
        );

        let surface_quad = ScreenQuad::new(&device, &queue, internal_size, surface_size);
        let pass_quad = ScreenQuad::new(&device, &queue, internal_size, internal_size);

        let mut renderer = Self {
            rng,
            surface,
            device,
            queue,
            surface_configuration,
            render_pipeline,
            uniform_bind_group,
            render_uniform_buffer,
            post_uniform_buffer,
            internal_size,
            models,
            instance_buffer,
            surface_quad,
            pass_quad,
            post_noise_texture,
            depth_texture,
            rgba_textures,
            light_pass,
            bloom_x_pass,
            bloom_y_pass,
            post_pass,
            output_pass,
        };

        renderer.resize(surface_size);
        Ok(renderer)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_configuration.width = new_size.width;
            self.surface_configuration.height = new_size.height;
            self.configure_surface();
            self.surface_quad
                .resize(&self.queue, self.internal_size, new_size);
        }
    }

    pub fn configure_surface(&self) {
        self.surface
            .configure(&self.device, &self.surface_configuration);
    }

    fn render_screen_pass<'a>(
        &self,
        encoder: &'a mut wgpu::CommandEncoder,
        surface_texture: &wgpu::TextureView,
        pass: &Pass,
        quad: &ScreenQuad,
    ) {
        let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = pass
            .targets()
            .iter()
            .map(|target| {
                Some(wgpu::RenderPassColorAttachment {
                    view: match target {
                        pass::Target::Surface(_) => surface_texture,
                        pass::Target::Texture(id) => &self.rgba_textures[*id],
                    },
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.,
                            g: 0.,
                            b: 0.,
                            a: 1.,
                        }),
                        store: true,
                    },
                })
            })
            .collect();

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
        });

        pass.set_pass_state(&mut render_pass);
        quad.draw(&mut render_pass);
    }

    pub fn render(&mut self, scene: &scene::Scene) -> Result<(), wgpu::SurfaceError> {
        // Get surface texture
        let surface_texture = self.surface.get_current_texture()?;
        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Update uniforms
        let camera_position = Vec4::from((scene.camera.position, 1.));
        let view_mat = Mat4::look_at_rh(scene.camera.position, scene.camera.target, Vec3::Y);
        let projection_mat = Mat4::perspective_rh(
            scene.camera.fov,
            self.internal_size.width as f32 / self.internal_size.height as f32,
            0.1,
            100.,
        );
        let view_projection_mat = projection_mat * view_mat;
        let mut lights: [Light; MAX_LIGHTS] = [Light::default(); MAX_LIGHTS];
        for (i, light) in scene.lights.iter().take(MAX_LIGHTS).enumerate() {
            lights[i] = Light {
                coordinates: light.coordinates,
                rgb_intensity: Vec3::from(NormRgb::from(light.color)),
                _pad: 0.,
            }
        }
        self.queue.write_buffer(
            &self.render_uniform_buffer,
            0,
            bytemuck::cast_slice(&[RenderUniforms {
                view_projection_mat,
                inverse_view_projection_mat: view_projection_mat.inverse(),
                camera_position,
                lights,
            }]),
        );

        // Update textures
        let noise: Vec<u8> = (0..POST_NOISE_SIZE.pow(2) * 4)
            .map(|_| self.rng.gen())
            .collect();
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.post_noise_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &noise,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * POST_NOISE_SIZE),
                rows_per_image: std::num::NonZeroU32::new(POST_NOISE_SIZE),
            },
            wgpu::Extent3d {
                width: POST_NOISE_SIZE,
                height: POST_NOISE_SIZE,
                depth_or_array_layers: 1,
            },
        );

        // Update instances
        let instances: Vec<Instance> = scene
            .instances_by_model
            .iter()
            .flat_map(|inst| inst.iter())
            .map(|i| {
                let model =
                    Mat4::from_scale_rotation_translation(i.scale, i.rotation, i.translation);
                Instance {
                    model,
                    normal: model.inverse().transpose(),
                }
            })
            .collect();
        self.queue
            .write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        // Render commands
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rgba_textures[0],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                b: 0.,
                                g: 0.,
                                a: 0.,
                            }),
                            store: true,
                        },
                    }),
                    Some(wgpu::RenderPassColorAttachment {
                        view: &self.rgba_textures[1],
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.,
                                b: 0.,
                                g: 0.,
                                a: 0.,
                            }),
                            store: true,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            let mut instance_offset = 0;
            for (model_id, instances) in scene.instances_by_model.iter().enumerate() {
                // Draw instances of current model
                render_pass.set_vertex_buffer(0, self.models[model_id].vertex_buffer.slice(..));
                render_pass.draw(
                    0..self.models[model_id].num_vertices,
                    instance_offset..(instance_offset + instances.len() as u32),
                );
                // Bump instance buffer slice offset for next model
                instance_offset += instances.len() as u32;
            }
        }

        self.queue.submit(Some(encoder.finish()));

        // Render deferred lighting ---------------------------------------------------------------

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render_screen_pass(
            &mut encoder,
            &surface_view,
            &self.light_pass,
            &self.pass_quad,
        );
        self.queue.submit(Some(encoder.finish()));

        // Render Bloom X -------------------------------------------------------------------------

        let mut post_uniforms = PostUniforms {
            screen_size: vec2(
                self.internal_size.width as f32,
                self.internal_size.height as f32,
            ),
            post_noise_size: vec2(POST_NOISE_SIZE as f32, POST_NOISE_SIZE as f32),
            bloom_offset: vec2(1., 0.),
            bloom_sample_bias: 2.,
            _pad: 0.,
        };

        self.queue.write_buffer(
            &self.post_uniform_buffer,
            0,
            bytemuck::cast_slice(&[post_uniforms]),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render_screen_pass(
            &mut encoder,
            &surface_view,
            &self.bloom_x_pass,
            &self.pass_quad,
        );
        self.queue.submit(Some(encoder.finish()));

        // Render Bloom Y -------------------------------------------------------------------------

        post_uniforms.bloom_offset = vec2(0., 1.);
        post_uniforms.bloom_sample_bias = 0.;
        self.queue.write_buffer(
            &self.post_uniform_buffer,
            0,
            bytemuck::cast_slice(&[post_uniforms]),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render_screen_pass(
            &mut encoder,
            &surface_view,
            &self.bloom_y_pass,
            &self.pass_quad,
        );
        self.queue.submit(Some(encoder.finish()));

        // Render Post Processing -----------------------------------------------------------------

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render_screen_pass(
            &mut encoder,
            &surface_view,
            &self.post_pass,
            &self.pass_quad,
        );
        self.queue.submit(Some(encoder.finish()));

        // Output (scaling) to window pass --------------------------------------------------------

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.render_screen_pass(
            &mut encoder,
            &surface_view,
            &self.output_pass,
            &self.surface_quad,
        );
        self.queue.submit(Some(encoder.finish()));

        surface_texture.present();

        Ok(())
    }
}
