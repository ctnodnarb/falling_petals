//! This module defines structs that have memory layouts that are compatible with being placed into
//! GPU buffers.

use cgmath::prelude::*;

/// Struct to store 4x4 matrices in a format that is compatible with being put in buffers sent to
/// the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Matrix4 {
    matrix: [[f32; 4]; 4],
}

impl Matrix4 {
    fn new() -> Self {
        Self {
            matrix: cgmath::Matrix4::identity().into(),
        }
    }
}

impl From<[[f32; 4]; 4]> for Matrix4 {
    fn from(matrix: [[f32; 4]; 4]) -> Self {
        Matrix4 { matrix }
    }
}

impl From<cgmath::Matrix4<f32>> for Matrix4 {
    fn from(matrix: cgmath::Matrix4<f32>) -> Self {
        let matrix: [[f32; 4]; 4] = matrix.into();
        matrix.into()
    }
}
