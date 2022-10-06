
// Interface matching (passing inputs and outpus between the CPU and the different shaders):
//   - Each interface is considered valid as long as the set of inputs consumed by the next stage is
//      a subset of the set of outputs produced by the last stage. (See
//      https://github.com/gpuweb/gpuweb/issues/644.)
//   - However, I tried writing a fragment shader that did not consume/define a color input
//      corresponding to a color output from the vertex shader, and it crashed with an error saying
//      "location[0] is provided by the previous stage output but is not consumed as input by this
//      stage".
//   - You can use the same struct as the vertex shader output and as the fragment shader input.
//      Or you can define different structs for the vertex shader output and fragment shader input
//      as long as they match / consume all the provided locations.  The order of the fields in the
//      struct definition do not appear to matter as long as the @location specifications match
//      correctly.

// Vertex shader -----------------------------------------------------------------------------------

struct PositionColorVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct PositionColorVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct PositionTextureVertexInput {
    // location(x) is used to match up input and output variables (between the cpu and gpu, as well
    // as between the different shader stages).  Note that each location can only hold up to a
    // vector of 4 i32s or f32s.  So a 4x4 matrix of f32s will need to use up 4 locations, and a 
    // vector of 4 doubles would use up 2 locations.
    @location(0) position: vec3<f32>,
    @location(1) texture_coords: vec2<f32>,
};

struct PositionTextureVertexOutput {
    // builtin(position) is a vec4<f32> specifying:
    //   - For a vertex shader output, the homogeneous output coordinates of the vertex (does not
    //     have to be normalized).  After normalizing by dividing everything by w, the position will
    //     be in normalized device coordinates (NDC).
    //   - For a fragment shader input, the framebuffer (screen) space position of the current
    //     fragment.  The x and y components are the pixel coordinates, and z is the value that
    //     would get written to the depth buffer (if the depth test allows and if a different value
    //     is not specified via the frag_depth builtin output value).  The w coordinate will be 1.
    //     Note that the y axis points down in framebuffer coordinates, with the origin in the upper
    //     left of the window.
    // See https://www.w3.org/TR/WGSL/#builtin-values.
    // Also see https://dmnsgn.me/blog/from-glsl-to-wgsl-the-future-of-shaders-on-the-web/#built-in.
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
};

struct PositionTextureIndexVertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_coords: vec2<f32>,
    @location(2) index: u32,
};

struct PositionTextureIndexVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
};

struct PoseInput {
    @location(5) pose_matrix_c0: vec4<f32>,
    @location(6) pose_matrix_c1: vec4<f32>,
    @location(7) pose_matrix_c2: vec4<f32>,
    @location(8) pose_matrix_c3: vec4<f32>,
};

struct Matrix4Uniform {
    matrix4: mat4x4<f32>,
};

// Define uniforms passed in through the bind group.  Note that shaders that do not access these 
// uniform variables should not need the bind group with them to be present (I think).  If I
// understand correctly, these bindings will only apply to shaders where the variable with that
// specified binding is used (anywhere in the shader's function heirarchy).
@group(0) @binding(0)
var<uniform> color_pipeline_camera: Matrix4Uniform;
@group(1) @binding(0)
var<uniform> texture_pipeline_camera: Matrix4Uniform;

@vertex
fn vs_colored_vertex(vertex_in: PositionColorVertexInput) -> PositionColorVertexOutput {
    var out: PositionColorVertexOutput;
    out.clip_position = color_pipeline_camera.matrix4 * vec4(vertex_in.position, 1.0);
    //out.clip_position = vec4(vertex_in.position, 1.0);
    out.color = vertex_in.color;
    return out;
}

@vertex
fn vs_textured_vertex(
    model: PositionTextureVertexInput,
    pose: PoseInput, 
    @builtin(instance_index) instance_index: u32,
) -> PositionTextureIndexVertexOutput {
    let pose_matrix = mat4x4<f32>(
        pose.pose_matrix_c0,
        pose.pose_matrix_c1,
        pose.pose_matrix_c2,
        pose.pose_matrix_c3,
    );
    var out: PositionTextureIndexVertexOutput;
    out.texture_coords = model.texture_coords;
    out.clip_position = texture_pipeline_camera.matrix4 * pose_matrix * vec4<f32>(model.position, 1.0);
    out.index = instance_index;
    return out;
}

// Fragment shader ---------------------------------------------------------------------------------

// Note the vertex shader output type is the same as the fragment shader input type.  However, the
// actual values coming in to the fragment shader have been interpolated along the primitive (line, 
// triangle, etc) for each fragment/pixel before coming in as inputs to the fragment shader.

// Note: This must have identical memory layout to PositionColorVertexOutput since that's what's
// getting interpolated to create these.  I could have just used PositionColorVertexOutput again
// as the fragment shader input, but was curious if you could define a different struct with more
// intuitive names and identical memory layout (turns out you can).
struct PositionColorFragmentInput {
    @builtin(position) screen_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

// Note: This must have identical memory layout to PositionTextureVertexOutput since that's what is
// getting interpolated to create these.
struct PositionTextureFragmentInput {
    @builtin(position) screen_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
};

// Note: This must have identical memory layout to PositionTextureIndexVertexOutput since that's
// what is getting interpolated to create these.
struct PositionTextureIndexFragmentInput {
    @builtin(position) screen_position: vec4<f32>,
    @location(0) texture_coords: vec2<f32>,
    @location(1) @interpolate(flat) index: u32,
};

@fragment
fn fs_colored_vertex(fragment_in: PositionColorFragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(fragment_in.color, 1.0);
    // Create a plaid look by cycling the colors based on pixel location
    // Note that "let" creates immutable variables, and "var" creates mutable ones.
    //let r_cycle = 100.0;
    //let g_cycle = 50.0;
    //let b_cycle = 125.0;
    //let pct = 0.8; // Blend in the color interpolated from the vertexes at this percentage.
    //let r = pct * fragment_in.color[0] + (1.0-pct) * (fragment_in.screen_position[1] % r_cycle / r_cycle);
    //let g = pct * fragment_in.color[1] + (1.0-pct) * (fragment_in.screen_position[0] % g_cycle / g_cycle);
    //let b = pct * fragment_in.color[2] + (1.0-pct) * (fragment_in.screen_position[0] % b_cycle / b_cycle);
    //return vec4<f32>(r, g, b, 1.0);
}

// Uniform buffers require a stride of at least 16 between elements of an array.  To pass an array
// with elements with a smaller stride than that, we must wrap them to give them a stride of 16.
// See https://www.w3.org/TR/WGSL/#address-space-layout-constraints, particularly the examples.
struct UniformU32{
    @size(16) value: u32,
};
struct PetalVariant {
    // All-caps values are textually replaced at runtime before shader creation/compilation.
    petal_texture_index: UniformU32,
    texture_u_v_width_height: vec4<f32>,
};

// Define uniforms passed in through the bind group.  Note that shaders that do not access these 
// uniform variables should not need the bind group with them to be present (I think).  If I
// understand correctly, these bindings will only apply to shaders where the variable with that
// specified binding is used (anywhere in the shader's function heirarchy).
@group(0) @binding(0)
var texture_pipeline_petal_textures: binding_array<texture_2d<f32>>;
@group(0) @binding(1)
var texture_pipeline_petal_samplers: binding_array<sampler>;
// TODO:  I think something about how I'm defining these arrays is causing problems when I try to
// use too many different petal variants.  It crashes with a validation error saying that I'm using
// more uniform buffer bindings than is allowed, an dthat the limit is 12.  So seemingly, increasing
// the number of petal variants is increasing the number of uniform buffer bindings.  I'm not sure
// why increasing the number of petals before didn't also do that, as I think I was defining that
// array in the same way.  Perhaps I need to change the var<uniform> below to be a single struct,
// and define the array of PetalVariants inside of that struct.  Perhaps then it would put an array
// of PetalVariants into a single uniform buffer instead of creating an array of uniform buffers
// with a slot for each petal variant???  Actually, I just tested that and increasing N_PETALS also
// breaks it by exceeding the limit of 12.  So apparently I'm passing the array of indices
// differently now from when I was previously able to increase the petal count up to many thousands.
@group(0) @binding(2)
var<uniform> texture_pipeline_petal_variants: array<PetalVariant, N_PETAL_VARIANTS>;
@group(0) @binding(3)
var<uniform> texture_pipeline_petal_variant_indices: array<UniformU32, N_PETALS>;

@fragment
fn fs_textured_vertex(in: PositionTextureIndexFragmentInput) -> @location(0) vec4<f32> {
    let variant_idx = texture_pipeline_petal_variant_indices[in.index].value;
    let tex_idx = texture_pipeline_petal_variants[variant_idx].petal_texture_index.value;
    let tex_bounds = texture_pipeline_petal_variants[variant_idx].texture_u_v_width_height;
    let texture_sample = textureSample(
        texture_pipeline_petal_textures[tex_idx], 
        texture_pipeline_petal_samplers[tex_idx], 
        vec2<f32>(
            tex_bounds[0] + in.texture_coords[0] * tex_bounds[2],
            tex_bounds[1] + in.texture_coords[1] * tex_bounds[3],
        )
    );
    if texture_sample[3] < 0.01{
        discard;
    } else {
        return texture_sample;
    }
}
