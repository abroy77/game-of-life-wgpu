// Simulation step: computes the next generation of the Game of Life.
//
// Runs once per cell, all in parallel. Each GPU thread handles one cell:
// count its live neighbours in `current_state`, apply the rules, write the result to `next_state`.
// Reading from one buffer and writing to another means no thread ever sees a half-updated grid.
// Dispatched from `GraphicsContext::update` (graphics.rs).

// Grid size. Matches `ComputeUniform` in game_data.rs, padded to 16 bytes.
struct GridInfo {
    rows: u32,
    cols: u32,
    _pad: vec2<u32>
};

// @group(N) @binding(M) must match the bind group layouts on the Rust side:
// group 0 = `compute_uniform_bind_group`, group 1 = `game_state_bind_group_a/b`.
@group(0)@binding(0)
var<uniform> grid: GridInfo;

// The grid as a flat array, row by row from the bottom. 1 = alive, 0 = dead.
@group(1)@binding(0)
var<storage, read_write> current_state: array<u32>;
@group(1)@binding(1)
var<storage, read_write> next_state: array<u32>;


// (x, y) steps to the 8 neighbours around a cell
const OFFSETS: array<vec2<i32>, 8> = array<vec2<i32>, 8>(
    vec2<i32>( 0, -1), // bottom
    vec2<i32>( -1, -1), // bottom left
    vec2<i32>(-1,  0), // left
    vec2<i32>(-1, 1), // top left
    vec2<i32>( 0,  1), // top
    vec2<i32>( 1,  1), // top right
    vec2<i32>( 1,  0),  // Right
    vec2<i32>( 1,  -1)  // bottom right
);

// Entry point. Each workgroup is 16x16 threads, and `global_invocation_id` is this thread's
// position across all workgroups, i.e. its cell's (column, row).
@compute @workgroup_size(16,16,1)
fn  main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<i32>(i32(global_id.x), i32(global_id.y));

    // We launch whole workgroups, so some threads land past the grid edge. They do nothing.
    if !pos_in_grid(pos) {
        return;
    }

    let state_index = pos_to_index(pos);

    let state = current_state[state_index];

    // Apply the rules:
    // - exactly 3 neighbours → alive 
    // - alive with exactly 2 neighbours → stays alive
    // - anything else → dead
    let n_neighbors = get_n_neighbors(pos);
    var next: u32 = 0u;
    if n_neighbors == 3u {
        next = 1u;
    } else if state == 1u && n_neighbors == 2u {
        next = 1u;
    }

    // Write to the next_state buffer.
    next_state[state_index] = next;

}

// (column, row) → index into the flat grid array.
fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

// Is (column, row) inside the grid?
fn pos_in_grid(pos:vec2<i32>) -> bool {
    return pos.x >= 0 &&
        pos.x < i32(grid.cols) &&
        pos.y >= 0 &&
        pos.y < i32(grid.rows);
}

// Count live neighbours. Positions outside the grid count as dead (the edges don't wrap around).
fn get_n_neighbors(pos: vec2<i32>) -> u32 {
    var n: u32 = 0u;
    for (var i: i32 = 0; i < 8; i=i+1) {
        let offset = OFFSETS[i];
        let neighbor = pos + offset;
        if pos_in_grid(neighbor) {
            let idx = pos_to_index(neighbor);
            if current_state[idx] == 1u {
                n= n+1u;
            }

        }

    }

    return n;
}