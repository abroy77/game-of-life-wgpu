// Vertex Shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

struct RenderUniform {
    cell_size: f32,
};

@group(0) @binding(0)
var<uniform> uniforms: RenderUniform;


@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) instance_position: vec2f
) -> VertexOutput {
    var out: VertexOutput;

    let scaled_position = position * uniforms.cell_size;
    let translated_position = scaled_position + instance_position;
    out.clip_position = vec4<f32>(translated_position ,0.0,1.0);
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return vec4f(1);
}
