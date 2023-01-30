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
    let config_str;
    if !config_path.exists() {
        println!("No config.toml file found in current directory.");
        println!("Generating a default config.toml file and exiting...");
        let config = configuration::FallingPetalsConfig::default();
        match toml::to_string(&config) {
            Ok(serialized_config) => config_str = serialized_config,
            Err(error) => {
                println!("Error generating default config: {error}");
                return;
            }
        }
        if let Err(error) = std::fs::write(config_path, config_str) {
            println!("Error writing default config.toml file: {error}");
            return;
        }
        println!("Default config.toml generated.  Edit it if desired and run the program again to use it.");
        return;
    }
    match std::fs::read_to_string(config_path) {
        Ok(file_contents) => config_str = file_contents,
        Err(error) => {
            println!("Error reading config.toml: {error}");
            return;
        }
    }
    let config;
    match toml::from_str(&config_str) {
        Ok(parsed_config) => config = parsed_config,
        Err(error) => {
            println!("Error parsing config.toml: {error}");
            println!("Rename or remove config.toml and rerun to generate a new config.toml with default settings.");
            return;
        }
    }

    // Window setup
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let video_fps = 30;
    let video_export_config = crate::graphics::VideoExportConfig::new(
        1920,
        1080,
        video_fps,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let mut game_state = state::FallingPetalsState::new(&window, config, video_export_config);

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::DeviceEvent { ref event, .. } => {
                game_state.handle_device_event(event);
            }
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => {
                if !game_state.handle_window_event(event, &window) {
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
                game_state.update();

                // Continually request redraws by calling request_redraw() in response to this
                // event.  Or could just render here instead for things like games that are
                // continuously redrawing (as mentioned by the documentation).
                window.request_redraw();
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                match game_state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => game_state.reconfigure_rendering_surface(),
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        eprintln!("Exiting due to wgpu::SurfaceError::OutOfMemory");
                        *control_flow = ControlFlow::Exit
                    }
                    Err(e) => eprintln!("{e:?}"),
                }
            }
            _ => {}
        }
    });
}
