
// Vertex shader -----------------------------------------------------------------------------------

struct PositionTextureVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coords: vec2<f32>,
};

struct PositionTextureVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
};

struct PositionColorVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct PositionColorVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_colored_vertex(vertex_in: PositionColorVertexInput) -> PositionColorVertexOutput {
    var out: PositionColorVertexOutput;
    out.clip_position = vec4(vertex_in.position, 1.0);
    out.color = vertex_in.color;
    return out;
}

//@vertex
//fn vertex_shader(model: VertexInput) -> VertexOutput{
//    var out: VertexOutput;
//    out.texture_coords = model.texture_coords;
//    out.clip_position = vec4<f32>(model.position, 1.0);
//    return out;
//}

// Fragment shader ---------------------------------------------------------------------------------

// Note the vertex shader output type is the same as the fragment shader input type.  However, the
// actual values coming in to the fragment shader have been interpolated along the primitive (line, 
// triangle, etc) for each fragment/pixel before coming in as inputs to the fragment shader.

@fragment
fn fs_colored_vertex(fragment_in: PositionColorVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(fragment_in.color, 1.0);
}

//@group(0) @binding(0)
//var texture_diffuse: texture_2d<f32>;
//@group(0) @binding(1)
//var sampler_diffuse: sampler;

//@fragment
//fn fragment_shader(in: VertexOutput) -> @location(0) vec4<f32> {
//    //return textureSample(texture_diffuse, sampler_diffuse, in.texture_coords);
//    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
//}
