// Vertex shader

struct RenderUniforms {
    cell_size: vec2<f32>,
    gap: f32,
    rows: u32,
    cols: u32,
    _pad: u32,
}

@group(0) @binding(0)
var<uniform> uniforms: RenderUniforms;

@group(1) @binding(0)
var<storage, read> game_state: array<u32>;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct InstanceInput {
    @location(1) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) instance_pos: vec2<f32>
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    
    // Check if this cell is alive
    let is_alive = game_state[instance_index];
    
    // If cell is dead, move it off-screen
    if is_alive == 0u {
        out.clip_position = vec4<f32>(-10.0, -10.0, 0.0, 1.0);
        return out;
    }
    
    // Scale the vertex position by cell size and translate by instance position
    let scaled_position = model.position * uniforms.cell_size;
    let world_position = scaled_position + instance.position;
    
    out.clip_position = vec4<f32>(world_position, 0.0, 1.0);
    out.instance_pos = instance.position;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = (in.instance_pos + vec2<f32>(1.0,1.0)) * 0.5;
    let color = vec3<f32>(pos.x,pos.y,1.0 - pos.x * pos.y);
    return vec4<f32>(color, 1.0); // White color for alive cells
}
