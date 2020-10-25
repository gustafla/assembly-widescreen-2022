use anyhow::{anyhow, Context, Result};
use demo::{Demo, RcGl};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    Api, ContextBuilder, GlRequest,
};
use simple_logger::SimpleLogger;

fn main() -> Result<()> {
    // Init logging
    SimpleLogger::new()
        .init()
        .context("Failed to initialize logger")?;

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

    let gl = RcGl::new(|s| windowed_context.get_proc_address(s));

    let mut demo = Demo::new(size.width, size.height, gl)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(keycode),
                            ..
                        },
                    ..
                } => match keycode {
                    VirtualKeyCode::Escape | VirtualKeyCode::Q => *control_flow = ControlFlow::Exit,
                    _ => (),
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                if let Err(e) = demo.render() {
                    panic!("Unexpected runtime error! {}", e);
                }

                windowed_context
                    .swap_buffers()
                    .expect("Failed to swap buffers");
            }
            _ => (),
        }
    });
}