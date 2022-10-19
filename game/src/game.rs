mod controller;

use crate::game::controller::ControllerState;
use crate::graphics::{camera::UprightPerspectiveCamera, gpu_types::PetalVariant, GraphicsState};

use cgmath::prelude::*;
use cgmath::{Deg, Rad};
use noise::{NoiseFn, Seedable};
use rand::prelude::*;
use rand_distr::StandardNormal;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::window::Window;

const MOVEMENT_SPEED: f32 = 0.04;
const TURN_SPEED: Rad<f32> = Rad::<f32>(std::f32::consts::PI / 180.0 / 10.0);
// The max value that can be used (currently) for N_PETALS is 4096.  This is because the max uniform
// buffer binding size is 64KB (65536 bytes), thus limiting the number of variant indices (u32) to
// 4096 since each one is padded out to 16 bytes.  I can probably quadruple this if I pack 4 indices
// into each element (struct) of the array in the uniform buffer, but then indexing would be
// slightly more complex (would have to calculate the index of the struct, then which of the 4
// indexes inside the struct to use).  All the above is because uniform buffers don't allow array
// fields with stride less than 16.
// TODO: Maybe the above is not entirely true?  I ran into the 16 byte minimum stride I think when
// I was still mistakenly passing an array of uniform buffers.  Maybe now that I've switched it to
// pass an array inside a uniform buffer, I might be able to get rid of the padding and just send
// a densely packed array of u32s?

// N_PETALS must be a multiple of 4 due to how I'm assuming on the shader side that the petal
// variant indexes are packed into an array of vec4<u32>s.  If it is not a multiple of 4, then the
// buffer size sent from the CPU will not match the expected buffer size on the shader/GPU side,
// which will cause a crash due to a validation error.  Packing the indexes into vec4s allows me to
// fit 4 times as many of them into a uniform buffer (which has a max size of 64k or 65536 bytes on
// my GPU) than I otherwise would be able to.  With that uniform buffer size limit, the max number
// of petals that can be rendered is 16384.
const N_PETALS: usize = 3000;
const MAX_DISPLACEMENT: f32 = 50.0;
const FALL_SPEED: f32 = 0.1;
const VELOCITY_DECAY: f32 = 0.999;
const PER_PETAL_ACCELERATION: f32 = 0.000;
const WIND_ACCELERATION: f32 = 0.005;

pub struct GameState {
    /// Random number generator for this thread
    rng: rand::rngs::ThreadRng,
    /// Game start time
    start_time: std::time::Instant,
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
    petal_velocities: Vec<cgmath::Vector3<f32>>,
    wind_velocity: cgmath::Vector3<f32>,
    // Random noise generator
    noise_generator: noise::Perlin,
}

impl GameState {
    pub async fn new(window: &Window) -> Self {
        let mut rng = rand::thread_rng();

        // -----------------------------------------------------------------------------------------
        log::debug!("Petal variants setup");
        let petal_texture_image_paths = vec![
            "game/res/pink_petals_long.png",
            "game/res/pink_petal.png",
            "game/res/pink_petals_short.png",
            "game/res/purple_petals.png",
            "game/res/red_petal.png",
            "game/res/red_petals.png",
            "game/res/rose_petals.png",
        ];
        let petal_variants = vec![
            // pink_petals_long.png -- contains 8 petal images
            PetalVariant::new(0, 0.000, 0.021, 0.250, 0.412),
            PetalVariant::new(0, 0.250, 0.021, 0.250, 0.412),
            PetalVariant::new(0, 0.500, 0.005, 0.253, 0.445),
            PetalVariant::new(0, 0.751, 0.001, 0.249, 0.458),
            PetalVariant::new(0, 0.000, 0.541, 0.251, 0.407),
            PetalVariant::new(0, 0.250, 0.532, 0.253, 0.423),
            PetalVariant::new(0, 0.502, 0.488, 0.253, 0.512),
            PetalVariant::new(0, 0.767, 0.487, 0.216, 0.513),
            // pink_petal.png -- contains 1 petal image
            PetalVariant::new(1, 0.0, 0.0, 1.0, 1.0),
            // pink_petals_short -- contains 8 petal images
            PetalVariant::new(2, 0.000, 0.000, 0.218, 0.500),
            PetalVariant::new(2, 0.256, 0.000, 0.223, 0.500),
            PetalVariant::new(2, 0.506, 0.000, 0.239, 0.500),
            PetalVariant::new(2, 0.765, 0.000, 0.235, 0.500),
            PetalVariant::new(2, 0.000, 0.500, 0.218, 0.500),
            PetalVariant::new(2, 0.256, 0.500, 0.223, 0.500),
            PetalVariant::new(2, 0.506, 0.500, 0.239, 0.500),
            PetalVariant::new(2, 0.765, 0.500, 0.235, 0.500),
            // purple_petals.png -- contains 8 petal images
            PetalVariant::new(3, 0.000, 0.011, 0.250, 0.447),
            PetalVariant::new(3, 0.250, 0.000, 0.250, 0.455),
            PetalVariant::new(3, 0.499, 0.022, 0.237, 0.408),
            PetalVariant::new(3, 0.750, 0.060, 0.250, 0.373),
            PetalVariant::new(3, 0.000, 0.549, 0.250, 0.451),
            PetalVariant::new(3, 0.250, 0.551, 0.251, 0.449),
            PetalVariant::new(3, 0.501, 0.565, 0.251, 0.435),
            PetalVariant::new(3, 0.751, 0.592, 0.249, 0.381),
            // red_petal.png -- contains 1 petal image
            PetalVariant::new(4, 0.0, 0.0, 1.0, 1.0),
            // red_petals.png -- contains 6 petal images
            PetalVariant::new(5, 0.000, 0.027, 0.317, 0.424),
            PetalVariant::new(5, 0.328, 0.000, 0.341, 0.465),
            PetalVariant::new(5, 0.682, 0.023, 0.305, 0.410),
            PetalVariant::new(5, 0.000, 0.567, 0.315, 0.421),
            PetalVariant::new(5, 0.346, 0.541, 0.344, 0.459),
            PetalVariant::new(5, 0.690, 0.504, 0.310, 0.405),
            // rose_petals.png -- contains 3 petal images
            PetalVariant::new(6, 0.012, 0.032, 0.312, 0.933),
            PetalVariant::new(6, 0.364, 0.052, 0.284, 0.900),
            PetalVariant::new(6, 0.686, 0.047, 0.296, 0.896),
        ];

        // -----------------------------------------------------------------------------------------
        log::debug!("Instance setup");
        let mut petal_variant_indices: Vec<u32> = Vec::with_capacity(N_PETALS);
        let mut petal_poses: Vec<Pose> = Vec::with_capacity(N_PETALS);
        let mut petal_velocities: Vec<cgmath::Vector3<f32>> = Vec::with_capacity(N_PETALS);
        for _ in 0..N_PETALS {
            // Chose a random variant for each petal instance
            let variant_index = rng.gen_range(0..petal_variants.len() as u32);
            petal_variant_indices.push(variant_index);
            let aspect_ratio = petal_variants[variant_index as usize]
                .texture_u_v_width_height
                .vector[2]
                / petal_variants[variant_index as usize]
                    .texture_u_v_width_height
                    .vector[3];
            petal_poses.push(Pose {
                // Generate random petal positions in view of the camera -- in the [-1,1] x/y range
                // covered by NDC (normalized device coordinates).
                position: cgmath::vec3(
                    2.0 * MAX_DISPLACEMENT * rng.gen::<f32>() - MAX_DISPLACEMENT,
                    2.0 * MAX_DISPLACEMENT * rng.gen::<f32>() - MAX_DISPLACEMENT,
                    2.0 * MAX_DISPLACEMENT * rng.gen::<f32>() - MAX_DISPLACEMENT,
                ),
                // Randomly choose a rotation (this gives a uniform distribution over all rotations
                // in 3d space):
                rotation: cgmath::Quaternion::new(
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                )
                .normalize(),
                // Give the petal the right shape
                aspect_ratio,
                // Give the petal no rotation, represented by a quaternion of 1.0 real part and
                // zeros in all the imaginary dimensions.  If you think of complex numbers as
                // representing where the point 1.0 along the real axis would get rotated to if
                // operated on by that complex number, then this is basically just saying it stays
                // in the same place---thus no rotation.
                //rotation: cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0),
                scale: 1.5 * rng.gen::<f32>() + 0.5,
            });
            petal_velocities.push(cgmath::vec3(
                rng.gen::<f32>() * 10.0 * PER_PETAL_ACCELERATION - 5.0 * PER_PETAL_ACCELERATION,
                rng.gen::<f32>() * 10.0 * PER_PETAL_ACCELERATION - 5.0 * PER_PETAL_ACCELERATION,
                rng.gen::<f32>() * 10.0 * PER_PETAL_ACCELERATION - 5.0 * PER_PETAL_ACCELERATION,
            ))
        }

        // -----------------------------------------------------------------------------------------
        log::debug!("Noise generator setup");
        let noise_generator = noise::Perlin::default().set_seed(rng.gen()); //noise::Fbm::<noise::OpenSimplex>::default().set_seed(rng.gen());

        // -----------------------------------------------------------------------------------------
        let graphics_state = GraphicsState::new(
            window,
            &petal_texture_image_paths,
            petal_variants,
            petal_variant_indices,
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
        let wind_velocity = cgmath::vec3(
            rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
            rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
            rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
        );
        Self {
            rng,
            start_time: std::time::Instant::now(),
            graphics_state,
            controller_state,
            camera,
            petal_poses,
            petal_velocities,
            wind_velocity,
            game_window_focused: false,
            mouse_look_enabled: true,
            noise_generator,
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
                    window
                        .set_cursor_grab(if self.mouse_look_enabled {
                            winit::window::CursorGrabMode::Confined
                        } else {
                            winit::window::CursorGrabMode::None
                        })
                        .unwrap();
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
                    window
                        .set_cursor_grab(if self.game_window_focused {
                            winit::window::CursorGrabMode::Confined
                        } else {
                            winit::window::CursorGrabMode::None
                        })
                        .unwrap();
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
        for (pose, velocity) in self
            .petal_poses
            .iter_mut()
            .zip(self.petal_velocities.iter_mut())
        {
            pose.rotation = cgmath::Quaternion::from_angle_y(cgmath::Rad(0.03)) * pose.rotation;
            //pose.position += *velocity + self.wind_velocity;
            pose.position[1] -= FALL_SPEED;
            //*velocity += cgmath::vec3(
            //    self.rng.gen::<f32>() * PER_PETAL_ACCELERATION * 2.0 - PER_PETAL_ACCELERATION,
            //    self.rng.gen::<f32>() * PER_PETAL_ACCELERATION * 2.0 - PER_PETAL_ACCELERATION,
            //    self.rng.gen::<f32>() * PER_PETAL_ACCELERATION * 2.0 - PER_PETAL_ACCELERATION,
            //);
            //self.wind_velocity += cgmath::vec3(
            //    self.rng.gen::<f32>() * WIND_ACCELERATION * 2.0 - WIND_ACCELERATION,
            //    self.rng.gen::<f32>() * WIND_ACCELERATION * 2.0 - WIND_ACCELERATION,
            //    self.rng.gen::<f32>() * WIND_ACCELERATION * 2.0 - WIND_ACCELERATION,
            //);
            //*velocity *= VELOCITY_DECAY;
            //self.wind_velocity *= VELOCITY_DECAY;
            //pose.position[0] += 0.1
            //    * self.noise_generator.get([
            //        f64::from(pose.position[0]),
            //        f64::from(pose.position[1]),
            //        f64::from(pose.position[2]),
            //        0.1 * self.start_time.elapsed().as_secs_f64(),
            //    ]) as f32;
            if pose.position[0] < -MAX_DISPLACEMENT {
                pose.position[0] += 2.0 * MAX_DISPLACEMENT;
            } else if pose.position[0] > MAX_DISPLACEMENT {
                pose.position[0] -= 2.0 * MAX_DISPLACEMENT;
            }
            if pose.position[1] < -MAX_DISPLACEMENT {
                pose.position[1] += 2.0 * MAX_DISPLACEMENT;
            } else if pose.position[1] > MAX_DISPLACEMENT {
                pose.position[1] -= 2.0 * MAX_DISPLACEMENT;
            }
            if pose.position[2] < -MAX_DISPLACEMENT {
                pose.position[2] += 2.0 * MAX_DISPLACEMENT;
            } else if pose.position[2] > MAX_DISPLACEMENT {
                pose.position[2] -= 2.0 * MAX_DISPLACEMENT;
            }
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

#[derive(Debug, Copy, Clone)]
pub struct Pose {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
    // Aspect ratio: width / height
    aspect_ratio: f32,
    scale: f32,
}

impl Pose {
    fn new() -> Self {
        Pose {
            position: cgmath::vec3(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::one(),
            aspect_ratio: 1.0,
            scale: 1.0,
        }
    }
}

impl Default for Pose {
    fn default() -> Self {
        Pose::new()
    }
}

impl From<&Pose> for crate::graphics::gpu_types::Matrix4 {
    fn from(pose: &crate::game::Pose) -> Self {
        crate::graphics::gpu_types::Matrix4 {
            matrix: (cgmath::Matrix4::from_translation(pose.position)
                * cgmath::Matrix4::from(pose.rotation)
                //* cgmath::Matrix4::from_scale(pose.scale)
                * cgmath::Matrix4::from_nonuniform_scale(pose.scale * pose.aspect_ratio, pose.scale, pose.scale))
            .into(),
        }
    }
}
