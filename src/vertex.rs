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

pub fn get_instances(
    rows: usize,
    cols: usize,
    gap_size: (f32, f32),
    cell_size: (f32, f32),
) -> Vec<Instance> {
    let mut result = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            let x = -1.0 + gap_size.0 + cell_size.0 / 2.0 + col as f32 * (cell_size.0 + gap_size.0);
            let y = -1.0 + gap_size.1 + cell_size.1 / 2.0 + row as f32 * (cell_size.1 + gap_size.1);

            result.push(Instance { position: [x, y] });
        }
    }
    result
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
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}
