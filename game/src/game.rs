mod controller;

use crate::game::controller::ControllerState;
use crate::graphics::{camera::UprightPerspectiveCamera, gpu_types::PetalVariant, GraphicsState};

use cgmath::prelude::*;
use cgmath::{Deg, Rad};
use rand::Rng;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::window::Window;

const MOVEMENT_SPEED: f32 = 0.01;
const TURN_SPEED: Rad<f32> = Rad::<f32>(std::f32::consts::PI / 180.0 / 10.0);
const N_PETALS: usize = 8;

pub struct GameState {
    /// Random number generator for this thread
    rng: rand::rngs::ThreadRng,
    /// Holds handles to GPU resources and objects in a form compatible with being passed/copied to
    /// GPU buffers/resources.
    graphics_state: GraphicsState,
    /// Tracks the state of the user input.
    controller_state: ControllerState,
    /// Camera used to render the world
    camera: UprightPerspectiveCamera,
    /// Used to enable / disable input and control whether or not the mouse is grabbed.
    game_window_focused: bool,
    mouse_look_enabled: bool,
    // Petals
    petal_poses: Vec<Pose>,
    petal_variant_indices: Vec<u32>,
}

impl GameState {
    pub async fn new(window: &Window) -> Self {
        let mut rng = rand::thread_rng();

        // -----------------------------------------------------------------------------------------
        log::debug!("Petal variants setup");
        let petal_texture_image_paths =
            vec!["game/res/pink_petals_long.png", "game/res/pink_petal.png"];
        let petal_variants = vec![
            // TODO: If I include any more of these, I exceed the max number of UniformBuffer
            // bindings that the device support (it caps out at 12).  I think this means that the
            // way I'm defining and passing the uniforms is actually creating an array of uniform
            // buffers... maybe???  I'm not really sure on that, because I though I was previously
            // passing the texture indices the same way (as an array), but that seemed to work even
            // when I set it to a very large number of petals.  This is probably going to take some
            // more research.  These appear to use one more slot for each uncommented line.  It also
            // appears that increasing N_PETALS also can break that limit, so I've changed something
            // about how I'm passing the petal indexes from before when I was able to render
            // thousands.
            // pink_petals_long.png -- contains 8 petal images
            PetalVariant::new(0, 0.000, 0.021, 0.250, 0.412),
            PetalVariant::new(0, 0.250, 0.021, 0.250, 0.412),
            PetalVariant::new(0, 0.500, 0.005, 0.253, 0.445),
            //PetalVariant::new(0, 0.751, 0.001, 0.249, 0.458),
            //PetalVariant::new(0, 0.000, 0.541, 0.251, 0.407),
            //PetalVariant::new(0, 0.250, 0.532, 0.253, 0.423),
            //PetalVariant::new(0, 0.502, 0.488, 0.253, 0.512),
            //PetalVariant::new(0, 0.767, 0.487, 0.216, 0.513),
            // pink_petal.png -- contains 1 petal image
            PetalVariant::new(1, 0.0, 0.0, 1.0, 1.0),
        ];

        // -----------------------------------------------------------------------------------------
        log::debug!("Instance setup");
        let mut petal_variant_indices = Vec::with_capacity(N_PETALS);
        let mut petal_poses = Vec::with_capacity(N_PETALS);
        for _ in 0..N_PETALS {
            // Chose a random variant for each petal instance
            petal_variant_indices.push(rng.gen_range(0..petal_variants.len() as u32));
            petal_poses.push(Pose {
                // Generate random petal positions in view of the camera -- in the [-1,1] x/y range
                // covered by NDC (normalized device coordinates).
                position: cgmath::vec3(
                    2.0 * rng.gen::<f32>() - 1.0,
                    2.0 * rng.gen::<f32>() - 1.0,
                    2.0 * rng.gen::<f32>() - 1.0,
                ),
                // Give the petal no rotation, represented by a quaternion of 1.0 real part and
                // zeros in all the imaginary dimensions.  If you think of complex numbers as
                // representing where the point 1.0 along the real axis would get rotated to if
                // operated on by that complex number, then this is basically just saying it stays
                // in the same place---thus no rotation.
                rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: 1.5 * rng.gen::<f32>() + 0.5,
            });
        }

        // -----------------------------------------------------------------------------------------
        let graphics_state = GraphicsState::new(
            window,
            &petal_texture_image_paths,
            petal_variants,
            &petal_variant_indices,
            &petal_poses,
            true,
        )
        .await;
        let controller_state = ControllerState::new();

        // -----------------------------------------------------------------------------------------
        log::debug!("Camera setup");
        // Place the camera out a ways on the +z axis (out of the screen according to NDCs) so it
        // can view objects placed around the origin when looking in the -z direction.  This way we
        // should have a similar view of things that we orignally rendered directly in NDCs without
        // having to change their coordinates.
        let camera_location = cgmath::Point3::<f32>::new(0.0, 0.0, 4.0);
        // Turn the camera 90 degrees to the left (ccw around the y axis pointing up) to face in the
        // -z direction, thus matching normalized device coordinates.  Note that the camera is
        // defined such that pan and tilt angles of 0 mean the camera is pointing the same direction
        // as the +x axis.
        let camera_pan = Rad::<f32>(0.0); //cgmath::Rad::<f32>::turn_div_4();
        let camera_tilt = Rad::<f32>(0.0);
        let camera_fov_y = Rad::<f32>::from(Deg::<f32>(60.0));
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

        // -----------------------------------------------------------------------------------------
        Self {
            rng,
            graphics_state,
            controller_state,
            camera,
            petal_poses,
            petal_variant_indices,
            game_window_focused: false,
            mouse_look_enabled: true,
        }
    }

    /// Handles the passed event if possible, and returns a boolean value indicating if the event
    /// was handled or not.
    pub fn handle_window_event(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                self.controller_state.handle_keyboard_event(input)
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                self.mouse_look_enabled = !self.mouse_look_enabled;
                if self.game_window_focused {
                    window.set_cursor_visible(!self.mouse_look_enabled);
                    window.set_cursor_grab(self.mouse_look_enabled).unwrap();
                }
                // Clear any pan / tilt that has been accumulated to avoid sudden jumps in rotation
                // when mouse look is re-enabled.
                self.controller_state.get_pan_tilt_delta();
                true
            }
            WindowEvent::Focused(focused) => {
                self.game_window_focused = *focused;
                if self.mouse_look_enabled {
                    window.set_cursor_visible(!self.game_window_focused);
                    window.set_cursor_grab(self.game_window_focused).unwrap();
                }
                // Clear any pan / tilt that has been accumulated to avoid sudden jumps in rotation
                // when focus is regained.
                self.controller_state.get_pan_tilt_delta();
                true
            }
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

    /// Handles the passed event if possible.
    pub fn handle_device_event(&mut self, event: &DeviceEvent) {
        self.controller_state.handle_device_event(event);
    }

    pub fn update(&mut self) {
        // Game state update code goes here.

        if self.game_window_focused {
            self.update_based_on_controller_state();
        }

        // Rotate all the petals a bit each frame to test changing the instance pose buffer
        for pose in &mut self.petal_poses {
            pose.rotation = cgmath::Quaternion::from_angle_y(cgmath::Rad(0.03)) * pose.rotation;
        }

        // Update GPU buffers according to the current game state.
        self.graphics_state.update(&self.camera, &self.petal_poses);
    }

    fn update_based_on_controller_state(&mut self) {
        self.camera.move_relative_to_pan_angle(
            MOVEMENT_SPEED * self.controller_state.forward_multiplier(),
            MOVEMENT_SPEED * self.controller_state.right_muliplier(),
            MOVEMENT_SPEED * self.controller_state.jump_multiplier(),
        );
        if self.mouse_look_enabled {
            let (pan_delta, tilt_delta) = self.controller_state.get_pan_tilt_delta();
            self.camera
                .pan_and_tilt(TURN_SPEED * pan_delta, TURN_SPEED * tilt_delta)
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.graphics_state.render()
    }

    /// Attempt to reconfigure / reacquire the rendering surface using the last known window size.
    pub fn reconfigure_rendering_surface(&mut self) {
        self.graphics_state.resize(self.graphics_state.size)
    }
}

pub struct Pose {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    scale: f32,
}

impl From<&Pose> for crate::graphics::gpu_types::Matrix4 {
    fn from(pose: &crate::game::Pose) -> Self {
        crate::graphics::gpu_types::Matrix4 {
            matrix: (cgmath::Matrix4::from_translation(pose.position)
                * cgmath::Matrix4::from(pose.rotation)
                * cgmath::Matrix4::from_scale(pose.scale))
            .into(),
        }
    }
}
