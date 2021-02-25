mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{Demo, DemoSync, Player, Resolution};
use pico_args::Arguments;

fn print_help() {
    print!(
        r#"List of available options:
    --help              Print this help
    --benchmark         Log frametimes
    -w, --width         Set the rendering width (default 1280)
    -h, --height        Set the rendering height (default 720)
"#
    );

    #[cfg(feature = "glutin")]
    print!(
        r#"    --list-monitors     List available monitors and video modes
    --monitor id        Specify a monitor to use in fullscreen
    --exclusive mode    Exclusive fullscreen (see --list-monitors for modes)
    --windowed          Don't go fullscreen

To force X11 or Wayland, set the environment variable
WINIT_UNIX_BACKEND to x11 or wayland.
"#
    );
}

#[cfg(feature = "glutin")]
fn list_monitors<T>(event_loop: glutin::event_loop::EventLoop<T>) {
    for (i, monitor) in event_loop.available_monitors().enumerate() {
        println!("Monitor {}", i);
        for (j, mode) in monitor.video_modes().enumerate() {
            println!(
                "   Mode {}: {}x{} {}-bit {}Hz",
                j,
                mode.size().width,
                mode.size().height,
                mode.bit_depth(),
                mode.refresh_rate()
            );
        }
    }
}

#[cfg(feature = "glutin")]
fn run(
    title: &str,
    mut pargs: Arguments,
    internal_size: Resolution,
    mut player: Player,
    mut sync: DemoSync,
) -> Result<()> {
    use glutin::{
        event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::unix::WindowBuilderExtUnix,
        window::{Fullscreen, WindowBuilder},
        Api, ContextBuilder, GlRequest,
    };

    // Initialize winit
    let event_loop = EventLoop::new();

    // Process glutin-specific args
    if pargs.contains("--list-monitors") {
        list_monitors(event_loop);
        return Ok(());
    }
    eprintln!("See --help if the default options don't work for you");
    let monitor: Option<usize> = pargs.opt_value_from_str("--monitor")?;
    let exclusive_mode: Option<usize> = pargs.opt_value_from_str("--exclusive")?;
    let windowed = pargs.contains("--windowed");
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        return Err(anyhow!("Unknown arguments {:?}", remaining));
    }

    // Select fullscreen mode
    let fullscreen = if !(windowed || cfg!(debug_assertions)) {
        match (monitor, exclusive_mode) {
            (Some(id), Some(mode)) => Some(Fullscreen::Exclusive(
                event_loop
                    .available_monitors()
                    .nth(id)
                    .context("Requested monitor doesn't exist")?
                    .video_modes()
                    .nth(mode)
                    .context("Requested mode doesn't exist")?,
            )),
            (None, Some(mode)) => Some(Fullscreen::Exclusive(
                event_loop
                    .primary_monitor()
                    .context("Cannot determine primary monitor, use --monitor to choose manually")?
                    .video_modes()
                    .nth(mode)
                    .context("Requested mode doesn't exist")?,
            )),
            (Some(id), None) => Some(Fullscreen::Borderless(Some(
                event_loop
                    .available_monitors()
                    .nth(id)
                    .context("Requested monitor doesn't exist")?,
            ))),
            (None, None) => Some(Fullscreen::Borderless(None)),
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

    // Load demo content
    let mut demo = Demo::new(internal_size)?;

    // If release build, start the music and hide the cursor
    #[cfg(not(debug_assertions))]
    {
        windowed_context.window().set_cursor_visible(false);
        player.play();
    }

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
                    #[cfg(debug_assertions)]
                    VirtualKeyCode::R => demo.reload().unwrap(),
                    _ => (),
                },
                WindowEvent::Resized(size) => {
                    windowed_context.resize(size);
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                // Update sync, timing and audio related frame parameters
                if sync.update(&mut player) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

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
        }
    })
}

#[cfg(feature = "rpi")]
fn run(
    title: &str,
    mut pargs: Arguments,
    internal_size: Resolution,
    mut player: Player,
    mut sync: DemoSync,
) -> Result<()> {
    use videocore::bcm_host;

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        return Err(anyhow!("Unknown arguments {:?}", remaining));
    }

    // Initialize videocore rendering and get screen resolution
    bcm_host::init();
    let display_size =
        bcm_host::graphics_get_display_size(0).context("Cannot query display size")?;
    println!("Display is {}x{}", display_size.width, display_size.height);

    // Test the player
    player.play();
    std::thread::sleep(std::time::Duration::new(20, 0));

    // Deinitialize videocore
    bcm_host::deinit();
    todo!()
}

fn main() -> Result<()> {
    let title = "Demo";

    // Process CLI
    let mut pargs = Arguments::from_env();
    if pargs.contains("--help") {
        print_help();
        return Ok(());
    }
    let benchmark = pargs.contains("--benchmark");
    let internal_size = Resolution {
        width: pargs.opt_value_from_str(["-w", "--width"])?.unwrap_or(1280),
        height: pargs.opt_value_from_str(["-h", "--height"])?.unwrap_or(720),
    };
    for &size in &[internal_size.width, internal_size.height] {
        if size < 1 || size > 2u32.pow(14) {
            return Err(anyhow!("Size cannot be {}", size));
        }
    }

    // Initialize logging
    log::set_logger(&logger::Logger).unwrap();
    log::set_max_level(log::LevelFilter::max());

    // Load music
    let player = Player::new("resources/music.ogg").context("Failed to load music")?;

    // Initialize rocket
    let sync = DemoSync::new(120., 8., benchmark || cfg!(debug_assertions));

    run(title, pargs, internal_size, player, sync)
}
