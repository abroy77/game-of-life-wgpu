use crate::constants::{ROWS, COLS, GAP, CELL_SIZE};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    cell_size: [f32;2],
    gap: f32,
    rows: u32,
    cols: u32,
    _pad: u32
}

impl Uniforms {
    pub fn new() -> Self {
        Self {
            cell_size: [CELL_SIZE,CELL_SIZE],
            gap: GAP,
            rows: ROWS as u32,
            cols: COLS as u32,
            _pad: 0
        }
    }
}


