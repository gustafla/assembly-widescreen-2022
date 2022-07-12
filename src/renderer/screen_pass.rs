use super::shader_quad::ShaderQuad;
use winit::dpi::PhysicalSize;

pub struct ScreenPass {
    color_texture: wgpu::Texture,
    depth_texture: wgpu::Texture,
    texture_bind_group: wgpu::BindGroup,
    shader_quad: super::shader_quad::ShaderQuad,
}

impl ScreenPass {
    pub const COLOR_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
    pub const DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        resolution: PhysicalSize<u32>,
        target_format: wgpu::TextureFormat,
        target_resolution: PhysicalSize<u32>,
        shader: wgpu::ShaderModuleDescriptor,
        input_bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> Self {
        let size = wgpu::Extent3d {
            width: resolution.width,
            height: resolution.height,
            depth_or_array_layers: 1,
        };

        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::COLOR_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Screen Pass Color Texture"),
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Screen Pass Depth Texture"),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Screen Pass Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Screen Pass Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, // Color
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, // Depth
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2, // Sampler
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Screen Pass Texture Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &color_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        &depth_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut bgls = vec![&texture_bind_group_layout];
        bgls.extend_from_slice(input_bind_group_layouts);

        Self {
            color_texture,
            depth_texture,
            texture_bind_group,
            shader_quad: ShaderQuad::new(
                device,
                queue,
                resolution,
                target_format,
                target_resolution,
                shader,
                &bgls,
            ),
        }
    }

    pub fn textures(&self) -> (&wgpu::Texture, &wgpu::Texture) {
        (&self.color_texture, &self.depth_texture)
    }

    pub fn resolution(&self) -> PhysicalSize<u32> {
        self.shader_quad.resolution()
    }

    pub fn set_target_resolution(
        &mut self,
        queue: &wgpu::Queue,
        target_resolution: PhysicalSize<u32>,
    ) {
        self.shader_quad
            .set_target_resolution(queue, target_resolution);
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        input_bind_groups: &[&wgpu::BindGroup],
    ) {
        let mut bgs = vec![&self.texture_bind_group];
        bgs.extend_from_slice(input_bind_groups);

        self.shader_quad.render(device, queue, target, &bgs);
    }
}
