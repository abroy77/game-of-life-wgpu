struct GridInfo {
    rows: u32,
    cols: u32,
    _pad: vec2<u32>
};
@group(0)@binding(0)
var<uniform> grid: GridInfo;

@group(1)@binding(0)
var<storage, read> current_state: array<u32>;
@group(1)@binding(1)
var<storage, read_write> next_state: array<u32>;


const OFFSETS: array<vec2<i32>, 8> = array<vec2<i32>, 8>(
    vec2<i32>( 0, -1), // top
    vec2<i32>( -1, -1), // top left 
    vec2<i32>(-1,  0), // left
    vec2<i32>(-1, 1), // bottom left
    vec2<i32>( 0,  1), // bottom
    vec2<i32>( 1,  1), // bottom right
    vec2<i32>( 1,  0),  // Right
    vec2<i32>( 1,  -1)  // top right
);

@compute @workgroup_size(16,16,1)
fn  main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pos = vec2<i32>(i32(global_id.x), i32(global_id.y));

    if !pos_in_grid(pos) {
        return;
    }

    let state_index = pos_to_index(pos);

    let state = current_state[state_index];

    let n_neighbors = get_n_neighbors(pos);
    var next: u32 = 0u;
    if n_neighbors == 3u {
        next = 1u;
    } else if state == 1u && n_neighbors == 2u {
        next = 1u;
    }

    next_state[state_index] = next;

}

// fn grid_position(idx: u32) -> vec2<i32> {
//     let row = i32(idx / grid.cols);
//     let col = i32(idx % grid.cols);
//     return vec2<i32>(col,row);
// }

fn pos_to_index(pos: vec2<i32>) -> u32 {
    return u32(pos.x + pos.y * i32(grid.cols));
}

fn pos_in_grid(pos:vec2<i32>) -> bool {
    return pos.x >= 0 &&
        pos.x < i32(grid.cols) &&
        pos.y >= 0 &&
        pos.y < i32(grid.rows);
}

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