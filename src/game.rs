mod controller;

use crate::graphics::{camera::UprightPerspectiveCamera, GraphicsState};

use cgmath::prelude::*;
use winit::event::{KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::window::Window;

pub struct GameState {
    graphics_state: GraphicsState,
    camera: UprightPerspectiveCamera,
}

impl GameState {
    pub async fn new(window: &Window) -> Self {
        let graphics_state = GraphicsState::new(window).await;

        log::debug!("Camera setup");
        // Place the camera out a ways on the +z axis (out of the screen according to NDCs) so it
        // can view objects placed around the origin when looking in the -z direction.  This way we
        // should have a similar view of things that we orignally rendered directly in NDCs without
        // having to change their coordinates.
        let camera_location = cgmath::Point3::<f32>::new(0.0, 0.0, 10.0);
        // Turn the camera 90 degrees to the left (ccw around the y axis pointing up) to face in the
        // -z direction, thus matching normalized device coordinates.  Note that the camera is
        // defined such that pan and tilt angles of 0 mean the camera is pointing the same direction
        // as the +x axis.
        let camera_pan = cgmath::Rad::<f32>::turn_div_4();
        let camera_tilt = cgmath::Rad::<f32>(0.0);
        let camera_fov_y = cgmath::Rad::<f32>::from(cgmath::Deg::<f32>(60.0));
        let camera_z_near = 0.1;
        let camera_z_far = 100.0;
        let camera = UprightPerspectiveCamera::new(
            camera_location,
            camera_pan,
            camera_tilt,
            camera_fov_y,
            graphics_state.get_aspect_ratio(),
            camera_z_near,
            camera_z_far,
        );

        Self {
            graphics_state,
            camera,
        }
    }

    /// Handles the passed event if possible, and returns a boolean value indicating if the event
    /// was handled or not.
    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state,
                        virtual_keycode: Some(keycode),
                        ..
                    },
                ..
            } => match keycode {
                VirtualKeyCode::W | VirtualKeyCode::Up => {
                    self.camera.move_relative_to_pan_angle(0.2, 0.0, 0.0);
                    true
                }
                VirtualKeyCode::A | VirtualKeyCode::Down => {
                    self.camera.move_relative_to_pan_angle(-0.2, 0.0, 0.0);
                    true
                }
                VirtualKeyCode::S | VirtualKeyCode::Left => {
                    self.camera
                        .pan_and_tilt(cgmath::Rad::<f32>(0.1), cgmath::Rad::<f32>(0.0));
                    true
                }
                VirtualKeyCode::D | VirtualKeyCode::Right => {
                    self.camera
                        .pan_and_tilt(cgmath::Rad::<f32>(-0.1), cgmath::Rad::<f32>(0.0));
                    true
                }
                _ => false,
            },
            WindowEvent::Resized(physical_size) => {
                self.graphics_state.resize(*physical_size);
                true
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                self.graphics_state.resize(**new_inner_size);
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self) {
        // Game state update code goes here.

        // Update GPU buffers according to the current game state.
        self.graphics_state
            .update(self.camera.get_view_projection_matrix().into());
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.graphics_state.render()
    }

    /// Attempt to reconfigure / reacquire the rendering surface using the last known window size.
    pub fn reconfigure_rendering_surface(&mut self) {
        self.graphics_state.resize(self.graphics_state.size)
    }
}
