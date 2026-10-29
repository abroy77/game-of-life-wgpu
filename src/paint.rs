//! Painting live cells with the mouse or a finger.
//!
//! Input events mark cells in a CPU-side **paint mask** (`Vec<u32>`, same layout as the grid).
//! Every `paint_fps` run / tick, [`crate::graphics::GraphicsContext::paint()`] uploads the mask and runs
//! `paint.wgsl`, which sets painted cells to newborns (1) in the current grid. The mask is then cleared.
//! Painting runs on its own timer, so you can draw smoothly even while the simulation is paused or slow.

use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::dpi::LogicalPosition;
use winit::window::Window;

use crate::config::AppConfig;

/// Input state plus the GPU resources for painting.
pub struct MousePainter {
    /// Is the cursor over the window?
    pub in_grid: bool,
    /// Is the left button (or the tracked finger) held down?
    pub is_pressed: bool,
    /// Last cursor position in logical pixels, origin top-left, y pointing down.
    pub pos: LogicalPosition<f64>,
    /// Cells painted since the last upload: 1 = paint this cell alive.
    pub paint_buffer_cpu: Vec<u32>,
    /// GPU copy of the paint mask, read by `paint.wgsl` at `@group(2)`.
    pub paint_buffer_gpu: wgpu::Buffer,
    /// Logical pixels per cell in x and y. Dividing a cursor position by this gives a cell coordinate.
    pub array_div_factor: (f32, f32),
    /// Compiled `paint.wgsl`.
    pub painter_pipeline: wgpu::ComputePipeline,
    /// Bind group holding `paint_buffer_gpu`.
    pub painter_buffer_bind_group: wgpu::BindGroup,
    /// The finger we're following on a touch screen, so a second finger doesn't jump the stroke.
    pub finger_id: Option<u64>,
    /// previous position of cursor in a single stroke.
    /// Reverts to None when a stroke ends.
    pub previous_paint_cell: Option<(usize, usize)>,
}

/// Window size in logical pixels (physical pixels / display scale factor).
fn get_window_logical_size(window: &Arc<Window>) -> (f32, f32) {
    let physical_size = window.inner_size();
    let scale_factor = window.scale_factor();
    let log = physical_size.to_logical::<f32>(scale_factor);
    let (x, y) = (log.width, log.height);
    (x, y)
}
impl MousePainter {
    /// Create the paint mask buffers and the paint compute pipeline.
    ///
    /// The two layouts passed in come from [`crate::game_data::GameData`], so the paint shader
    /// can use the same uniform and grid bind groups as the simulation. Aha! finally a way to reuse something.
    pub fn new(
        device: &wgpu::Device,
        // Grid buffers, `@group(1)`. The paint shader writes into binding 0 (current state).
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        // Grid size uniform, `@group(0)`.
        compute_uniform_bind_group_layout: &wgpu::BindGroupLayout,
        config: &AppConfig,
        window: Arc<Window>,
    ) -> Self {
        let array_div_factor = MousePainter::calc_array_div_factor(&window, config);
        // The paint shader uses three bind groups:
        //   @group(0) grid size uniform    (shared with the GameData)
        //   @group(1) grid state buffers   (shared with the GameData, written to here as well)
        //   @group(2) paint mask buffer    (created below)
        // It does `state = state | paint_mask` for every cell.

        // The paint mask buffer, all zeros. STORAGE so the shader can read it,
        // COPY_DST so we can upload new strokes each paint tick.
        let paint_buffer = vec![0; config.num_elements()];
        let painter_buffer_gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Painter Buffer"),
            contents: bytemuck::cast_slice(&paint_buffer),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        // Layout and bind group for the paint mask (`@group(2) @binding(0)`). The shader only reads it.
        let painter_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Painter Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let painter_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Painter Bind Group"),
            layout: &painter_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: painter_buffer_gpu.as_entire_binding(),
            }],
        });

        // Pipeline layout in @group order: uniform, grid, paint mask.
        let painter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Painter Pipeline Layout"),
                bind_group_layouts: &[
                    Some(compute_uniform_bind_group_layout),
                    Some(compute_bind_group_layout),
                    Some(&painter_bind_group_layout),
                ],
                immediate_size: 0,
            });

        // Compile the shader and build the compute pipeline.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Painter shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/paint.wgsl").into()),
        });

        let painter_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Painter Pipeline"),
            layout: Some(&painter_pipeline_layout),
            cache: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        let finger_id = None;

        Self {
            in_grid: false,
            is_pressed: false,
            pos: LogicalPosition { x: 0.0, y: 0.0 },
            paint_buffer_cpu: paint_buffer,
            paint_buffer_gpu: painter_buffer_gpu,
            array_div_factor,
            painter_pipeline,
            painter_buffer_bind_group,
            finger_id,
            previous_paint_cell: None,
        }
    }
    /// Mark the cell under the cursor (`self.pos`) in the paint mask.
    pub fn add_to_buffer(&mut self, config: &AppConfig) {
        let Some(cell) = self.pos_to_cell(config) else {
            // mark None when out of bounds.
            self.previous_paint_cell = None;
            return;
        };
        // if we're in the grid then
        match self.previous_paint_cell {
            Some(prev_cell) => self.mark_line(prev_cell, cell, config),
            None => self.mark_cell(cell, config),
        }
        // Remember this cell, so the next event in this stroke draws a line from here.
        self.previous_paint_cell = Some(cell);
    }
    /// Convert the received logical pixel to cell coordinates in the grid.
    /// None if out of bounds.
    pub fn pos_to_cell(&self, config: &AppConfig) -> Option<(usize, usize)> {
        // Left of or above the window. Without this, `as usize` would turn the negative
        // value into 0 and wrongly map it to the edge column / top row.
        if self.pos.x < 0.0 || self.pos.y < 0.0 {
            return None;
        }
        // Convert the logical pixel position into a column and row.
        let (div_x, div_y) = self.array_div_factor;
        let x = (self.pos.x as f32 / div_x) as usize;
        if x >= config.cols {
            return None;
        }
        // Window y goes top → bottom, but the grid's row 0 is at the bottom (clip space y points up),
        // so we flip it. `checked_sub` avoids underflow if the cursor is below the bottom most row.
        // The result is at most `rows - 1`.
        let y = (config.rows - 1).checked_sub((self.pos.y as f32 / div_y) as usize);

        y.map(|y| (x, y))
    }
    /// Mark a single cell as `on` by its grid coordinates.
    pub fn mark_cell(&mut self, pos: (usize, usize), config: &AppConfig) {
        let array_pos = pos.0 + config.cols * pos.1;
        self.paint_buffer_cpu[array_pos] = 1;
    }
    /// Mark every cell on a straight line from `from` to `to` inclusive.
    ///
    /// [`Bresenham's line algorithm for all cases`](https://en.wikipedia.org/wiki/Bresenham%27s_line_algorithm#All_cases)
    /// which walks one cell at a time either in x, y or both from start to end
    pub fn mark_line(&mut self, from: (usize, usize), to: (usize, usize), config: &AppConfig) {
        // starting x, y which will update as we move the plotter
        // using i64 as step directions can be -ve
        let (mut x, mut y) = (from.0 as i64, from.1 as i64);
        // destination
        let (x1, y1) = (to.0 as i64, to.1 as i64);

        // How far the line goes in x and y. directions handled by sx, sy.
        // Since the sx,sy are handling direction we can think of dx as how far
        // right the line goes and dy as how far up.
        // dy is stored as negative so that we can have one `err` value
        // updating instead of 2 separate ones
        let dx = (x1 - x).abs();
        let dy = -(y1 - y).abs(); // negative
        // initial err value. +ve means shallow line. -ve means steep line
        // at least at the start
        let mut err = dx + dy;

        // steps
        let sx = if x < x1 { 1 } else { -1 };
        let sy = if y < y1 { 1 } else { -1 };

        // err      = how far the diagonal cell D (x+1, y+1) is off the true line (scaled to whole numbers)
        // 2·err ≤ dx   → D within ½ cell vertically   → move in y
        // 2·err ≥ dy   → D within ½ cell horizontally → move in x
        // both         → diagonal step
        // moved in x   → err += dy  (D got closer horizontally)
        // moved in y   → err += dx  (D got higher)
        // sx, sy       → turn "right/up" into the real direction
        loop {
            self.mark_cell((x as usize, y as usize), config);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy; // dy is -ve so this reduces err
                x += sx;
            }
            if e2 <= dx {
                err += dx; // increases err
                y += sy;
            }
        }
    }
    /// Zero the paint mask after it has been uploaded.
    /// #TODO: this is currently quite a heavy operation to be doing for each paint pass.
    /// Need to find a better way to encode and send the paint buffer points to GPU. maybe as indices perhaps?
    /// rather than the whole mask? that way when there's no painting then I can just not do this clear step.
    pub fn clear_buffer(&mut self) {
        self.paint_buffer_cpu.iter_mut().for_each(|x| *x = 0);
    }
    /// Logical pixels per cell: window size divided by the number of columns/rows.
    pub fn calc_array_div_factor(window: &Arc<Window>, config: &AppConfig) -> (f32, f32) {
        let window_size = get_window_logical_size(window);
        (
            window_size.0 / config.cols as f32,
            window_size.1 / config.rows as f32,
        )
    }
    /// Recompute the pixel → cell mapping. Call when the window is resized or its scale changes.
    pub fn configure(&mut self, window: &Arc<Window>, config: &AppConfig) {
        self.array_div_factor = MousePainter::calc_array_div_factor(window, config);
    }
    /// Upload the CPU paint mask to the GPU buffer.
    /// #TODO: also a very heavy operation, definitely should be some way to improve this.
    pub fn write_to_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.paint_buffer_gpu,
            0,
            bytemuck::cast_slice(&self.paint_buffer_cpu),
        );
    }
}
