// Simulation step: computes the next generation of the Game of Life.
//
// Runs once per cell, all in parallel. Each GPU thread handles one cell:
// count its live neighbours in `current_state`, apply the rules, write the result to `next_state`.
// If the cell was alive previously, increment the current_state value.
// Current state values are clamped at `color_steps` for coloring in the vertex shader.
// Reading from one buffer and writing to another means no thread ever sees a half-updated grid.
// Dispatched from `GraphicsContext::update` (graphics.rs).

// Grid size. Matches `ComputeUniform` in game_data.rs, padded to 16 bytes.
struct GridInfo {
    rows: u32,
    cols: u32,
    color_steps: u32,
    _pad: u32,
}

// @group(N) @binding(M) must match the bind group layouts on the Rust side:
// group 0 = `compute_uniform_bind_group`, group 1 = `game_state_bind_group_a/b`.
@group(0) @binding(0)
var<uniform> grid: GridInfo;

// The grid as a flat array, row by row from the bottom. 0 = dead, n ≥ 1 = alive with age n - 1.
@group(1) @binding(0)
var<storage, read_write> current_state: array<u32>;
@group(1) @binding(1)
var<storage, read_write> next_state: array<u32>;

// Entry point. Each workgroup is 16x16 threads, and `global_invocation_id` is this thread's
// position across all workgroups, i.e. its cell's (column, row).
@compute @workgroup_size(16, 16, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<i32>(i32(global_id.x), i32(global_id.y));

    // We launch whole workgroups, so some threads land past the grid edge. They do nothing.
    if !pos_in_grid(pos) {
        return;
    }

    let state_index = pos_to_index(pos);

    let state = current_state[state_index];
    let alive = state != 0;

    // Apply the rules:
    // - exactly 3 neighbours → alive
    // - alive with exactly 2 neighbours → stays alive
    // - anything else → dead
    let n_neighbors = count_alive_neighbours(pos);
    var next: u32 = 0u;
    if n_neighbors == 3u || (alive && n_neighbors == 2u) {
        // newborn → 1, survivor → +1, capped so state - 1 is always a valid colour index
        next = select(1u, min(state + 1u, grid.color_steps), alive);
    }

    // Write to the next_state buffer.
    next_state[state_index] = next;
}

// (column, row) → index into the flat grid array.
fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

// Is (column, row) inside the grid?
fn pos_in_grid(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.x < i32(grid.cols) && pos.y >= 0 && pos.y < i32(grid.rows);
}

fn count_alive_neighbours(pos: vec2<i32>) -> u32 {
    let x = u32(pos.x);
    let y = u32(pos.y);

    // `select` is much cheaper than using modulo in GPUs apparently
    // selects a value based on a predicate:
    // select(val_if_false, val_if_true, predicate)
    // we use it here to wrap around the grid for the neighbours
    let left = select(x - 1u, grid.cols - 1u, x == 0u);
    let right = select(x + 1u, 0u, x == grid.cols - 1u);
    let up = select(y + 1u, 0u, y == grid.rows - 1u);
    let down = select(y - 1u, grid.rows - 1u, y == 0u);

    // for the 3 rows the neighbours are in get the index offset
    let row_down = down * grid.cols;
    let row = y * grid.cols;
    let row_up = up * grid.cols;

    // add up the 8 neighbours alive values:
    return is_alive(row_down + left) + // bottom left
        is_alive(row_down + x) + // bottom
        is_alive(row_down + right) + // bottom right
        is_alive(row + left) + // left
        is_alive(row + right) + // right
        is_alive(row_up + left) + // top left 
        is_alive(row_up + x) + // top
        is_alive(row_up + right) // top right
        ;
}

// accesses a cell's alive value by index and returns `1`
// if it's alive.
fn is_alive(idx: u32) -> u32 {
    return u32(current_state[idx] != 0u);
}
