use cgmath::{Deg, Rad};

/// n_petals must be a multiple of 4 due to how I'm assuming on the shader side that the petal
/// variant indexes are packed into an array of vec4<u32>s.  If it is not a multiple of 4, then the
/// buffer size sent from the CPU will not match the expected buffer size on the shader/GPU side,
/// which will cause a crash due to a validation error.  Packing the indexes into vec4s allows me to
/// fit 4 times as many of them into a uniform buffer (which has a max size of 64k or 65536 bytes on
/// my GPU) than I otherwise would be able to.  With that uniform buffer size limit, the max number
/// of petals that can be rendered is 16384.
pub struct FallingPetalsConfig {
    pub n_petals: usize,
    pub min_scale: f32,
    pub max_scale: f32,
    pub fall_speed: f32,
    pub camera_near: f32,
    pub camera_far: f32,
    pub camera_fov_y: Rad<f32>,
    pub max_x: f32,
    pub max_y: f32,
    pub max_z: f32,
    pub player_movement_speed: f32,
    pub player_turn_speed: Rad<f32>,
    pub movement_period: u32,
    pub movement_max_freq: u32,
    pub movement_amplitude_min: f32,
    pub movement_amplitude_max: f32,
    pub min_rotation_speed: Deg<f32>,
    pub max_rotation_speed: Deg<f32>,
}
