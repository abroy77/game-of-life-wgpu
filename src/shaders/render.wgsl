// Vertex Shader


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};


@vertex
fn vs_main(
    @location(0) position: vec2f,
) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(position,0.0,1.0);
    return out;
}

@fragment
fn fs_main(
    in: VertexOutput
) -> @location(0) vec4f {
    return vec4f(1);
}
