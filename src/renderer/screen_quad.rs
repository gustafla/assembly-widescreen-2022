use bytemuck::{Pod, Zeroable};
use glam::*;
use winit::dpi::PhysicalSize;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    position: Vec2,
    uv: Vec2,
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0=>Float32x2, 1=>Float32x2];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct ScreenQuad {
    vertex_buffer: wgpu::Buffer,
}

impl ScreenQuad {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        internal_size: PhysicalSize<u32>,
        target_size: PhysicalSize<u32>,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Screen Quad Buffer"),
            size: std::mem::size_of::<Vertex>() as u64 * 6,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut quad = Self { vertex_buffer };

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

    pub fn draw<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..6, 0..1);
    }
}
