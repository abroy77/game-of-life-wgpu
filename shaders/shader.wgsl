// Vertex shader

struct Uniforms {
    cell_size: vec2<f32>,
    gap: f32,
    rows: u32,
    cols: u32,
    _pad: u32
}

@group(0) @binding(0)
var<uniform> uniform: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) instance_pos: vec2<f32>
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) cell_pos: vec2<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>((model.position * uniform.cell_size) + model.instance_pos ,0.0, 1.0);
    out.cell_pos = model.instance_pos;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Create a subtle gradient based on position for visual interest
    let color_r = 0.7 + 0.3 * (in.cell_pos.x + 1.0) * 0.5;
    let color_g = 0.7 + 0.3 * (in.cell_pos.y + 1.0) * 0.5;
    let color_b = 0.9;
    return vec4<f32>(color_r, color_g, color_b, 1.0);
}
