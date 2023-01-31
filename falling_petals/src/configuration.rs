use cgmath::Deg;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_STR: &str = include_str!("../res/config.toml");

/// n_petals must be a multiple of 4 due to how I'm assuming on the shader side that the petal
/// variant indexes are packed into an array of vec4<u32>s.  If it is not a multiple of 4, then the
/// buffer size sent from the CPU will not match the expected buffer size on the shader/GPU side,
/// which will cause a crash due to a validation error.  Packing the indexes into vec4s allows me to
/// fit 4 times as many of them into a uniform buffer (which has a max size of 64k or 65536 bytes on
/// my GPU) than I otherwise would be able to.  With that uniform buffer size limit, the max number
/// of petals that can be rendered is 16384.
#[derive(Serialize, Deserialize)]
pub struct FallingPetalsConfig {
    pub n_petals: usize,
    pub min_scale: f32,
    pub max_scale: f32,
    pub petal_bend_vertex_offset_multiplier: f32,
    pub petal_bend_vertex_offsets: [f32; 9],
    pub camera_near: f32,
    pub camera_far: f32,
    pub camera_fov_y: Deg<f32>,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
    pub player_movement_speed: f32,
    pub player_turn_speed: Deg<f32>,
    pub fall_speed: f32,
    pub movement_period: u32,
    pub movement_max_freq: u32,
    pub movement_amplitude_min: f32,
    pub movement_amplitude_max: f32,
    pub min_rotation_speed: Deg<f32>,
    pub max_rotation_speed: Deg<f32>,
    pub enable_ffmpeg_video_export: bool,
    pub video_export_fps: u32,
    pub video_export_width: u32,
    pub video_export_height: u32,
}

impl Default for FallingPetalsConfig {
    fn default() -> Self {
        match toml::from_str::<FallingPetalsConfig>(DEFAULT_CONFIG_STR) {
            Ok(result) => result,
            Err(error) => {
                log::error!("Error parsing the default config string:\n{error}");
                unreachable!();
                //Self {
                //    n_petals: 7000,
                //    min_scale: 1.0,
                //    max_scale: 2.0,
                //    petal_bend_vertex_offset_multiplier: 0.1,
                //    petal_bend_vertex_offsets: [1.0, 0.2, -0.6, -0.1, 0.0, -0.2, -1.0, 0.3, 0.7],
                //    camera_near: 1.0,
                //    camera_far: 100.0,
                //    camera_fov_y: Deg::<f32>(60.0),
                //    // Set the boundaries of the rectangular prism in which the petals are rendered so that
                //    // they are not visible in the view frustum (at its default location).
                //    // For a 60fovy frustum with 100 view depth and 1920x1080 aspect ratio, we need max_x > 103.
                //    max_x: 110.0,
                //    // For a 60fovy frustum with 100 view depth, we need max_y > 58.
                //    max_y: 65.0,
                //    // Note that max_z is doubled (goes negative and positive the same as max_x and max_y) in
                //    // determining the total volume in which the petals are rendered.  So the camera's view
                //    // depth gets set to double this value.
                //    max_z: 50.0,
                //    player_movement_speed: 0.5,
                //    player_turn_speed: Deg::<f32>(0.1),
                //    fall_speed: 0.1,
                //    movement_period: 60 * 15,
                //    movement_max_freq: 60,
                //    movement_amplitude_min: 0.015,
                //    movement_amplitude_max: 0.075,
                //    min_rotation_speed: Deg::<f32>(1.0),
                //    max_rotation_speed: Deg::<f32>(3.0),
                //    enable_ffmpeg_video_export: false,
                //    video_export_fps: 30,
                //    video_export_width: 1920,
                //    video_export_height: 1080,
                //}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_parses_without_error() {
        FallingPetalsConfig::default();
    }
}
