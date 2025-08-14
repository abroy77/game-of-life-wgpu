#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};
#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use winit::dpi::PhysicalPosition;

const PAINTER_BUFFER_CLEAR_INTERVAL: Duration = Duration::from_millis(16); // 16 fps

pub struct MousePainter {
    pub in_grid: bool,
    pub is_pressed: bool,
    pub pos: PhysicalPosition<f64>,
    pub paint_buffer: Vec<u32>,
    pub next_buffer_clear: Instant,
    pub array_div_factor: Option<(usize, usize)>,
    pub num_cols: usize,
}

impl MousePainter {
    pub fn new() -> Self {
        Self {
            in_grid: false,
            is_pressed: false,
            pos: PhysicalPosition { x: 0.0, y: 0.0 },
            paint_buffer: vec![],
            next_buffer_clear: Instant::now() + PAINTER_BUFFER_CLEAR_INTERVAL,
            array_div_factor: None,
            num_cols: 0,
        }
    }
    // set the cell array position to 1
    pub fn add_to_buffer(&mut self) {
        // we need to convert the physical coords into the array index for the cell
        if let Some((div_x, div_y)) = self.array_div_factor {
            let x = self.pos.x.round() as usize / div_x;
            let y = self.pos.y.round() as usize / div_y;
            // now get the array_pos:
            let array_pos = x + self.num_cols * y;
            println!("array_pos: {array_pos}");
            self.paint_buffer[array_pos] = 1;
        }
    }
    pub fn clear_buffer(&mut self) {
        self.paint_buffer.iter_mut().for_each(|x| *x = 0);
    }
    // configure the buffer size and the div factor when we resize the window
    // or change num elements.
    pub fn configure(&mut self, grid_size: (usize, usize), window_size: (usize, usize)) {
        self.num_cols = grid_size.0;
        self.paint_buffer = vec![0; grid_size.0 * grid_size.1];
        self.array_div_factor = Some((window_size.0 / grid_size.0, window_size.1 / grid_size.1));
    }
    pub fn paint(&mut self, queue: &wgpu::Queue, current_buffer: wgpu::Buffer) {
        queue.write_buffer(&current_buffer, 0, bytemuck::cast_slice(&self.paint_buffer));
    }
}
