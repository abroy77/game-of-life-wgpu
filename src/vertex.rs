#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]

pub struct Vertex {
    position: [f32; 2],
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    position: [f32; 2],
}

pub const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, -0.5],
    },
    Vertex {
        position: [0.5, -0.5],
    },
    Vertex {
        position: [0.5, 0.5],
    },
    Vertex {
        position: [-0.5, 0.5],
    },
];

pub const ROWS: usize = 10;
pub const COLS: usize = 10;
pub const GAP: f32 = 0.02;

pub const fn get_instances() -> [Instance; ROWS * COLS] {
    let mut arr = [Instance {
        position: [0.0, 0.0],
    }; ROWS * COLS];
    let mut i = 0;
    let cell_size = (2.0 - (COLS as f32 - 1.0) * GAP) / COLS as f32;
    while i < ROWS * COLS {
        let row = i / COLS;
        let col = i % COLS;
        // x and y are the NDC coords of the center of the cells
        let x = -1.0 + cell_size / 2.0 + col as f32 * (cell_size + GAP);
        let y = -1.0 + cell_size / 2.0 + row as f32 * (cell_size + GAP);
        // now make the position
        arr[i] = Instance { position: [x, y] };
        i += 1;
    }
    arr
}

pub const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            }],
        }
    }
}
