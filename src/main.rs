mod logger;

use anyhow::{anyhow, Context, Result};
use argh::FromArgs;
use demo::{Demo, Player, RcGl, Sync};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::unix::WindowBuilderExtUnix,
    window::WindowBuilder,
    Api, ContextBuilder, GlRequest,
};

/// A demo by Mehu
#[derive(FromArgs)]
struct CliArgs {
    /// list available monitors and their video modes
    #[argh(switch)]
    list_monitors: bool,
    /// run in exclusive fullscreen mode
    #[argh(switch)]
    fullscreen: bool,
}

fn list_monitors<T>(event_loop: EventLoop<T>) {
    for (i, monitor) in event_loop.available_monitors().enumerate() {
        println!("Monitor {}:", i);
        for (j, video_mode) in monitor.video_modes().enumerate() {
            println!(
                "\tMode {}: {}x{} {}-bit {}Hz",
                j,
                video_mode.size().width,
                video_mode.size().height,
                video_mode.bit_depth(),
                video_mode.refresh_rate()
            );
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    log::set_logger(&logger::Logger).unwrap();
    log::set_max_level(log::LevelFilter::max());

    // Initialize window stuff
    let title = "Demo";
    let size = PhysicalSize::new(1280, 720);
    let event_loop = EventLoop::new();

    // Process CLI
    let args: CliArgs = argh::from_env();
    if args.list_monitors {
        list_monitors(event_loop);
        return Ok(());
    }

    // Build a window with an OpenGL context
    let window_builder = WindowBuilder::new()
        .with_title(title)
        .with_app_id("demo".into())
        .with_inner_size(size)
        .with_resizable(false)
        .with_decorations(false);
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
    let mut player = Player::new("resources/music.ogg", title).context("Failed to load music")?;

    // Initialize rocket
    let mut sync = Sync::new(120., 8.);

    // Load demo content
    let mut demo = Demo::new(size.width, size.height, gl)?;

    // If release build, start the music
    #[cfg(not(debug_assertions))]
    player.play();

    event_loop.run(move |event, _, control_flow| match event {
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
            // Update sync, timing and audio related frame parameters
            *control_flow = sync.update(&mut player);

            // Render the frame
            if let Err(e) = demo.render(&mut sync) {
                panic!("{}", e);
            }

            // Display the frame
            windowed_context
                .swap_buffers()
                .expect("Failed to swap buffers");
        }
        _ => (),
    });
}
