use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub async fn run() {
    // Window setup
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Event loop
    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => match event {
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
            },
            Event::MainEventsCleared => {
                // Application update code goes here

                // Continually request redraws by calling request_redraw() in response to this
                // event.  Or could just render here instead for things like games that are
                // continuously redrawing (as mentioned by the documentation).
                window.request_redraw();
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {}
            _ => {}
        }
    });
}
