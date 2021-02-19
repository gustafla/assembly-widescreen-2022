mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{Demo, Player, RcGl, Sync};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::unix::WindowBuilderExtUnix,
    window::{Fullscreen, WindowBuilder},
    Api, ContextBuilder, GlRequest,
};

fn print_help() {
    print!(
        r#"List of available options:
    --help          Print this help
    --monitor id    Specify a monitor to use in fullscreen, 0-based
    --windowed      Don't go fullscreen
    -w, --width     Set the rendering width (default 1280)
    -h, --height    Set the rendering height (default 720)

To force X11 or Wayland, set the environment variable
WINIT_UNIX_BACKEND to x11 or wayland.
"#
    );
}

fn main() -> Result<()> {
    // Process CLI
    let mut pargs = pico_args::Arguments::from_env();
    if pargs.contains("--help") {
        print_help();
        return Ok(());
    }
    eprintln!("See --help if the default options don't work for you");
    let monitor: Option<usize> = pargs.opt_value_from_str("--monitor")?;
    let windowed: bool = pargs.contains("--windowed");
    let internal_size = PhysicalSize::new(
        pargs.opt_value_from_str(["-w", "--width"])?.unwrap_or(1280),
        pargs.opt_value_from_str(["-h", "--height"])?.unwrap_or(720),
    );
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        return Err(anyhow!("Unknown arguments {:?}", remaining));
    }
    for &size in &[internal_size.width, internal_size.height] {
        if size < 1 || size > 2u32.pow(14) {
            return Err(anyhow!("Size cannot be {}", size));
        }
    }

    // Initialize logging
    log::set_logger(&logger::Logger).unwrap();
    log::set_max_level(log::LevelFilter::max());

    // Initialize window stuff
    let title = "Demo";
    let event_loop = EventLoop::new();
    let fullscreen = if !(windowed || cfg!(debug_assertions)) {
        match monitor {
            Some(id) => Some(Fullscreen::Borderless(Some(
                event_loop
                    .available_monitors()
                    .nth(id)
                    .context("Requested monitor doesn't exist")?,
            ))),
            None => Some(Fullscreen::Borderless(None)),
        }
    } else {
        None
    };

    // Build a window with an OpenGL context
    let window_builder = WindowBuilder::new()
        .with_title(title)
        .with_app_id("demo".into())
        .with_inner_size(internal_size)
        .with_fullscreen(fullscreen)
        .with_decorations(false);
    let windowed_context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
        .with_vsync(true)
        .with_srgb(true)
        .with_hardware_acceleration(None)
        .build_windowed(window_builder, &event_loop)
        .context("Failed to build a window")?;

    // Make OpenGL context current
    let windowed_context = unsafe { windowed_context.make_current() }
        .map_err(|e| anyhow!("Failed to make context current: {:?}", e))?;

    // Load OpenGL interface
    let gl = RcGl::new(|s| windowed_context.get_proc_address(s));

    // Load music
    let mut player = Player::new("resources/music.ogg", title).context("Failed to load music")?;

    // Initialize rocket
    let mut sync = Sync::new(120., 8.);

    // Load demo content
    let mut demo = Demo::new(internal_size, gl)?;

    // If release build, start the music and hide the cursor
    #[cfg(not(debug_assertions))]
    {
        windowed_context.window().set_cursor_visible(false);
        player.play();
    }

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
            WindowEvent::Resized(size) => {
                windowed_context.resize(size);
            }
            _ => (),
        },
        Event::MainEventsCleared => {
            // Update sync, timing and audio related frame parameters
            *control_flow = sync.update(&mut player);

            // Render the frame
            if let Err(e) = demo.render(&mut sync, windowed_context.window().inner_size()) {
                panic!("{}", e);
            }

            // Display the frame
            windowed_context
                .swap_buffers()
                .expect("Failed to swap buffers");
        }
        _ => (),
    })
}
