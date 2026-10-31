// Painting: draws this tick's stroke segments into the grid, setting their cells to newborns (1).
//
// One thread per (point along a segment, segment), using the DDA line algorithm.
// Dispatched from `GraphicsContext::paint()` (graphics.rs) with the same uniform (group 0)
// and grid (group 1) bind groups as the simulation, plus the segments (group 2) from `MousePainter`.

// Grid size. Matches `ComputeUniform` in game_data.rs.
struct GridInfo {
    rows: u32,
    cols: u32,
    color_steps: u32,
    _pad: u32,
}

// Line from cell `a` to cell `b`. Matches `Segment` in paint.rs.
struct Segment {
    a: vec2<i32>,
    b: vec2<i32>,
}

// count at byte 0, 4 bytes padding, segments at byte 8 (vec2<i32> aligns to 8). CPU must match.
struct Path {
    count: u32,
    segments: array<Segment>,
}

@group(0) @binding(0)
var<uniform> grid: GridInfo;

// The current grid (binding 0 of the simulation's grid bind group). We write into it directly.
@group(1) @binding(0)
var<storage, read_write> current_state: array<u32>;

// This tick's segments, uploaded by `MousePainter::write_to_buffer`.
@group(2) @binding(0)
var<storage, read> path: Path;

// Must match the div_ceil(64) dispatch in `GraphicsContext::paint()`.
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    // x will handle steps in a segment
    // y will handle segments
    if gid.y >= path.count {
        return;
    }

    let seg = path.segments[gid.y];

    let d = seg.b - seg.a;
    let steps = max(abs(d.x), abs(d.y));
    // x handles steps so we only need those within our step count
    if gid.x > u32(steps) {
        return;
    }

    // for this gid.x, how far along the segment are we? from 0-1
    // where 0 is at seg.a, and 1 is at seg.b
    let step_fraction = select(0.0, f32(gid.x) / f32(steps), steps > 0);

    // mix() linearly interpolates a → b (per component) at step_fraction;
    // round() snaps that exact point to the nearest cell.
    let position = vec2<i32>(round(mix(vec2<f32>(seg.a), vec2<f32>(seg.b), step_fraction)));

    if !pos_in_grid(position) {
        return;
    }

    let state_index = pos_to_index(position);

    // Painted cells become newborns (1, resetting any age); others are unchanged.
    current_state[state_index] = 1u;
}

// Same helpers as compute.wgsl (WGSL has no imports, so they're copied).
fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

fn pos_in_grid(pos: vec2<i32>) -> bool {
    return pos.x >= 0 && pos.x < i32(grid.cols) && pos.y >= 0 && pos.y < i32(grid.rows);
}
