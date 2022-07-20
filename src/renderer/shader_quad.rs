use bytemuck::{Pod, Zeroable};
use glam::*;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: Vec2,
    uv: Vec2,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0=>Float32x2, 1=>Float32x2];

    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct ShaderQuad {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

impl ShaderQuad {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        internal_size: PhysicalSize<u32>,
        target_size: PhysicalSize<u32>,
        targets: &[Option<wgpu::ColorTargetState>],
        shader: wgpu::ShaderModuleDescriptor,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> Self {
        let shader = device.create_shader_module(shader);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shader Quad Pipeline Layout"),
            bind_group_layouts,
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shader Quad Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
                targets,
            }),
            multiview: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Quad Buffer"),
            size: std::mem::size_of::<Vertex>() as u64 * 6,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut quad = Self {
            vertex_buffer,
            render_pipeline,
        };

        quad.resize(queue, internal_size, target_size);

        quad
    }

    pub fn resize(
        &mut self,
        queue: &wgpu::Queue,
        internal_size: PhysicalSize<u32>,
        target_size: PhysicalSize<u32>,
    ) {
        let mut left = -1.;
        let mut right = 1.;
        let mut down = -1.;
        let mut up = 1.;

        // Letterbox the aspect ratio difference
        let from_w = internal_size.width as f32;
        let from_h = internal_size.height as f32;
        let to_w = target_size.width as f32;
        let to_h = target_size.height as f32;
        let from_aspect_ratio = from_w / from_h;
        let to_aspect_ratio = to_w / to_h;
        let h_scale = from_h / to_h;
        let w_scale = from_w / to_w;

        if from_aspect_ratio < to_aspect_ratio {
            right = w_scale / h_scale;
            left = -right;
        } else {
            up = h_scale / w_scale;
            down = -up;
        };

        #[rustfmt::skip]
        let quad = [
            Vertex{position: vec2(left, down),  uv: vec2(0., 1.)},
            Vertex{position: vec2(right, down), uv: vec2(1., 1.)},
            Vertex{position: vec2(right, up),   uv: vec2(1., 0.)},
            Vertex{position: vec2(left, down),  uv: vec2(0., 1.)},
            Vertex{position: vec2(right, up),   uv: vec2(1., 0.)},
            Vertex{position: vec2(left, up),    uv: vec2(0., 0.)},
        ];

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&quad));
    }

    pub fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color_attachments: &[Option<wgpu::RenderPassColorAttachment>],
        bind_groups: &[wgpu::BindGroup],
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Shader Quad Command Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Shader Quad Render Pass"),
                color_attachments,
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            for (i, bind_group) in bind_groups.iter().enumerate() {
                render_pass.set_bind_group(i as u32, bind_group, &[]);
            }
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}
