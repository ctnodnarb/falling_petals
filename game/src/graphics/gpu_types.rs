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
    pub matrix: [[f32; 4]; 4],
}

impl Matrix4 {
    pub fn new() -> Self {
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

// TODO:  It turns out I don't need this struct for the purpose I originally intended, whihc was to
// use the index value to index into a texture array in order to vary the texture for each instance
// of the object.  The problem is that the vertex data (e.g. 4 vertices to draw a square) is the
// same for every instance that I render, so I can't actually use it to vary the texture on an
// instance-by-instance basis (but rather only on a vertex-by-vertex basis).  Instead, I need to
// figure out a way to send instance-by-instance indices (perhaps along with the
// instance-by-instance pose matrices) to the shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionTextureIndexVertex {
    /// 3d position of the vertex
    pub position: [f32; 3],
    /// 2d texture coordinates at the vertex
    pub texture_coords: [f32; 2],
    /// Index associated with the vertex (e.g. to index a texture array)
    pub index: u32,
}

impl VertexBufferEntry for PositionTextureIndexVertex {
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PositionTextureIndexVertex>() as wgpu::BufferAddress,
            // step_mode specifies whether a vertex buffer is indexed by vertex or by instance.
            step_mode: wgpu::VertexStepMode::Vertex,
            // A list of the attributes within the vertex struct (assumed to be tightly packed).
            attributes: &[
                // Info for the "position" field
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // position field has @location(0) in the wgsl code.
                    shader_location: 0,
                },
                // Info for the "texture_coords" field
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    // Location of this field in the corresponding struct defined in the shader
                    // code (the @location value for the corresponding field).  In this case, the
                    // texture_coords field has @location(1) in the wgsl code.
                    shader_location: 1,
                },
                // Info for the "index" field
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Uint32,
                    offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                },
            ],
        }
    }
}

/// Struct to store Vector4 values in a format that is compatible with being put in buffers sent to
/// the GPU.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vector4 {
    pub vector: [f32; 4],
}

impl Vector4 {
    pub fn new() -> Self {
        Self {
            vector: cgmath::Vector4::zero().into(),
        }
    }
}

impl From<[f32; 4]> for Vector4 {
    fn from(vector: [f32; 4]) -> Self {
        Vector4 { vector }
    }
}

impl From<cgmath::Vector4<f32>> for Vector4 {
    fn from(vector: cgmath::Vector4<f32>) -> Self {
        let vector: [f32; 4] = vector.into();
        vector.into()
    }
}

/// Struct used to put u32 values into uniform buffers passed into shaders.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UniformU32 {
    pub value: u32,
    /// Needed to give this struct the minimum 16-byte alignment required by uniform buffers.
    _pad: [u32; 3],
}

impl UniformU32 {
    pub fn new() -> Self {
        Self {
            value: 0,
            _pad: [0, 0, 0],
        }
    }
}

impl From<&u32> for UniformU32 {
    fn from(value: &u32) -> Self {
        Self {
            value: *value,
            _pad: [0, 0, 0],
        }
    }
}

/// Struct to store the texture index and the u/v coordinate and width and height of the section of
/// the texture to use when rendering a particular petal.  This allows me to pick between multiple
/// textures and slice out individual petals from textures that contain multiple images of petals.
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PetalTextureInfo {
    pub petal_texture_index: UniformU32,
    pub texture_u_v_width_height: Vector4,
}
