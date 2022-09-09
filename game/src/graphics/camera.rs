use cgmath::prelude::*;

/// Trait for objects that can be used as a camera.  Defines a method that returns a view-projection
/// matrix.
pub trait Camera {
    fn get_view_projection_matrix(&self) -> cgmath::Matrix4<f32>;
}

/// Thw wpgu package uses the DirectX NDC coordinate system, but cgmath uses OpenGL's.  This matrix
/// converts from OpenGL's to DirectX's NDCs:
/// OpenGL:
///     Clip space in [-1, 1] for all dimensions (XYZ); window origin in lower left corner.
/// DirectX:
///     Clip space in [-1, 1] for XY and in [0, 1] for Z; window origin in upper left corner.
pub const CGMATH_NDC_TO_WGPU_NDC_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0, // col 1
    0.0, 1.0, 0.0, 0.0, // col 2
    0.0, 0.0, 0.5, 0.0, // col 3 -- Compress [-1, 1] to [-0.5, 0.5] along the z-axis.
    0.0, 0.0, 0.5, 1.0, // col 4 -- Translate [-0.5, 0.5] to [0, 1] along the z-axis.
);

/// Represents a camera that can turn side to side and look up and down, but cannot roll.  With zero
/// rotation, the camera points along the positive x axis with the y axis as its up vector and the z
/// axis extending to the right.
#[derive(Debug)]
pub struct UprightPerspectiveCamera {
    /// Coordinate of the focal point of the camera.
    pub location: cgmath::Point3<f32>,
    /// Direction the camera is facing in the x/z (horizontal) plane, in range [0, 2*pi).
    pub pan_angle: cgmath::Rad<f32>,
    /// Amount of tilt (up and down) of the camera, in range [-pi/2, pi/2].
    pub tilt_angle: cgmath::Rad<f32>,
    /// Field of view angle in the Y direction
    pub fov_y: cgmath::Rad<f32>,
    /// Aspect ratio (width / height)
    pub aspect_ratio: f32,
    /// Near clipping plane location
    pub z_near: f32,
    /// Far clipping plane location
    pub z_far: f32,
    /// Maximum tilt angle allowed
    pub max_tilt: cgmath::Rad<f32>,
    /// Minimum tilt angle allowed
    pub min_tilt: cgmath::Rad<f32>,
}

impl Default for UprightPerspectiveCamera {
    fn default() -> Self {
        Self {
            location: (0.0, 0.0, 0.0).into(), //cgmath::Point3::<f32>::new(0.0, 0.0, 0.0),
            pan_angle: cgmath::Rad::<f32>(0.0),
            tilt_angle: cgmath::Rad::<f32>(0.0),
            fov_y: cgmath::Rad::turn_div_4(),
            aspect_ratio: 1.0,
            z_near: 0.1,
            z_far: 100.0,
            max_tilt: cgmath::Rad::turn_div_4(),
            min_tilt: -cgmath::Rad::turn_div_4(),
        }
    }
}

impl UprightPerspectiveCamera {
    pub fn new(
        location: cgmath::Point3<f32>,
        pan_angle: cgmath::Rad<f32>,
        tilt_angle: cgmath::Rad<f32>,
        fov_y: cgmath::Rad<f32>,
        aspect_ratio: f32,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            location,
            pan_angle,
            tilt_angle,
            fov_y,
            aspect_ratio,
            z_near,
            z_far,
            ..Default::default()
        }
    }

    /// Construct a matrix representing the multiplication of the projection and view matrices.
    pub fn get_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let pan_tilt_rotation = cgmath::Quaternion::<f32>::from_angle_y(self.pan_angle)
            * cgmath::Quaternion::<f32>::from_angle_x(self.tilt_angle);
        let view: cgmath::Matrix4<f32> = cgmath::Matrix4::look_to_rh(
            self.location,
            pan_tilt_rotation * cgmath::Vector3::unit_x(),
            pan_tilt_rotation * cgmath::Vector3::unit_y(),
        );
        let projection =
            cgmath::perspective(self.fov_y, self.aspect_ratio, self.z_near, self.z_far);
        CGMATH_NDC_TO_WGPU_NDC_MATRIX * projection * view
    }

    /// Rotates the camera horizontally (pan) and vertically (tilt).
    pub fn pan_and_tilt(
        &mut self,
        pan_angle_change: cgmath::Rad<f32>,
        tilt_angle_change: cgmath::Rad<f32>,
    ) {
        self.pan_angle = (self.pan_angle + pan_angle_change) % cgmath::Rad::full_turn();
        self.tilt_angle += tilt_angle_change;
        if self.tilt_angle > self.max_tilt {
            self.tilt_angle = self.max_tilt;
        } else if self.tilt_angle < self.min_tilt {
            self.tilt_angle = self.min_tilt;
        }
    }

    /// Moves the camera relative to it's current pan orientation (but not relative to it's tilt).
    pub fn move_relative_to_pan_angle(&mut self, forward: f32, right: f32, up: f32) {
        let pan_rotation = cgmath::Quaternion::<f32>::from_angle_y(self.pan_angle);
        self.location += pan_rotation * cgmath::Vector3::<f32>::new(forward, up, right);
    }
}
