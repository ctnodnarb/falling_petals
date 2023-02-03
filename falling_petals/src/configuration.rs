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
    pub petal_textures: Vec<PetalTextureConfig>,
    pub petal_bend_vertex_offset_multiplier: f32,
    pub petal_bend_vertex_offsets: [f32; 9],
    pub enable_frame_rate_limit: bool,
    pub frame_rate_limit: u32,
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
    pub movement_n_frequencies: u32,
    pub movement_high_freq_max_amplitude: f32,
    pub movement_low_freq_max_amplitude: f32,
    pub min_rotation_speed: Deg<f32>,
    pub max_rotation_speed: Deg<f32>,
    pub enable_ffmpeg_video_export: bool,
    pub video_export_file: String,
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
                panic!("Error parsing the default config string:\n{error}");
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PetalTextureConfig {
    pub file: String,
    pub scale: f32,
    pub x_multiplier: f32,
    pub y_multiplier: f32,
    pub petal_coordinates: Vec<[f32; 4]>,
}

pub struct VideoExportConfig {
    pub export_enabled: bool,
    pub output_file: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: u32,
    pub pixel_count: u32,
    pub frame_size: u64,
    pub texture_format: wgpu::TextureFormat,
}

impl VideoExportConfig {
    pub fn new(
        export_enabled: bool,
        output_file: String,
        width: u32,
        height: u32,
        frame_rate: u32,
        texture_format: wgpu::TextureFormat,
    ) -> Self {
        let pixel_count = width * height;
        VideoExportConfig {
            export_enabled,
            output_file,
            width,
            height,
            frame_rate,
            pixel_count,
            // One u32 per pixel for Bgra8unorm
            frame_size: std::mem::size_of::<u32>() as u64 * pixel_count as u64,
            texture_format,
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
