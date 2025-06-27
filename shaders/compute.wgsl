// offsets for neighbours. 8-connectivity
const OFFSETS: array<vec2<i32>,8> = array<vec2<i32>,8>(
    vec2<i32>(-1,-1),
    vec2<i32>(-1,0),
    vec2<i32>(-1,1),
    vec2<i32>(0,1),
    vec2<i32>(1,1),
    vec2<i32>(1,0),
    vec2<i32>(1,-1),
    vec2<i32>(0,-1),
)

struct ComputeUniforms {
    rows: u32,
    cols: u32,
}

@group(1) @binding(0)
var<uniform> uniform: ComputeUniforms;

@group(0) @binding(0)
var<storage, read> current_state: array<u8>;
@group(0) @binding(1) 
var<storage, read_write> next_state: array<u8>; 

// Define the workgroup size
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let x = i32(global_id.x);
    let y = i32(global_id.y);

    // bounds check
    if x < 0 || x >= i32(uniform.cols) || y < 0 || y >= i32(uniform.rows) {
        return;   
    }

    let idx = get_index(x,y);
    let cell = get_cell(x,y);
    let neighbors = count_neighbors(x,y);
    var next: u8 = 0; // default to dead
    if cell == 0u && neighbors == 3u {
        next = 1u; // born
    } else if (cell == 1u) && (neighbors == 2u || neighbors == 3u){
        next = 1u; // survives
    }

    next_state[idx] = next;

}


fn get_index(x: i32, y: i32) -> u32 {
    // gives us the index of the cell in the cell_state buffers
    // ie current_state and next_state
    return y * uniform.cols + x;
}

fn get_cell(x:i32, y:i32) -> u8 {
    if x < 0 || x >= i32(uniform.cols) || y < 0 || y >= i32(uniform.rows) {
        return 0u;
    }
    return current_state[get_index(x,y)];
}



fn count_neighbors(x: i32,y:i32) -> u8 {
    var sum: u8 = 0;
    for (var i = 0; i < OFFSETS.length(); i = i + 1u) {
        let offset = OFFSETS[i]
        sum = sum + get_cell(x + offset.x, y + offset.y);
    }
    return sum
    
}


