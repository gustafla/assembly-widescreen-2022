use anyhow::{anyhow, Context, Result};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlRequest,
};

fn main() -> Result<()> {
    // Build a window
    let size = PhysicalSize::new(1280, 720);
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("mehustin")
        .with_inner_size(size);
    let windowed_context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
        .with_vsync(true)
        .build_windowed(window_builder, &event_loop)
        .context("Failed to build a window")?;

    let windowed_context = unsafe { windowed_context.make_current() }
        .map_err(|e| anyhow!("Failed to make context current: {:?}", e))?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                _ => (),
            },
            Event::RedrawRequested(_) => {
                windowed_context
                    .swap_buffers()
                    .expect("Failed to swap buffers");
            }
            _ => (),
        }
    });
}
