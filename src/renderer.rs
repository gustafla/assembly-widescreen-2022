mod screen_pass;
mod shader_quad;

use crate::scene::Scene;
use anyhow::{Context, Result};
use bytemuck::{Pod, Zeroable};
use glam::*;
use screen_pass::ScreenPass;
use winit::{dpi::PhysicalSize, window::Window};

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VertexUniforms {
    model_mat: Mat4,
    view_projection_mat: Mat4,
    normal_mat: Mat4,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FragmentUniforms {
    light_position: Vec4,
    camera_position: Vec4,
    ambient: f32,
    diffuse: f32,
    specular: f32,
    pad: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
    pub color: Vec4,
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

pub struct Renderer {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_configuration: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    uniform_bind_group: wgpu::BindGroup,
    vertex_uniform_buffer: wgpu::Buffer,
    fragment_uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    output_screen: ScreenPass,
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

impl Renderer {
    pub async fn new(internal_size: PhysicalSize<u32>, window: &Window) -> Result<Self> {
        let size = window.inner_size();
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

        let format = surface
            .get_supported_formats(&adapter)
            .into_iter()
            .next()
            .context(format!(
                "No surface format available for adapter {}",
                adapter_name
            ))?;
        if format.describe().srgb {
            log::info!("Preferred surface is sRGB");
        } else {
            log::warn!("Preferred surface is not sRGB!");
        }
        let surface_configuration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        let shader = device.create_shader_module(get_shader("shader.wgsl"));

        let vertex_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Uniform Buffer"),
            size: std::mem::size_of::<VertexUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let fragment_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Fragment Uniform Buffer"),
            size: std::mem::size_of::<FragmentUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
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
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: vertex_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: fragment_uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: ScreenPass::DEPTH_TEXTURE_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: ScreenPass::COLOR_TEXTURE_FORMAT,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: std::mem::size_of::<Vertex>() as u64 * 4096 * 3,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_screen = ScreenPass::new(
            &device,
            &queue,
            internal_size,
            surface_configuration.format,
            size,
            get_shader("output.wgsl"),
            &[],
        );

        let mut renderer = Self {
            surface,
            device,
            queue,
            surface_configuration,
            render_pipeline,
            uniform_bind_group,
            vertex_uniform_buffer,
            fragment_uniform_buffer,
            vertex_buffer,
            output_screen,
        };

        renderer.resize(size);
        Ok(renderer)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_configuration.width = new_size.width;
            self.surface_configuration.height = new_size.height;
            self.configure_surface();
            self.output_screen
                .set_target_resolution(&self.queue, new_size);
        }
    }

    pub fn configure_surface(&self) {
        self.surface
            .configure(&self.device, &self.surface_configuration);
    }

    pub fn render(&self, scene: &Scene) -> Result<(), wgpu::SurfaceError> {
        // Update uniforms
        let resolution = self.output_screen.resolution();
        let camera_position = Vec4::from((scene.cameras[0].position, 1.));
        let view_mat =
            Mat4::look_at_rh(scene.cameras[0].position, scene.cameras[0].target, Vec3::Y);
        let project_mat = Mat4::perspective_rh(
            scene.cameras[0].fov,
            resolution.width as f32 / resolution.height as f32,
            0.1,
            100.,
        );
        let view_projection_mat = project_mat * view_mat;
        let model_mat = Mat4::from_scale_rotation_translation(
            scene.objects[0].scale,
            scene.objects[0].rotation,
            scene.objects[0].translation,
        );
        let normal_mat = model_mat.inverse().transpose();

        self.queue.write_buffer(
            &self.vertex_uniform_buffer,
            0,
            bytemuck::cast_slice(&[VertexUniforms {
                model_mat,
                view_projection_mat,
                normal_mat,
            }]),
        );

        self.queue.write_buffer(
            &self.fragment_uniform_buffer,
            0,
            bytemuck::cast_slice(&[FragmentUniforms {
                light_position: camera_position,
                camera_position,
                ambient: 0.2,
                diffuse: 0.5,
                specular: 0.3,
                pad: 0.,
            }]),
        );

        // Update Vertex Buffer
        let vertex_data: Vec<Vertex> = scene.objects[0]
            .positions
            .iter()
            .zip(scene.objects[0].normals.iter())
            .map(|(p, n)| Vertex {
                position: Vec4::from((*p, 1.)),
                normal: Vec4::from((*n, 1.)),
                color: Vec4::ONE,
            })
            .collect();
        self.queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(vertex_data.as_slice()),
        );

        // Create output screen views
        let (color_texture, depth_texture) = self.output_screen.textures();
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Render commands
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Command Encoder"),
            });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.,
                            b: 0.,
                            g: 0.,
                            a: 1.,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: false,
                    }),
                    stencil_ops: None,
                }),
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            render_pass.draw(0..vertex_data.len() as u32, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Create window surface output view
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        self.output_screen
            .render(&self.device, &self.queue, &view, &[]);
        output.present();

        Ok(())
    }
}
