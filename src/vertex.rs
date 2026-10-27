//! The geometry we draw: one square, repeated once per cell.
//!
//! Instead of uploading `rows * cols * 4` corners, we are using **instancing**: Upload one square ([`CELL_VERTICES`])
//! and one position per cell ([`Instance`]). The GPU then draws the square many times,
//! each time moved to the next instance's position.

/// One corner of the cell square, relative to the square's centre.
///
/// `#[repr(C)]` + `bytemuck::Pod` let us upload a `&[Vertex]` straight into a GPU buffer.
/// `Pod` stands for Plain Old Data. assures the compiler that we're storing data not references /
/// objects with interior mutability. Just data.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 2],
}
/// The 4 corners of a unit square centred on (0, 0), in anticlockwise order.
pub const CELL_VERTICES: &[Vertex] = &[
    Vertex {
        // bottom left
        position: [-0.5, -0.5],
    },
    Vertex {
        // bottom right
        position: [0.5, -0.5],
    },
    Vertex {
        // top right
        position: [0.5, 0.5],
    },
    Vertex {
        // top left
        position: [-0.5, 0.5],
    },
];

/// Cell's centre position in clip space.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    position: [f32; 2],
}

/// The centre of every cell in clip space, row by row starting at the bottom.
///
/// Instance `i` is the cell at `col = i % cols`, `row = i / cols` ie **Row-major** (C style) ordering.
/// The render shader can use the instance index to look up the cell's state.
pub fn get_instances(
    rows: usize,
    cols: usize,
    gap_size: (f32, f32),
    cell_size: (f32, f32),
) -> Vec<Instance> {
    let mut result = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            // Start at the left/bottom edge (-1), skip the first gap and half a cell to reach
            // the first centre, then step one (cell + gap) per column/row. Eg:
            // x_center(i=0) = -1 + gap + half a cell's x-width
            // x_center(i=n) = -1 + gap + half a cell's x-width + n * (cell_size + gap_size)
            let x = -1.0 + gap_size.0 + cell_size.0 / 2.0 + col as f32 * (cell_size.0 + gap_size.0);
            let y = -1.0 + gap_size.1 + cell_size.1 / 2.0 + row as f32 * (cell_size.1 + gap_size.1);

            result.push(Instance { position: [x, y] });
        }
    }
    result
}

/// GPUs draw triangles, so the square is two triangles: (0, 1, 2) and (2, 3, 0).
/// ie (bottom left, bottom right, top right) and (top right, top left, bottom left).
/// An **index buffer** lets the triangles share corners instead of repeating them.
/// Both triangles are anticlockwise, which the render pipeline treats as front-facing.
pub const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

impl Vertex {
    /// Tells the render pipeline how to read a [`Vertex`] buffer.
    ///
    /// - `array_stride`: bytes from one vertex to the next.
    /// - `step_mode: Vertex`: move to the next entry for each *corner*.
    /// - `shader_location: 0`: shows up as `@location(0) position` in `render.wgsl`.
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
    /// Tells the render pipeline how to read an [`Instance`] buffer.
    ///
    /// - `step_mode: Instance`: move to the next entry for each *square*, not each corner.
    /// - `shader_location: 1`: shows up as `@location(1) instance_position` in `render.wgsl`.
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
