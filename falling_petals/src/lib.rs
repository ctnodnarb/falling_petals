//mod ecs;
mod falling_petals;
mod graphics;
mod input;

use cgmath::{Deg, Rad};

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub fn run() {
    // Window setup
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Initialize the game
    let game_config = falling_petals::FallingPetalsConfig {
        n_petals: 7000,
        min_scale: 1.0,
        max_scale: 2.0,
        fall_speed: 0.1,
        camera_near: 1.0,
        camera_far: 100.0,
        camera_fov_y: Rad::<f32>::from(Deg::<f32>(60.0)),
        // Set the boundaries of the rectangular prism in which the petals are rendered so that
        // they are not visible in the view frustum (at its default location).
        // For a 60fovy frustum with 100 view depth and 1920x1080 aspect ratio, we need max_x > 103.
        max_x: 110.0,
        // For a 60fovy frustum with 100 view depth, we need max_y > 58.
        max_y: 65.0,
        // Note that max_z is doubled (goes negative and positive the same as max_x and max_y) in
        // determining the total volume in which the petals are rendered.  So the camera's view
        // depth gets set to double this value.
        max_z: 50.0,
        player_movement_speed: 0.5,
        player_turn_speed: Rad::<f32>(std::f32::consts::PI / 180.0 / 10.0),
        movement_period: 60 * 15,
        movement_max_freq: 60,
        movement_amplitude_min: 0.015,
        movement_amplitude_max: 0.075,
        min_rotation_speed: Deg::<f32>(1.0),
        max_rotation_speed: Deg::<f32>(3.0),
    };
    let video_fps = 30;
    let video_export_config = crate::graphics::VideoExportConfig::new(
        1920,
        1080,
        video_fps,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    );
    let mut game_state =
        falling_petals::FallingPetalsState::new(&window, game_config, video_export_config);

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
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            _ => {}
        }
    });
}
