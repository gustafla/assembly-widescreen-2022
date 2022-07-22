use super::screen_quad;
use super::PASS_TEXTURE_FORMAT;

pub enum Target {
    Surface(wgpu::TextureFormat),
    Texture(usize),
}

pub struct Pass {
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    targets: Vec<Target>,
}

impl Pass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        uniform_buffer: &wgpu::Buffer,
        sampler: (wgpu::SamplerBindingType, &wgpu::Sampler),
        depth_texture: Option<&wgpu::TextureView>,
        textures_in: &[&wgpu::TextureView],
        targets: Vec<Target>,
        shader: wgpu::ShaderModuleDescriptor,
    ) -> Self {
        let mut bgle = vec![
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
                ty: wgpu::BindingType::Sampler(sampler.0),
                count: None,
            },
        ];
        if let Some(_depth) = depth_texture {
            bgle.push(wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }
        let offset = bgle.len();
        for (i, _) in textures_in.iter().enumerate() {
            bgle.push(wgpu::BindGroupLayoutEntry {
                binding: (i + offset) as u32,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
        }

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Pass Bind Group Layout"),
            entries: &bgle,
        });

        let mut bge = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler.1),
            },
        ];
        if let Some(depth_texture) = depth_texture {
            bge.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(depth_texture),
            });
        };
        let offset = bge.len();
        for (i, tex) in textures_in.iter().enumerate() {
            bge.push(wgpu::BindGroupEntry {
                binding: (i + offset) as u32,
                resource: wgpu::BindingResource::TextureView(tex),
            });
        }

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Pass Bind Group"),
            layout: &bind_group_layout,
            entries: &bge,
        });

        let target_states: Vec<Option<wgpu::ColorTargetState>> = targets
            .iter()
            .map(|target| {
                Some(wgpu::ColorTargetState {
                    format: match target {
                        Target::Surface(format) => *format,
                        Target::Texture(_) => PASS_TEXTURE_FORMAT,
                    },
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })
            })
            .collect();

        let shader = device.create_shader_module(shader);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pass Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pass Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[screen_quad::Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &target_states,
            }),
            multiview: None,
        });

        Self {
            bind_group,
            targets,
            render_pipeline,
        }
    }

    pub fn targets(&self) -> &[Target] {
        &self.targets
    }

    pub fn set_pass_state<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
    }
}
