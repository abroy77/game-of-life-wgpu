// Painting: sets every cell marked in the paint mask to a newborn (1).
//
// Runs once per cell, like compute.wgsl. Dispatched from `GraphicsContext::paint()` (graphics.rs)
// with the same uniform (group 0) and grid (group 1) bind groups as the simulation, plus
// the paint mask (group 2) from `MousePainter`.

// Grid size. Matches `ComputeUniform` in game_data.rs.
struct GridInfo {
    rows: u32,
    cols: u32,
    color_steps: u32,
    _pad: u32,
}

@group(0) @binding(0)
var<uniform> grid: GridInfo;

// The current grid (binding 0 of the simulation's grid bind group). We write into it directly.
@group(1) @binding(0)
var<storage, read_write> current_state: array<u32>;

// The paint mask uploaded from the CPU: 1 = paint this cell alive.
@group(2) @binding(0)
var<storage, read> paint_buffer: array<u32>;

@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<i32>(i32(global_id.x), i32(global_id.y));

    // Threads past the grid edge do nothing (see compute.wgsl).
    if !pos_in_grid(pos) {
        return;
    }

    let idx = pos_to_index(pos);

    // Painted cells become newborns (1, resetting any age); others are unchanged.
    current_state[idx] = select(current_state[idx], 1u, paint_buffer[idx] != 0u);
    return;
}

// Same helpers as compute.wgsl (WGSL has no imports, so they're copied).
fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

fn pos_in_grid(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.x < i32(grid.cols) && pos.y >= 0 && pos.y < i32(grid.rows);
}
