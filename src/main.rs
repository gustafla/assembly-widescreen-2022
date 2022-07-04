mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{Demo, DemoSync, Player};
use pico_args::Arguments;
#[cfg(target_family = "unix")]
use winit::platform::unix::WindowBuilderExtUnix;
use winit::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, WindowBuilder},
};

fn print_help() {
    print!(
        r#"List of available options:
    --help              Print this help
    --benchmark         Log frametimes
    -w, --width         Set the rendering width (default 1280)
    -h, --height        Set the rendering height (default 720)
    --list-monitors     List available monitors and video modes
    --monitor id        Specify a monitor to use in fullscreen
    --exclusive mode    Exclusive fullscreen (see --list-monitors for modes)
    --windowed          Don't go fullscreen

To force X11 or Wayland, set the environment variable
WINIT_UNIX_BACKEND to x11 or wayland.
"#
    );
}

struct DisplayConfiguration {
    title: &'static str,
    monitor: Option<usize>,
    exclusive_mode: Option<usize>,
    windowed: bool,
}

fn list_monitors() {
    let event_loop = winit::event_loop::EventLoop::new();
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

fn run(
    internal_size: PhysicalSize<u32>,
    mut player: Player,
    mut sync: DemoSync,
    disp: DisplayConfiguration,
) -> Result<()> {
    // Initialize winit
    let event_loop = EventLoop::new();

    // Select fullscreen mode
    let fullscreen = if !(disp.windowed || cfg!(debug_assertions)) {
        match (disp.monitor, disp.exclusive_mode) {
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

    // Build a Window
    let window_builder = WindowBuilder::new()
        .with_title(disp.title)
        .with_inner_size(internal_size)
        .with_fullscreen(fullscreen)
        .with_decorations(false);

    #[cfg(not(debug_assertions))]
    let window_builder = window_builder.with_resizable(false);

    #[cfg(target_family = "unix")]
    let window_builder = window_builder.with_app_id("demo".into());

    let window = window_builder
        .build(&event_loop)
        .context("Failed to build a window")?;

    // Load demo content
    let mut demo = Demo::new();

    // If release build, start the music and hide the cursor
    #[cfg(not(debug_assertions))]
    {
        player.play();
        window.set_cursor_visible(false);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape | VirtualKeyCode::Q),
                            ..
                        },
                    ..
                } => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(_size) => {
                    //windowed_context.resize(size);
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
                /*if let Err(e) = demo.render(&mut sync, windowed_context.window().inner_size()) {
                    panic!("{}", e);
                }*/

                // Display the frame
                /*windowed_context
                .swap_buffers()
                .expect("Failed to swap buffers");*/
            }
            _ => (),
        }
    })
}

fn main() -> Result<()> {
    // Process CLI
    let mut pargs = Arguments::from_env();
    if pargs.contains("--help") {
        print_help();
        return Ok(());
    }
    if pargs.contains("--list-monitors") {
        list_monitors();
        return Ok(());
    }
    let benchmark = pargs.contains("--benchmark");
    let internal_size = PhysicalSize::new(
        pargs.opt_value_from_str(["-w", "--width"])?.unwrap_or(1280),
        pargs.opt_value_from_str(["-h", "--height"])?.unwrap_or(720),
    );
    for &size in &[internal_size.width, internal_size.height] {
        if size < 1 || size > 2u32.pow(14) {
            return Err(anyhow!("Size cannot be {}", size));
        }
    }
    eprintln!("See --help if the default options don't work for you");

    let disp = DisplayConfiguration {
        title: "Demo",
        monitor: pargs.opt_value_from_str("--monitor")?,
        exclusive_mode: pargs.opt_value_from_str("--exclusive")?,
        windowed: pargs.contains("--windowed"),
    };

    // Finish args
    let remaining = pargs.finish();
    if !remaining.is_empty() {
        return Err(anyhow!("Unknown arguments {:?}", remaining));
    }

    // Initialize logging
    log::set_logger(&logger::Logger)
        .map_err(|e| eprintln!("{}\nFailed to initialize logger. Going without.", e))
        .ok();
    log::set_max_level(log::LevelFilter::max());

    // Load music
    let player = Player::new("music.ogg");

    // Initialize rocket
    let sync = DemoSync::new(120., 8., benchmark || cfg!(debug_assertions));

    run(internal_size, player, sync, disp)?;

    Ok(())
}
