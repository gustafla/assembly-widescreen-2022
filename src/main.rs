mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{Demo, Player, RcGl, Sync};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::unix::WindowBuilderExtUnix,
    window::WindowBuilder,
    Api, ContextBuilder, GlRequest,
};

fn main() -> Result<()> {
    // Initialize logging
    log::set_logger(&logger::Logger).unwrap();
    log::set_max_level(log::LevelFilter::max());

    // Build a window with an OpenGL context
    let size = PhysicalSize::new(1280, 720);
    let event_loop = EventLoop::new();
    let window_builder = WindowBuilder::new()
        .with_title("Demo")
        .with_app_id("demo".into())
        .with_inner_size(size);
    let windowed_context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
        .with_vsync(true)
        .with_srgb(true)
        .with_hardware_acceleration(None)
        .build_windowed(window_builder, &event_loop)
        .context("Failed to build a window")?;
    let windowed_context = unsafe { windowed_context.make_current() }
        .map_err(|e| anyhow!("Failed to make context current: {:?}", e))?;

    // Load OpenGL interface
    let gl = RcGl::new(|s| windowed_context.get_proc_address(s));

    // Load music
    let mut player = Player::new("resources/music.ogg").context("Failed to load music")?;

    // Initialize rocket
    let mut sync = Sync::new(120., 8.);

    // Load demo content
    let mut demo = Demo::new(size.width, size.height, gl)?;

    #[cfg(not(debug_assertions))]
    player.play();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        #[cfg(not(debug_assertions))]
        if player.time_secs() >= player.len_secs() {
            *control_flow = ControlFlow::Exit;
        }

        sync.update(&mut player);

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
                if let Err(e) = demo.render(&mut sync) {
                    panic!("{}", e);
                }

                windowed_context
                    .swap_buffers()
                    .expect("Failed to swap buffers");
            }
            _ => (),
        }
    });
}
