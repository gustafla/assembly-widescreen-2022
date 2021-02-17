mod logger;

use anyhow::{anyhow, Context, Result};
use demo::{Demo, Player, RcGl, Sync};
use glutin::{
    dpi::PhysicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::unix::{EventLoopWindowTargetExtUnix, WindowBuilderExtUnix},
    window::{Fullscreen, WindowBuilder},
    Api, ContextBuilder, GlRequest,
};
use std::convert::TryInto;

#[derive(Debug, Default)]
struct FullscreenOptions {
    monitor: Option<usize>,
    mode: usize,
}

impl FullscreenOptions {
    fn configure<T>(self, event_loop: &EventLoop<T>) -> Result<Fullscreen> {
        let monitor = match self.monitor {
            Some(monitor_id) => Some(
                event_loop
                    .available_monitors()
                    .nth(monitor_id)
                    .context("Requested monitor does not exist")?,
            ),
            None => None,
        };

        // Use borderless fullscreen on Wayland
        if event_loop.is_wayland() {
            return Ok(Fullscreen::Borderless(monitor));
        }

        // Use exclusive fullscreen on other platforms
        Ok(Fullscreen::Exclusive(
            monitor
                .unwrap_or_else(|| {
                    event_loop.primary_monitor().unwrap_or_else(|| {
                        log::info!("Can't determine primary monitor, using first");
                        event_loop
                            .available_monitors()
                            .next()
                            .expect("Can't enumerate monitors")
                    })
                })
                .video_modes()
                .nth(self.mode)
                .context("Requested video mode does not exist")?,
        ))
    }
}

impl std::str::FromStr for FullscreenOptions {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.splitn(2, ',');

        Ok(Self {
            monitor: split.next().map(|s| s.parse()).transpose()?,
            mode: split.next().map(|s| s.parse().unwrap_or(0)).unwrap_or(0),
        })
    }
}

fn print_monitors<T>(event_loop: EventLoop<T>) {
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
    let default_size = PhysicalSize::new(1280, 720);
    let event_loop = EventLoop::new();

    // Process CLI
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains("--list-monitors") {
        print_monitors(event_loop);
        return Ok(());
    }

    // Parse fullscreen options with fallback
    let fullscreen: Option<FullscreenOptions> = pargs
        .opt_value_from_str("--fullscreen")
        .unwrap_or_else(|_| Some(Default::default()));

    // Fullscreen options to winit configuration
    let fullscreen = match fullscreen {
        Some(fullscreen_opt) => Some(fullscreen_opt.configure(&event_loop)?),
        None => None,
    };

    // Build a window with an OpenGL context
    let window_builder = WindowBuilder::new()
        .with_title(title)
        .with_app_id("demo".into())
        .with_inner_size(match &fullscreen {
            Some(Fullscreen::Borderless(Some(monitor))) => monitor.size(),
            Some(Fullscreen::Exclusive(video_mode)) => video_mode.size(),
            _ => default_size,
        })
        .with_fullscreen(fullscreen.clone())
        .with_resizable(false)
        .with_decorations(false);
    let windowed_context = ContextBuilder::new()
        .with_gl(GlRequest::Specific(Api::OpenGlEs, (2, 0)))
        .with_vsync(true)
        .with_srgb(true)
        .with_hardware_acceleration(None)
        .build_windowed(window_builder, &event_loop)
        .context("Failed to build a window")?;

    // Correct borderless fullscreen inner size
    if let Some(Fullscreen::Borderless(None)) = fullscreen {
        if let Some(monitor) = windowed_context.window().current_monitor() {
            windowed_context.window().set_inner_size(monitor.size());
        }
    }

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
    let size = windowed_context.window().inner_size();
    let mut demo = Demo::new(
        size.width.try_into().unwrap(),
        size.height.try_into().unwrap(),
        gl,
    )?;

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
