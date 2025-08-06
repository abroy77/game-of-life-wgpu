// Vertex Shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) @interpolate(flat) instance_idx: u32
};

struct RenderUniform {
    cell_size: f32,
    _pad: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: RenderUniform;
@group(1) @binding(0)
var<storage,read> current_state: array<u32>;


@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) instance_position: vec2f,
    @builtin(instance_index) instance_idx: u32
) -> VertexOutput {
    var out: VertexOutput;

    let scaled_position = position * uniforms.cell_size;
    let translated_position = scaled_position + instance_position;
    out.clip_position = vec4<f32>(translated_position ,0.0,1.0);
    out.instance_idx = instance_idx;
    return out;
}


@fragment
fn fs_main(
    in: VertexOutput,

) -> @location(0) vec4f {
    let alive = f32(current_state[in.instance_idx] & 1u);
    return vec4f(alive);
}
