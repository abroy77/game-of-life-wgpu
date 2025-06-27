use crate::constants::{ROWS, COLS, GAP, CELL_SIZE};
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

pub const CELL_VERTICES: &[Vertex] = &[
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
pub const fn get_instances() -> [Instance; ROWS * COLS] {
    let mut arr = [Instance {
        position: [0.0, 0.0],
    }; ROWS * COLS];
    let mut i = 0;
    while i < ROWS * COLS {
        let row = i / COLS;
        let col = i % COLS;
        // x and y are the NDC coords of the center of the cells
        let x = -1.0 + GAP + CELL_SIZE / 2.0 + col as f32 * (CELL_SIZE + GAP);
        let y = -1.0 + GAP + CELL_SIZE / 2.0 + row as f32 * (CELL_SIZE + GAP);
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
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

impl Instance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ],
        }
    }
}
