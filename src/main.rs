mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{DemoSync, Player, Renderer};
use pico_args::Arguments;
use rand::prelude::*;
use rand_xoshiro::Xoshiro128Plus;
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
    -s, --scale         Set the rendering scale (default 1.0)
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
    size: PhysicalSize<u32>,
    scale: f32,
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
        .with_inner_size(size)
        .with_fullscreen(fullscreen)
        .with_decorations(false);

    #[cfg(not(debug_assertions))]
    let window_builder = window_builder.with_resizable(false);

    #[cfg(target_family = "unix")]
    let window_builder = window_builder.with_app_id("demo".into());

    let window = window_builder
        .build(&event_loop)
        .context("Failed to build a window")?;

    // Initialize demo render data
    let mut rng = Xoshiro128Plus::seed_from_u64(0);
    let (mut state, models) = demo::State::new(&mut rng);

    // Initialize Renderer for window
    let internal_size = PhysicalSize::new(
        (size.width as f32 * scale) as u32,
        (size.height as f32 * scale) as u32,
    );
    let mut renderer = pollster::block_on(Renderer::new(internal_size, &window, models, &mut rng))?;

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
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode, ..
                        },
                    ..
                } => match virtual_keycode {
                    Some(VirtualKeyCode::Q | VirtualKeyCode::Escape) => {
                        *control_flow = ControlFlow::Exit
                    }
                    _ => {}
                },
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(physical_size);
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(*new_inner_size);
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                // Update sync, timing and audio related frame parameters
                if sync.update(&mut player) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                // Create the frame scene
                let scene = state.update(&mut rng, &mut sync);

                // Render the scene
                match renderer.render(&mut rng, scene) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => renderer.configure_surface(),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => log::error!("{:?}", e),
                }
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
    let scale = pargs.opt_value_from_str(["-s", "--scale"])?.unwrap_or(1.);
    if !(0.1..=2.).contains(&scale) {
        return Err(anyhow!("Scale must be from 0.1 to 2.0"));
    }
    eprintln!("See --help if the default options don't work for you");

    let size = PhysicalSize::new(3840, 768);
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

    run(size, scale, player, sync, disp)?;

    Ok(())
}
