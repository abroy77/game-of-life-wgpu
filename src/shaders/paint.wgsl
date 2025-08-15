struct GridInfo {
    rows: u32,
    cols: u32,
    _pad: vec2<u32>
};

@group(0)@binding(0)
var<uniform> grid: GridInfo;
@group(1)@binding(0)
var<storage, read_write> current_state: array<u32>;
@group(2)@binding(0)
var<storage, read> paint_buffer: array<u32>;


@compute @workgroup_size(16,16,1)
fn  main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<i32>(i32(global_id.x), i32(global_id.y));

    if !pos_in_grid(pos) {
        return;
    }

    let idx = pos_to_index(pos);

    current_state[idx] = current_state[idx] | paint_buffer[idx];
    return;
}

fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

fn pos_in_grid(pos:vec2<i32>) -> bool {
    return pos.x >= 0 &&
        pos.x < i32(grid.cols) &&
        pos.y >= 0 &&
        pos.y < i32(grid.rows);
}
