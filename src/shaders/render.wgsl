// Drawing the grid. Contains two shader stages, both used by the render pipeline in render_data.rs:
// - vs_main (vertex shader): runs once per corner of every square. Decides where it goes on screen.
// - fs_main (fragment shader): runs once per pixel inside the squares. Decides its colour.
// One instanced draw call draws every cell (see `GraphicsContext::render()`).

// What the vertex shader passes to the fragment shader.
struct VertexOutput {
    // Required: the corner's position in clip space (x, y, z, w).
    @builtin(position) clip_position: vec4<f32>,
    // Which cell this square (technically triangle) belongs to. Values normally get blended across a triangle;
    // `flat` turns that off, and integers must be flat.
    @interpolate(flat)
    @location(0) instance_idx: u32,
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

    // Scale the unit square to cell size, then move it to the cell's centre.
    let scaled_position = position * uniforms.cell_size;
    let translated_position = scaled_position + instance_position;
    // z = 0 (flat 2D), w = 1 (no perspective).
    out.clip_position = vec4<f32>(translated_position, 0.0, 1.0);
    // The instance index equals the cell's index in the grid, because instances
    // are in the same row-by-row order.
    out.instance_idx = instance_idx;
    return out;
}

// Output goes to colour target 0 (the surface texture).
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    // Alive → (1, 1, 1, 1): opaque white. Dead → (0, 0, 0, 0): fully transparent, so with
    // alpha blending the background colour shows through.
    let alive = f32(current_state[in.instance_idx] & 1u);
    return vec4f(alive);
}
