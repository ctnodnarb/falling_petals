//! This module defines structs that have memory layouts that are compatible with being placed into
//! GPU buffers.

use cgmath::prelude::*;

/// Trait for objects that can be placed in vertex buffers in wgpu.  Defines an associated function
/// that returns an object describing the memory layout of the vertex attiributes.
pub trait VertexBufferEntry {
    /// Returns a wgpu::VertexBufferLayout object describing the memory layout of the attributes in
    /// this type of vertex.
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a>;
}

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

impl VertexBufferEntry for Matrix4 {
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Matrix4>() as wgpu::BufferAddress,
            // TODO: Are there times when I would want the step mode to be Vertex instead of
            // Instance for Matrix4s?  If so, how can I make this configurable?
            // Tell the shader to only switch to use the next Matrix4 when the shader starts
            // processing a new instance.
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // TODO: I probably need a way to configure the shader locations more
                    // dynamically as well, instead of having it hard-coded for this type.  E.g. so
                    // multiple shaders in different locations can all use the Matrix4 type in
                    // different buffers.
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
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

impl From<&crate::graphics::Pose> for Matrix4 {
    fn from(position_rotation: &crate::graphics::Pose) -> Self {
        Matrix4 {
            matrix: (cgmath::Matrix4::from_translation(position_rotation.position)
                * cgmath::Matrix4::from(position_rotation.rotation))
            .into(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionTextureVertex {
    /// 3d position of the vertex
    pub position: [f32; 3],
    /// 2d texture coordinates at the vertex
    pub texture_coords: [f32; 2],
}

impl VertexBufferEntry for PositionTextureVertex {
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PositionTextureVertex>() as wgpu::BufferAddress,
            // step_mode specifies whether a vertex buffer is indexed by vertex or by instance.
            step_mode: wgpu::VertexStepMode::Vertex,
            // A list of the attributes within the vertex struct (assumed to be tightly packed).
            attributes: &[
                // Info for the "position" field in the PositionTextureVertex struct.
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // position field has @location(0) in the wgsl code.
                    shader_location: 0,
                },
                // Info for the "texture_coords" field in the PositionTextureVertex struct.
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // texture_coords field has @location(1) in the wgsl code.
                    shader_location: 1,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionColorVertex {
    /// 3d position of the vertex
    pub position: [f32; 3],
    /// 3d (f32) rgb color values at the vertex
    pub color: [f32; 3],
}

impl VertexBufferEntry for PositionColorVertex {
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PositionColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // position field has @location(0) in the wgsl code.
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // color field has @location(1) in the wgsl code.
                    shader_location: 1,
                },
            ],
        }
    }
}
