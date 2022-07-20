mod shader_quad;

use crate::scene;
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::*;
use shader_quad::ShaderQuad;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, window::Window};

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VertexUniforms {
    view_projection_mat: Mat4,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct PostUniforms {
    inverse_view_projection_mat: Mat4,
    light_position: Vec4,
    camera_position: Vec4,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    pad: f32,
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

struct Pass {
    textures: Vec<wgpu::Texture>,
    bind_groups: Vec<wgpu::BindGroup>,
    shader_quad: ShaderQuad,
}

pub struct Renderer<const M: usize> {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    uniform_bind_group: wgpu::BindGroup,
    vertex_uniform_buffer: wgpu::Buffer,
    internal_size: PhysicalSize<u32>,
    models: [Model; M],
    instance_buffer: wgpu::Buffer,
    post_pass_uniform_buffer: wgpu::Buffer,
    post_pass: Pass,
    output_pass: Pass,
}

fn get_shader<'a>(path: &'a str) -> wgpu::ShaderModuleDescriptor<'a> {
    #[cfg(debug_assertions)]
    {
        wgpu::ShaderModuleDescriptor {
            label: Some(path),
            source: wgpu::ShaderSource::Wgsl(
                std::fs::read_to_string(std::path::PathBuf::from(crate::RESOURCES_PATH).join(path))
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

impl<const M: usize> Renderer<M> {
    fn load_model(device: &wgpu::Device, model: scene::Model) -> Model {
        let vert = model.vertices;
        let vertices: Vec<Vertex> = vert
            .positions
            .into_iter()
            .zip(vert.color_roughness.into_iter())
            .zip(vert.normals.into_iter())
            .map(|((position, color_roughness), normal)| Vertex {
                position: Vec4::from((position, 1.)),
                color_roughness,
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
        models: [scene::Model; M],
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

        // Output Pass ----------------------------------------------------------------------------

        let size = wgpu::Extent3d {
            width: internal_size.width,
            height: internal_size.height,
            depth_or_array_layers: 1,
        };

        let output_pass_color_format = wgpu::TextureFormat::Rgba16Float;

        let textures = vec![device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: output_pass_color_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Output Pass Color Texture"),
        })];

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Output Pass Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = vec![device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Output Pass Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            label: Some("Output Pass Color Sampler"),
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Linear,
                            min_filter: wgpu::FilterMode::Linear,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        })];

        let output_pass = Pass {
            textures,
            bind_groups,
            shader_quad: ShaderQuad::new(
                &device,
                &queue,
                internal_size,
                surface_size,
                surface_format,
                get_shader("output.wgsl"),
                &[&bind_group_layout],
            ),
        };

        // Post Pass ----------------------------------------------------------------------------

        let post_pass_color_format = wgpu::TextureFormat::Rgba16Float;
        let post_pass_normal_format = wgpu::TextureFormat::Rgba16Float;
        let post_pass_depth_format = wgpu::TextureFormat::Depth32Float;

        let textures = vec![
            device.create_texture(&wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: post_pass_color_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Post Pass Color and Roughness Texture"),
            }),
            device.create_texture(&wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: post_pass_normal_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Post Pass Normal Texture"),
            }),
            device.create_texture(&wgpu::TextureDescriptor {
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: post_pass_depth_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                label: Some("Post Pass Depth Texture"),
            }),
        ];

        let post_pass_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Post Pass Uniform Buffer"),
            size: std::mem::size_of::<PostUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Post Pass Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let bind_groups = vec![device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Post Pass Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: post_pass_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&device.create_sampler(
                        &wgpu::SamplerDescriptor {
                            label: Some("Output Pass Sampler"),
                            address_mode_u: wgpu::AddressMode::ClampToEdge,
                            address_mode_v: wgpu::AddressMode::ClampToEdge,
                            address_mode_w: wgpu::AddressMode::ClampToEdge,
                            mag_filter: wgpu::FilterMode::Nearest,
                            min_filter: wgpu::FilterMode::Nearest,
                            mipmap_filter: wgpu::FilterMode::Nearest,
                            ..Default::default()
                        },
                    )),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(
                        &textures[2].create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
            ],
        })];

        let post_pass = Pass {
            textures,
            bind_groups,
            shader_quad: ShaderQuad::new(
                &device,
                &queue,
                internal_size,
                internal_size,
                output_pass_color_format,
                get_shader("post.wgsl"),
                &[&bind_group_layout],
            ),
        };

        // Scene ----------------------------------------------------------------------------------

        let shader = device.create_shader_module(get_shader("defer.wgsl"));

        let vertex_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Uniform Buffer"),
            size: std::mem::size_of::<VertexUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
                resource: vertex_uniform_buffer.as_entire_binding(),
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
                format: post_pass_depth_format,
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
                        format: post_pass_color_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent::REPLACE,
                            alpha: wgpu::BlendComponent::REPLACE,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    }),
                    Some(wgpu::ColorTargetState {
                        format: post_pass_normal_format,
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
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instance Buffer"),
            size: std::mem::size_of::<Instance>() as u64 * 1024,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut renderer = Self {
            surface,
            device,
            queue,
            surface_configuration,
            render_pipeline,
            uniform_bind_group,
            vertex_uniform_buffer,
            internal_size,
            models,
            instance_buffer,
            post_pass_uniform_buffer,
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
            self.output_pass
                .shader_quad
                .resize(&self.queue, self.internal_size, new_size);
        }
    }

    pub fn configure_surface(&self) {
        self.surface
            .configure(&self.device, &self.surface_configuration);
    }

    pub fn render(&self, scene: &scene::Scene<M>) -> Result<(), wgpu::SurfaceError> {
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
        self.queue.write_buffer(
            &self.vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[VertexUniforms {
                view_projection_mat,
            }]),
        );

        // Update instances
        let instances: Vec<Instance> = scene
            .instances_by_model
            .iter()
            .map(|inst| inst.iter())
            .flatten()
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

        // Create post processing texture views
        let color_view =
            self.post_pass.textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        let normal_view =
            self.post_pass.textures[1].create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view =
            self.post_pass.textures[2].create_view(&wgpu::TextureViewDescriptor::default());

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
                        view: &color_view,
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
                        view: &normal_view,
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
                    view: &depth_view,
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

        // Post process to output pass ------------------------------------------------------------

        self.queue.write_buffer(
            &self.post_pass_uniform_buffer,
            0,
            bytemuck::cast_slice(&[PostUniforms {
                inverse_view_projection_mat: view_projection_mat.inverse(),
                light_position: camera_position,
                camera_position,
                ambient: 0.2,
                diffuse: 0.5,
                specular: 0.3,
                pad: 0.,
            }]),
        );

        let view =
            self.output_pass.textures[0].create_view(&wgpu::TextureViewDescriptor::default());
        self.post_pass.shader_quad.render(
            &self.device,
            &self.queue,
            &view,
            &self.post_pass.bind_groups,
        );

        // Output (scaling) to window pass --------------------------------------------------------

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.output_pass.shader_quad.render(
            &self.device,
            &self.queue,
            &view,
            &self.output_pass.bind_groups,
        );

        output.present();

        Ok(())
    }
}
