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

struct MonitorId(Option<usize>);

impl std::str::FromStr for MonitorId {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Some(s.parse()?)))
    }
}

fn main() -> Result<()> {
    // Initialize logging
    log::set_logger(&logger::Logger).unwrap();
    log::set_max_level(log::LevelFilter::max());

    // Initialize window stuff
    let title = "Demo";
    let event_loop = EventLoop::new();

    // Process CLI
    let mut pargs = pico_args::Arguments::from_env();
    let monitor: Option<MonitorId> = pargs
        .opt_value_from_str("--fullscreen")
        .unwrap_or(Some(MonitorId(None)));
    let internal_size = PhysicalSize::new(
        pargs.value_from_str(["-w", "--width"]).unwrap_or(1280),
        pargs.value_from_str(["-h", "--height"]).unwrap_or(720),
    );

    // Configure fullscreen for the specified monitor
    let fullscreen = match monitor {
        Some(MonitorId(Some(id))) => Some(Fullscreen::Borderless(Some(
            event_loop
                .available_monitors()
                .nth(id)
                .context("Requested monitor doesn't exist")?,
        ))),
        Some(_) => Some(Fullscreen::Borderless(None)),
        _ => None,
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
