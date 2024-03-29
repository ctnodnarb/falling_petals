//mod ecs;
mod configuration;
mod graphics;
mod input;
mod state;

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn run() {
    // Load or generate config file
    let config_path = std::path::Path::new("config.toml");
    if !config_path.exists() {
        println!("No config.toml file found in current directory.");
        println!("Generating a default config.toml file and exiting...");
        if let Err(error) = std::fs::write(config_path, configuration::DEFAULT_CONFIG_STR) {
            println!("Error writing default config.toml file: {error}");
            return;
        }
        println!("Default config.toml generated.  Edit it if desired and run the program again to use it.");
        return;
    }
    let config_str = match std::fs::read_to_string(config_path) {
        Ok(file_contents) => file_contents,
        Err(error) => {
            println!("Error reading config.toml: {error}");
            return;
        }
    };
    let config: configuration::FallingPetalsConfig = match toml::from_str(&config_str) {
        Ok(parsed_config) => parsed_config,
        Err(error) => {
            println!("Error parsing config.toml: {error}");
            println!("Rename or remove config.toml and rerun to generate a new config.toml with default settings.");
            return;
        }
    };

    // Window setup
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let video_export_config = crate::configuration::VideoExportConfig::new(
        config.enable_ffmpeg_video_export,
        config.video_export_file.clone(),
        config.video_export_width,
        config.video_export_height,
        config.video_export_fps,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let mut simulation_state = state::FallingPetalsState::new(&window, config, video_export_config);

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                simulation_state.handle_device_event(event);
            }
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => {
                if !simulation_state.handle_window_event(event, &window) {
                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    }
                }
            }
            Event::MainEventsCleared => {
                // Application update code goes here
                // Update buffers with any new data from the game state.
                simulation_state.update();

                // Limit the framerate, if needed
                simulation_state.previous_time = simulation_state.current_time;
                let current_time = std::time::Instant::now();
                if simulation_state.config.enable_frame_rate_limit {
                    let min_frame_time =
                        std::time::Duration::new(1, 0) / simulation_state.config.frame_rate_limit;
                    if current_time - simulation_state.previous_time < min_frame_time {
                        // We rendered faster than the frame rate cap.  Compute when the next frame
                        // should be shown, and busy-wait until it's time to render.
                        simulation_state.current_time =
                            simulation_state.previous_time + min_frame_time;
                        while std::time::Instant::now() < simulation_state.current_time {}
                    } else {
                        // We were too slow to keep up with the frame rate cap for this frame, but
                        // we don't want to accumulate any deficit time going forward (i.e. don't
                        // render future frames faster than the limit in order to catch back up).
                        // So just set this frame's time to the actual current time.
                        simulation_state.current_time = current_time;
                    }
                }

                // Continually request redraws by calling request_redraw() in response to this
                // event.  Or could just render here instead for things like games that are
                // continuously redrawing (as mentioned by the documentation).
                window.request_redraw();
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => match simulation_state
                .render()
            {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => simulation_state.reconfigure_rendering_surface(),
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    eprintln!("Exiting due to wgpu::SurfaceError::OutOfMemory");
                    *control_flow = ControlFlow::Exit
                }
                Err(e) => eprintln!("{e:?}"),
            },
            _ => {}
        }
    });
}
