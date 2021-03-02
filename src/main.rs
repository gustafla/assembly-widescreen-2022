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
struct DisplayConfiguration {
    title: &'static str,
    monitor: Option<usize>,
    exclusive_mode: Option<usize>,
    windowed: bool,
}

#[cfg(feature = "glutin")]
fn list_monitors() {
    let event_loop = glutin::event_loop::EventLoop::new();
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
    internal_size: Resolution,
    mut player: Player,
    mut sync: DemoSync,
    disp: DisplayConfiguration,
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

    // Build a window with an OpenGL context
    let window_builder = WindowBuilder::new()
        .with_title(disp.title)
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
                    VirtualKeyCode::R => demo
                        .reload()
                        .map_err(|e| {
                            log::error!("Failed to reload: {}", e);
                            e
                        })
                        .unwrap(),
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
fn run(internal_size: Resolution, mut player: Player, mut sync: DemoSync) -> Result<()> {
    use videocore::{bcm_host, dispmanx, image};

    // Initialize videocore rendering and get screen resolution
    bcm_host::init();
    let size = bcm_host::graphics_get_display_size(0).context("Cannot query display size")?;

    // Fill parameter structs
    let mut src = image::Rect {
        x: 0,
        y: 0,
        width: (internal_size.width as i32) << 16,
        height: (internal_size.height as i32) << 16,
    };
    let internal_aspect = internal_size.width as f32 / internal_size.height as f32;
    let display_aspect = size.width as f32 / size.height as f32;
    let scale = if internal_aspect < display_aspect {
        size.height as f32 / internal_size.height as f32
    } else {
        size.width as f32 / internal_size.width as f32
    };
    let to_monitor_width = internal_size.width as f32 * scale;
    let to_monitor_height = internal_size.height as f32 * scale;
    let remaining_width = size.width as f32 - to_monitor_width;
    let remaining_height = size.height as f32 - to_monitor_height;
    let mut dst = image::Rect {
        x: (remaining_width / 2.) as i32, // Center the picture if narrow or tall
        y: (remaining_height / 2.) as i32, // Center the picture if thin or wide
        width: to_monitor_width as i32,
        height: to_monitor_height as i32,
    };
    let mut alpha = dispmanx::VCAlpha {
        flags: dispmanx::FlagsAlpha::FixedAllPixels,
        opacity: 255,
        mask: 0,
    };

    // Open dispmanx display
    let display = dispmanx::display_open(0);
    let update = dispmanx::update_start(0);

    // Create element to show
    let element = dispmanx::element_add(
        update,
        display,
        0,
        &mut dst,
        0,
        &mut src,
        dispmanx::DISPMANX_PROTECTION_NONE,
        &mut alpha,
        None,
        dispmanx::Transform::NoRotate,
    );
    println!(
        "update_submit_sync -> {}",
        dispmanx::update_submit_sync(update)
    );

    // Fill dispmanx Window for EGL
    let mut window = dispmanx::Window {
        element,
        width: size.width as i32,
        height: size.height as i32,
    };

    // EGL
    let egl_attribs = [
        khronos_egl::RED_SIZE,
        5,
        khronos_egl::GREEN_SIZE,
        6,
        khronos_egl::BLUE_SIZE,
        5,
        khronos_egl::ALPHA_SIZE,
        8,
        khronos_egl::DEPTH_SIZE,
        8,
        khronos_egl::STENCIL_SIZE,
        8,
        khronos_egl::SAMPLE_BUFFERS,
        0,
        khronos_egl::NONE,
    ];
    let egl = khronos_egl::Instance::new(khronos_egl::Static);
    let egl_display = egl.get_display(khronos_egl::DEFAULT_DISPLAY).unwrap();
    egl.initialize(egl_display).unwrap();
    let egl_config = egl
        .choose_first_config(egl_display, &egl_attribs)
        .unwrap()
        .unwrap();
    let egl_buffer = unsafe {
        egl.create_window_surface(
            egl_display,
            egl_config,
            &mut window as *mut dispmanx::Window as khronos_egl::NativeWindowType,
            None,
        )
    }
    .unwrap();
    let egl_context_attribs = [khronos_egl::CONTEXT_CLIENT_VERSION, 2, khronos_egl::NONE];
    let egl_context = egl
        .create_context(egl_display, egl_config, None, &egl_context_attribs)
        .unwrap();
    egl.make_current(
        egl_display,
        Some(egl_buffer),
        Some(egl_buffer),
        Some(egl_context),
    )
    .unwrap();

    // Load demo content
    let mut demo = Demo::new(internal_size)?;

    // If release build, start the music
    #[cfg(not(debug_assertions))]
    {
        player.play();
    }

    loop {
        // Update sync, timing and audio related frame parameters
        if sync.update(&mut player) {
            break;
        }

        // Render the frame
        if let Err(e) = demo.render(&mut sync, internal_size) {
            panic!("{}", e);
        }

        // Display the frame
        egl.swap_buffers(egl_display, egl_buffer).unwrap();
    }

    // Deinitialize videocore
    bcm_host::deinit();

    Ok(())
}

fn main() -> Result<()> {
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

    // Process glutin-specific args
    #[cfg(feature = "glutin")]
    {
        if pargs.contains("--list-monitors") {
            list_monitors();
            return Ok(());
        }
        eprintln!("See --help if the default options don't work for you");
    }
    #[cfg(feature = "glutin")]
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
    let player = Player::new("resources/music.ogg").context("Failed to load music")?;

    // Initialize rocket
    let sync = DemoSync::new(120., 8., benchmark || cfg!(debug_assertions));

    #[cfg(feature = "glutin")]
    run(internal_size, player, sync, disp)?;
    #[cfg(feature = "rpi")]
    run(internal_size, player, sync)?;

    Ok(())
}
