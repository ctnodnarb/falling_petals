use crate::configuration::{FallingPetalsConfig, VideoExportConfig};
use crate::graphics::{camera::UprightPerspectiveCamera, gpu_types::PetalVariant, GraphicsState};
use crate::input::InputState;

use cgmath::prelude::*;
use cgmath::{Deg, Rad};
//use noise::{NoiseFn, Seedable};
use rand::prelude::*;
use rand_distr::StandardNormal;
use winit::event::{DeviceEvent, ElementState, MouseButton, WindowEvent};
use winit::window::Window;

pub struct FallingPetalsState {
    /// Config values for the game
    pub config: FallingPetalsConfig,
    /// Random number generator for this thread
    pub rng: rand::rngs::ThreadRng,
    /// Game start time
    pub start_time: std::time::Instant,
    /// Holds handles to GPU resources and objects in a form compatible with being passed/copied to
    /// GPU buffers/resources.
    pub graphics_state: GraphicsState,
    /// Tracks the state of the user input.
    pub input_state: InputState,
    /// Camera used to render the world
    pub camera: UprightPerspectiveCamera,
    /// Used to enable / disable input and control whether or not the mouse is grabbed.
    pub game_window_focused: bool,
    pub mouse_look_enabled: bool,
    // Petals
    pub petal_states: Vec<PetalState>,
    pub x_movement: Vec<f32>,
    pub y_movement: Vec<f32>,
    pub z_movement: Vec<f32>,
    pub movement_frame_idx: u32,
    pub movement_period: u32,
}

impl FallingPetalsState {
    pub fn new(
        window: &Window,
        config: FallingPetalsConfig,
        video_export_config: VideoExportConfig,
    ) -> Self {
        let mut rng = rand::thread_rng();

        // -----------------------------------------------------------------------------------------
        log::debug!("Computing petal movement");
        let movement_period = config.movement_period * video_export_config.frame_rate;
        let x_movement = Self::generate_mixture_of_sines(
            movement_period,
            config.movement_n_frequencies,
            config.movement_low_freq_max_amplitude,
            config.movement_high_freq_max_amplitude,
            &mut rng,
        );
        let y_movement = Self::generate_mixture_of_sines(
            movement_period,
            config.movement_n_frequencies,
            config.movement_low_freq_max_amplitude,
            config.movement_high_freq_max_amplitude,
            &mut rng,
        );
        let z_movement = Self::generate_mixture_of_sines(
            movement_period,
            config.movement_n_frequencies,
            config.movement_low_freq_max_amplitude,
            config.movement_high_freq_max_amplitude,
            &mut rng,
        );

        // -----------------------------------------------------------------------------------------
        log::debug!("Petal variants setup");
        let petal_texture_image_paths = vec![
            "falling_petals/res/pink_petals_long.png",
            "falling_petals/res/pink_petal.png",
            "falling_petals/res/pink_petals_short.png",
            "falling_petals/res/purple_petals.png",
            "falling_petals/res/red_petal.png",
            "falling_petals/res/red_petals.png",
            "falling_petals/res/rose_petals.png",
            "falling_petals/res/PetalsArranged.png",
        ];
        // Grid spacing for PetalsArranged.png
        const SP: f32 = 128.0 / 8192.0;
        let petal_variants = vec![
            //// pink_petals_long.png -- contains 8 petal images
            //PetalVariant::new(0, 0.000, 0.021, 0.250, 0.412),
            //PetalVariant::new(0, 0.250, 0.021, 0.250, 0.412),
            //PetalVariant::new(0, 0.500, 0.005, 0.253, 0.445),
            //PetalVariant::new(0, 0.751, 0.001, 0.249, 0.458),
            //PetalVariant::new(0, 0.000, 0.541, 0.251, 0.407),
            //PetalVariant::new(0, 0.250, 0.532, 0.253, 0.423),
            //PetalVariant::new(0, 0.502, 0.488, 0.253, 0.512),
            //PetalVariant::new(0, 0.767, 0.487, 0.216, 0.513),
            //// pink_petal.png -- contains 1 petal image
            //PetalVariant::new(1, 0.0, 0.0, 1.0, 1.0),
            //// pink_petals_short -- contains 8 petal images
            //PetalVariant::new(2, 0.000, 0.000, 0.218, 0.500),
            //PetalVariant::new(2, 0.256, 0.000, 0.223, 0.500),
            //PetalVariant::new(2, 0.506, 0.000, 0.239, 0.500),
            //PetalVariant::new(2, 0.765, 0.000, 0.235, 0.500),
            //PetalVariant::new(2, 0.000, 0.500, 0.218, 0.500),
            //PetalVariant::new(2, 0.256, 0.500, 0.223, 0.500),
            //PetalVariant::new(2, 0.506, 0.500, 0.239, 0.500),
            //PetalVariant::new(2, 0.765, 0.500, 0.235, 0.500),
            //// purple_petals.png -- contains 8 petal images
            //PetalVariant::new(3, 0.000, 0.011, 0.250, 0.447),
            //PetalVariant::new(3, 0.250, 0.000, 0.250, 0.455),
            //PetalVariant::new(3, 0.499, 0.022, 0.237, 0.408),
            //PetalVariant::new(3, 0.750, 0.060, 0.250, 0.373),
            //PetalVariant::new(3, 0.000, 0.549, 0.250, 0.451),
            //PetalVariant::new(3, 0.250, 0.551, 0.251, 0.449),
            //PetalVariant::new(3, 0.501, 0.565, 0.251, 0.435),
            //PetalVariant::new(3, 0.751, 0.592, 0.249, 0.381),
            //// red_petal.png -- contains 1 petal image
            ////PetalVariant::new(4, 0.0, 0.0, 1.0, 1.0), // looks like an apple...
            //// red_petals.png -- contains 6 petal images
            //PetalVariant::new(5, 0.000, 0.027, 0.317, 0.424),
            //PetalVariant::new(5, 0.328, 0.000, 0.341, 0.465),
            //PetalVariant::new(5, 0.682, 0.023, 0.305, 0.410),
            //PetalVariant::new(5, 0.000, 0.567, 0.315, 0.421),
            //PetalVariant::new(5, 0.346, 0.541, 0.344, 0.459),
            //PetalVariant::new(5, 0.690, 0.504, 0.310, 0.405),
            //// rose_petals.png -- contains 3 petal images
            //PetalVariant::new(6, 0.012, 0.032, 0.312, 0.933),
            //PetalVariant::new(6, 0.364, 0.052, 0.284, 0.900),
            //PetalVariant::new(6, 0.686, 0.047, 0.296, 0.896),
            // PetalsArranged.png -- contains many petal images
            // Col 1: 3x1
            PetalVariant::new(7, 0.0 * SP, 0.0 * SP, 3.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 0.0 * SP, 1.0 * SP, 3.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 0.0 * SP, 2.0 * SP, 3.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 0.0 * SP, 3.0 * SP, 3.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 0.0 * SP, 4.0 * SP, 3.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 0.0 * SP, 5.0 * SP, 3.0 * SP, 1.0 * SP),
            // Col 2: 6x3
            PetalVariant::new(7, 3.0 * SP, 0.00 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 3.00 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 6.00 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 9.00 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 12.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 15.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 18.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 21.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 24.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 27.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 30.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 33.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 36.0 * SP, 6.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 3.0 * SP, 39.0 * SP, 6.0 * SP, 3.0 * SP),
            // Col 3: 5x3
            PetalVariant::new(7, 9.0 * SP, 0.00 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 3.00 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 6.00 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 9.00 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 12.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 15.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 18.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 21.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 24.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 27.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 30.0 * SP, 5.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 9.0 * SP, 33.0 * SP, 5.0 * SP, 3.0 * SP),
            // Col 4: 4x2
            PetalVariant::new(7, 14.0 * SP, 0.00 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 2.00 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 4.00 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 6.00 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 8.00 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 10.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 12.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 14.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 16.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 18.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 20.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 22.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 24.0 * SP, 4.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 14.0 * SP, 26.0 * SP, 4.0 * SP, 2.0 * SP),
            // Col 5: 5x2
            PetalVariant::new(7, 18.0 * SP, 0.00 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 2.00 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 4.00 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 6.00 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 8.00 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 10.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 12.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 14.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 16.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 18.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 20.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 22.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 24.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 26.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 28.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 30.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 32.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 34.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 36.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 38.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 40.0 * SP, 5.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 18.0 * SP, 42.0 * SP, 5.0 * SP, 2.0 * SP),
            // Col 6: 4x3
            PetalVariant::new(7, 23.0 * SP, 0.00 * SP, 4.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 23.0 * SP, 3.00 * SP, 4.0 * SP, 3.0 * SP),
            PetalVariant::new(7, 23.0 * SP, 6.00 * SP, 4.0 * SP, 3.0 * SP),
            // Col 7: 5x1
            PetalVariant::new(7, 27.0 * SP, 0.00 * SP, 5.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 27.0 * SP, 1.00 * SP, 5.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 27.0 * SP, 2.00 * SP, 5.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 27.0 * SP, 3.00 * SP, 5.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 27.0 * SP, 4.00 * SP, 5.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 27.0 * SP, 5.00 * SP, 5.0 * SP, 1.0 * SP),
            // Col 8: 6x2
            PetalVariant::new(7, 32.0 * SP, 0.00 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 2.00 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 4.00 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 6.00 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 8.00 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 10.0 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 12.0 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 14.0 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 16.0 * SP, 6.0 * SP, 2.0 * SP),
            PetalVariant::new(7, 32.0 * SP, 18.0 * SP, 6.0 * SP, 2.0 * SP),
            // Col 9: 4x1
            PetalVariant::new(7, 38.0 * SP, 0.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 1.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 2.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 3.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 4.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 5.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 6.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 7.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 8.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 9.00 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 10.0 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 11.0 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 12.0 * SP, 4.0 * SP, 1.0 * SP),
            PetalVariant::new(7, 38.0 * SP, 13.0 * SP, 4.0 * SP, 1.0 * SP),
            // Col 10: 6x4
            PetalVariant::new(7, 42.0 * SP, 0.00 * SP, 6.0 * SP, 4.0 * SP),
            // Col 11: 2x1
            PetalVariant::new(7, 48.0 * SP, 0.00 * SP, 2.0 * SP, 1.0 * SP),
            // Col 12: 3x2
            PetalVariant::new(7, 50.0 * SP, 0.00 * SP, 3.0 * SP, 2.0 * SP),
            // Col 13: 7x3
            PetalVariant::new(7, 53.0 * SP, 0.00 * SP, 7.0 * SP, 3.0 * SP),
        ];

        // -----------------------------------------------------------------------------------------
        log::debug!("Instance setup");
        let mut petal_states: Vec<PetalState> = Vec::with_capacity(config.n_petals);
        for _ in 0..config.n_petals {
            // Chose a random variant for each petal instance
            let variant_index = rng.gen_range(0..petal_variants.len() as u32);
            let aspect_ratio = petal_variants[variant_index as usize]
                .texture_u_v_width_height
                .vector[2]
                / petal_variants[variant_index as usize]
                    .texture_u_v_width_height
                    .vector[3];
            let actual_scale = if petal_variants[variant_index as usize]
                .petal_texture_index
                .value
                == 7
            {
                petal_variants[variant_index as usize]
                    .texture_u_v_width_height
                    .vector[3]
                    / (4.0 * SP)
            } else {
                1.0
            };
            let pose = Pose {
                // Generate random petal positions in view of the camera -- in the [-1,1] x/y range
                // covered by NDC (normalized device coordinates).
                position: cgmath::vec3(
                    2.0 * config.max_x * rng.gen::<f32>() - config.max_x,
                    2.0 * config.max_y * rng.gen::<f32>() - config.max_y,
                    2.0 * config.max_z * rng.gen::<f32>() - config.max_z,
                ),
                // Randomly choose a rotation (this gives a uniform distribution over all rotations
                // in 3d space):
                orientation: cgmath::Quaternion::new(
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                    rng.sample(StandardNormal),
                )
                .normalize(),
                // Give the petal the right shape
                aspect_ratio,
                scale: actual_scale
                    * ((config.max_scale - config.min_scale) * rng.gen::<f32>() + config.min_scale),
            };
            let rotation = Self::generate_random_rotation(
                Rad::<f32>::from(config.min_rotation_speed),
                Rad::<f32>::from(config.max_rotation_speed),
                &mut rng,
            );

            petal_states.push(PetalState {
                pose,
                variant_index,
                rotation,
            });
        }
        petal_states
            .sort_unstable_by(|a, b| a.pose.position[2].partial_cmp(&b.pose.position[2]).unwrap());

        // -----------------------------------------------------------------------------------------
        log::debug!("Noise generator setup");
        //let noise_generator = noise::Perlin::default().set_seed(rng.gen()); //noise::Fbm::<noise::OpenSimplex>::default().set_seed(rng.gen());

        // -----------------------------------------------------------------------------------------
        let graphics_state = GraphicsState::new(
            window,
            &petal_texture_image_paths,
            petal_variants,
            &petal_states,
            &config,
            video_export_config,
        );
        let input_state = InputState::new();

        // -----------------------------------------------------------------------------------------
        log::debug!("Camera setup");
        // Place the camera in the middle of the front side of the cube where the petals will be
        // spawned and will move around within, looking toward the opposite side of that cuve (in
        // the -z direction, silimar to how NDCs are oriented).  This gives it a good view of as
        // many petals in the volume as possible.
        let camera_location = cgmath::Point3::<f32>::new(0.0, 0.0, config.max_z);
        // Turn the camera 90 degrees to the left (ccw around the y axis pointing up) to face in the
        // -z direction, thus matching normalized device coordinates.  Note that the camera is
        // defined such that pan and tilt angles of 0 mean the camera is pointing the same direction
        // as the +x axis.
        let camera_pan = Deg::<f32>(0.0);
        let camera_tilt = Deg::<f32>(0.0);
        let camera_fov_y = Deg::<f32>(60.0);
        let camera_z_near = 1.0;
        // Set the far plane to be at the far edge of the petal simulation volume.
        let camera_z_far = 2.0 * config.max_z;
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
        //let wind_velocity = cgmath::vec3(
        //    rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
        //    rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
        //    rng.gen::<f32>() * 10.0 * WIND_ACCELERATION - 5.0 * WIND_ACCELERATION,
        //);
        Self {
            config,
            rng,
            start_time: std::time::Instant::now(),
            graphics_state,
            input_state,
            camera,
            petal_states,
            game_window_focused: false,
            mouse_look_enabled: false,
            x_movement,
            y_movement,
            z_movement,
            movement_frame_idx: 0,
            movement_period,
        }
    }

    /// Handles the passed event if possible, and returns a boolean value indicating if the event
    /// was handled or not.
    pub fn handle_window_event(&mut self, event: &WindowEvent, window: &Window) -> bool {
        match event {
            WindowEvent::KeyboardInput { input, .. } => {
                self.input_state.handle_keyboard_event(input)
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
                self.input_state.get_pan_tilt_delta();
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
                self.input_state.get_pan_tilt_delta();
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
        self.input_state.handle_device_event(event);
    }

    pub fn update(&mut self) {
        // Game state update code goes here.

        if self.game_window_focused {
            self.update_based_on_input_state();
        }

        // Rotate all the petals a bit each frame to test changing the instance pose buffer
        for petal_state in self.petal_states.iter_mut() {
            petal_state.pose.orientation = petal_state.rotation * petal_state.pose.orientation;

            petal_state.pose.position[1] -= self.config.fall_speed;

            petal_state.pose.position[0] += self.x_movement[self.movement_frame_idx as usize];
            petal_state.pose.position[1] += self.y_movement[self.movement_frame_idx as usize];
            petal_state.pose.position[2] += self.z_movement[self.movement_frame_idx as usize];

            if petal_state.pose.position[0] < -self.config.max_x {
                petal_state.pose.position[0] += 2.0 * self.config.max_x;
            } else if petal_state.pose.position[0] > self.config.max_x {
                petal_state.pose.position[0] -= 2.0 * self.config.max_x;
            }
            if petal_state.pose.position[1] < -self.config.max_y {
                petal_state.pose.position[1] += 2.0 * self.config.max_y;
            } else if petal_state.pose.position[1] > self.config.max_y {
                petal_state.pose.position[1] -= 2.0 * self.config.max_y;
            }
            if petal_state.pose.position[2] < -self.config.max_z {
                petal_state.pose.position[2] += 2.0 * self.config.max_z;
            } else if petal_state.pose.position[2] > self.config.max_z {
                petal_state.pose.position[2] -= 2.0 * self.config.max_z;
            }
        }

        // Update the z-ordering of the petals so that alpha blending renders correctly from back to
        // front.
        self.petal_states
            .sort_unstable_by(|a, b| a.pose.position[2].partial_cmp(&b.pose.position[2]).unwrap());

        // Update GPU buffers according to the current game state.
        self.graphics_state.update(&self.camera, &self.petal_states);

        self.movement_frame_idx = (self.movement_frame_idx + 1) % self.movement_period;
    }

    fn update_based_on_input_state(&mut self) {
        self.camera.move_relative_to_pan_angle(
            self.config.player_movement_speed * self.input_state.forward_multiplier(),
            self.config.player_movement_speed * self.input_state.right_muliplier(),
            self.config.player_movement_speed * self.input_state.jump_multiplier(),
        );
        if self.mouse_look_enabled {
            let (pan_delta, tilt_delta) = self.input_state.get_pan_tilt_delta();
            self.camera.pan_and_tilt(
                self.config.player_turn_speed * pan_delta,
                self.config.player_turn_speed * tilt_delta,
            )
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.graphics_state.render()
    }

    /// Attempt to reconfigure / reacquire the rendering surface using the last known window size.
    pub fn reconfigure_rendering_surface(&mut self) {
        self.graphics_state.resize(self.graphics_state.size)
    }

    fn generate_random_rotation(
        min_angle: Rad<f32>,
        max_angle: Rad<f32>,
        rng: &mut rand::rngs::ThreadRng,
    ) -> cgmath::Quaternion<f32> {
        let axis = cgmath::Vector3::<f32> {
            x: rng.sample(StandardNormal),
            y: rng.sample(StandardNormal),
            z: rng.sample(StandardNormal),
        }
        .normalize();
        let angle = min_angle + (max_angle - min_angle) * rng.gen::<f32>();
        cgmath::Rotation3::from_axis_angle(axis, angle)
    }

    fn generate_mixture_of_sines(
        length: u32,
        n_frequencies: u32,
        low_freq_max_amplitude: f32,
        high_freq_max_amplitude: f32,
        rng: &mut rand::rngs::ThreadRng,
    ) -> Vec<f32> {
        let mut amplitudes_by_frequency = Vec::<f32>::with_capacity(n_frequencies as usize);
        let mut phases_by_frequency = Vec::<f32>::with_capacity(n_frequencies as usize);
        for freq_idx in 0..n_frequencies {
            let max_amplitude = low_freq_max_amplitude
                - (low_freq_max_amplitude - high_freq_max_amplitude)
                    * (freq_idx as f32 / (n_frequencies - 1) as f32);
            amplitudes_by_frequency.push(max_amplitude * rng.gen::<f32>());
            phases_by_frequency.push(2.0 * std::f32::consts::PI * rng.gen::<f32>());
        }
        let mut values = Vec::<f32>::with_capacity(length as usize);
        for frame_idx in 0..length {
            let mut value = 0.0;
            for freq_idx in 0..n_frequencies {
                value += amplitudes_by_frequency[freq_idx as usize]
                    * f32::sin(
                        2.0 * std::f32::consts::PI
                            * freq_idx as f32
                            * (frame_idx as f32 / length as f32)
                            + phases_by_frequency[freq_idx as usize],
                    );
            }
            values.push(value);
        }
        values
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Pose {
    position: cgmath::Vector3<f32>,
    orientation: cgmath::Quaternion<f32>,
    // Aspect ratio: width / height
    aspect_ratio: f32,
    scale: f32,
}

impl Pose {
    fn new() -> Self {
        Pose {
            position: cgmath::vec3(0.0, 0.0, 0.0),
            orientation: cgmath::Quaternion::one(),
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
    fn from(pose: &crate::state::Pose) -> Self {
        crate::graphics::gpu_types::Matrix4 {
            matrix: (cgmath::Matrix4::from_translation(pose.position)
                * cgmath::Matrix4::from(pose.orientation)
                //* cgmath::Matrix4::from_scale(pose.scale)
                * cgmath::Matrix4::from_nonuniform_scale(pose.scale * pose.aspect_ratio, pose.scale, pose.scale))
            .into(),
        }
    }
}

pub struct PetalState {
    pub pose: Pose,
    pub variant_index: u32,
    pub rotation: cgmath::Quaternion<f32>,
}
