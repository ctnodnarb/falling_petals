pub trait Vertex {
    /// Returns a wgpu::VertexBufferLayout object describing the layout of this type of vertex
    fn vertex_buffer_layout<'a>() -> wgpu::VertexBufferLayout<'a>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PositionTextureVertex {
    /// 3d position of the vertex
    pub position: [f32; 3],
    /// 2d texture coordinates at the vertex
    pub texture_coords: [f32; 2],
}

impl Vertex for PositionTextureVertex {
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
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
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

impl Vertex for PositionColorVertex {
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
