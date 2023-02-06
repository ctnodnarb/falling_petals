use cgmath::Deg;
use serde::{Deserialize, Serialize};

/// Include the text of the default config.toml file (which includes comments on what the different
/// settings are) into the executable so that it can be generated easily without depending on any
/// other files to exist in any particular location.
pub const DEFAULT_CONFIG_STR: &str = include_str!("../res/config.toml");

/// Configuration values for the falling petals visualization.  Note that n_petals cannot be set
/// larger than 1/4 the maximum allowed uniform buffer size of the GPU.  So on a GPU with a maximum
/// uniform buffer size of 65536 bytes, n_petals cannot be set above 16384.  Doing so will cause a
/// crash when the program tries to allocate a uniform buffer that is too big.
#[derive(Serialize, Deserialize)]
pub struct FallingPetalsConfig {
    /// The number of petals moving around in the simulation volume.
    pub n_petals: usize,
    /// Lower bound of the random scale factor applied to each petal.
    pub min_scale: f32,
    /// Upper bound of the random scale factor applied to each petal.
    pub max_scale: f32,
    /// List of texture files and where all the individual petal images are within each texture.
    pub petal_textures: Vec<PetalTextureConfig>,
    /// Multiplier to adjust overall amount of petal bend.
    pub petal_bend_vertex_offset_multiplier: f32,
    /// Z-offsets for the 9 vertices (in row-major order) of each petal instance, used to ensure
    /// that the petals do not look perfectly flat.
    pub petal_bend_vertex_offsets: [f32; 9],
    /// Whether or not to limit the live rendering frame rate.  This does not affect the frame rate
    /// of any exported video.
    pub enable_frame_rate_limit: bool,
    /// The live rendering frame rate limit to use (if enabled).
    pub frame_rate_limit: u32,
    /// The distance between the focal point of the camera and the near clipping plane.
    pub camera_near: f32,
    /// The distance between the focal point of the camera and the far clipping plane.
    pub camera_far: f32,
    /// The vertical field of view angle of the camera.
    pub camera_fov_y: Deg<f32>,
    /// The maximum magnitude of the x-coordinate of each petal, used to define the size of the
    /// simulation volume.
    pub max_x: f32,
    /// The maximum magnitude of the y-coordinate of each petal, used to define the size of the
    /// simulation volume.
    pub max_y: f32,
    /// The maximum magnitude of the z-coordinate of each petal, used to define the size of the
    /// simulation volume.
    pub max_z: f32,
    /// The distance the camera moves (forward, back, left, right, up, or down) each frame when
    /// controlled with the keyboard.
    pub player_movement_speed: f32,
    /// The angle the camera pans/tilts per pixel of mouse movement when mouselook is enabled.
    pub player_turn_speed: Deg<f32>,
    /// A constant speed at which all the petals fall per frame.  This fall speed is added to the
    /// other motion of the petal (which may counteract it).
    pub fall_speed: f32,
    /// The period of the overal mixture of sinusoids used for petal motion, in seconds based on the
    /// video_export_fps value.  If set to 60 with a video_export_fps of 30, then the sinusoidal
    /// movement pattern would repeat every 1800 frames (or every minute in the exported video).
    pub movement_period: u32,
    /// The number of frequencies that will be mixed together to generate the mixture of sinusoids.
    /// Together with movement_period, this serves as a frequency cap for how rapidly the petal
    /// movement can change.
    pub movement_n_frequencies: u32,
    /// Amplitudes are randomly chosen (up to a cap) for each frequency in the mixture of sinusoids.
    /// This defines the amplitude cap for the highest frequency.  The caps for intermediate
    /// frequencies are linearly interpolated between this and the cap for the lowest frequency.
    pub movement_high_freq_max_amplitude: f32,
    /// Amplitudes are randomly chosen (up to a cap) for each frequency in the mixture of sinusoids.
    /// This defines the amplitude cap for the lowest frequency.  The caps for intermediate
    /// frequencies are linearly interpolated between this and the cap for the highest frequency.
    pub movement_low_freq_max_amplitude: f32,
    /// The rotation speed for each petal is randomly chosen between min_rotation_speed and
    /// max_rotation_speed.
    pub min_rotation_speed: Deg<f32>,
    /// The rotation speed for each petal is randomly chosen between min_rotation_speed and
    /// max_rotation_speed.
    pub max_rotation_speed: Deg<f32>,
    /// Whether or not to export the rendered visualization to video.  If enabled, ffmpeg must be
    /// installed and visible on the current PATH for it to work.  Enabling this causes each frame
    /// to be rendered a second time to an off-screen buffer, whose pixel values are then piped over
    /// to an ffmpeg process that encodes and compresses them into a video file.
    pub enable_ffmpeg_video_export: bool,
    /// The name of the video file to create, if video export is enabled.
    pub video_export_file: String,
    /// The frame rate of the exported video, if video export is enabled.
    pub video_export_fps: u32,
    /// The width (x resolution) of the exported video, if video export is enabled.
    pub video_export_width: u32,
    /// The height (y resolution) of the exported video, if video export is enabled.
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
