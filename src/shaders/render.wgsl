// Drawing the grid. Contains two shader stages, both used by the render pipeline in render_data.rs:
// - vs_main (vertex shader): runs once per corner of every square. Decides where it goes on screen
//   and its colour (from the cell's age).
// - fs_main (fragment shader): runs once per pixel inside the squares. Outputs that colour.
// One instanced draw call draws every cell (see `GraphicsContext::render()`).

// What the vertex shader passes to the fragment shader.
struct VertexOutput {
    // Required: the corner's position in clip space (x, y, z, w).
    @builtin(position) clip_position: vec4<f32>,
    // The cell's colour. `flat`: use it as-is instead of blending between corners.
    @interpolate(flat)
    @location(0) color: vec4<f32>,
}

// Matches `RenderUniform` in graphics.rs, padded to 16 bytes.
struct RenderUniform {
    cell_size: vec2<f32>,
    _pad: vec2<f32>,
}

// group 0 = `render_uniform_bind_group` (render_data.rs),
// group 1 = `render_bind_group_a/b` (game_data.rs): whichever grid buffer is current.
@group(0) @binding(0)
var<uniform> uniforms: RenderUniform;
// Age → colour table from colormap.rs: index = cell age.
@group(0) @binding(1)
var<storage, read> age_colors: array<vec4<f32>>;
// 0 = dead, n ≥ 1 = alive with age n - 1.
@group(1) @binding(0)
var<storage, read> current_state: array<u32>;

// Inputs come from the two vertex buffers, by `shader_location` (see vertex.rs):
// location 0 = this corner of the unit square, location 1 = this cell's centre.
@vertex
fn vs_main(
    @location(0) position: vec2f,
    @location(1) instance_position: vec2f,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    var out: VertexOutput;

    // The instance index equals the cell's index in the grid (same row-by-row order).
    let state = current_state[instance_idx];
    let alive = state != 0u;
    // Dead cells: state - 1u wraps to a huge number, min() clamps it (colour unused).
    let age = min(state - 1u, arrayLength(&age_colors) - 1u);
    out.color = age_colors[age];

    // Scale the unit square to cell size, then move it to the cell's centre.
    let scaled_position = position * uniforms.cell_size;
    let translated_position = scaled_position + instance_position;
    // Branch-free: dead cells collapse to one point (zero-area, no pixels).
    // z = 0 (flat 2D), w = 1 (no perspective).
    out.clip_position = select(
        vec4<f32>(0.0, 0.0, 0.0, 1.0),
        vec4<f32>(translated_position, 0.0, 1.0),
        alive,
    );
    return out;
}

// Output goes to colour target 0 (the surface texture).
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}
