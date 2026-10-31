//! Painting live cells with the mouse or a finger.
//!
//! Input events append line segments (previous cell → current cell) to a CPU list.
//! Every `paint_fps` tick, [`crate::graphics::GraphicsContext::paint()`] uploads them and runs
//! `paint.wgsl`, which draws each segment with the DDA (Digital Differential Analyzer) line
//! algorithm, setting its cells to newborns (1). If nothing was painted, the pass is skipped.
//! Painting runs on its own timer, so you can draw smoothly even while the simulation is paused or slow.

use std::num::NonZeroU64;
use std::sync::Arc;
use winit::dpi::LogicalPosition;
use winit::window::Window;

use crate::config::AppConfig;

/// Max segments per paint tick; very hard for a human to exceed this.
const MAX_SEGMENTS: usize = 1024;

/// `Path` header in paint.wgsl: count + 4 bytes padding so the array is 8-byte aligned.
const HEADER_BYTES: u64 = 8;

/// Input state plus the GPU resources for painting.
pub struct MousePainter {
    /// Is the cursor over the window?
    pub in_grid: bool,
    /// Is the left button (or the tracked finger) held down?
    pub is_pressed: bool,
    /// Last cursor position in logical pixels, origin top-left, y pointing down.
    pub pos: LogicalPosition<f64>,
    /// Segments painted since the last upload.
    pub segments: Vec<Segment>,
    /// GPU copy of the paint buffer, read by `paint.wgsl` at `@group(2)`.
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
/// Line segment from cell `a` to cell `b`. Matches `Segment` in paint.wgsl.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Segment {
    a: [i32; 2],
    b: [i32; 2],
}

impl Segment {
    /// DDA steps from `a` to `b`: the length along the longer axis.
    fn steps(&self) -> u32 {
        self.a[0]
            .abs_diff(self.b[0])
            .max(self.a[1].abs_diff(self.b[1]))
    }
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
    /// Create the paint buffers and the paint compute pipeline.
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
        //   @group(2) paint buffer         (created below)
        // It sets every cell along each uploaded segment to 1.

        // The paint buffer: a fixed-size header + MAX_SEGMENTS segments. STORAGE so the shader
        // can read it, COPY_DST so we can upload new strokes each paint tick.
        let segments = Vec::with_capacity(MAX_SEGMENTS);
        let paint_buffer_gpu = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Painter Buffer"),
            size: HEADER_BYTES + (MAX_SEGMENTS * size_of::<Segment>()) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Layout and bind group for the paint buffer (`@group(2) @binding(0)`). The shader only reads it.
        let painter_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Painter Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(
                            HEADER_BYTES + size_of::<Segment>() as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let painter_buffer_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Painter Bind Group"),
            layout: &painter_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: paint_buffer_gpu.as_entire_binding(),
            }],
        });

        // Pipeline layout in @group order: uniform, grid, paint buffer.
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
            segments,
            paint_buffer_gpu,
            array_div_factor,
            painter_pipeline,
            painter_buffer_bind_group,
            finger_id,
            previous_paint_cell: None,
        }
    }
    /// Add a segment from the previous cell in this stroke to the cell under the cursor (`self.pos`).
    pub fn add_to_buffer(&mut self, config: &AppConfig) {
        let Some(cell) = self.pos_to_cell(config) else {
            // Off the grid: end the stroke.
            self.previous_paint_cell = None;
            return;
        };
        // First cell of a stroke is a zero-length segment.
        let src = self.previous_paint_cell.unwrap_or(cell);
        self.push_segment(src, cell);
        // Remember this cell, so the next event in this stroke draws a line from here.
        self.previous_paint_cell = Some(cell);
    }
    /// Queue a segment for the next upload.
    fn push_segment(&mut self, src: (usize, usize), dst: (usize, usize)) {
        if self.segments.len() == MAX_SEGMENTS {
            return;
        }
        let b = [dst.0 as i32, dst.1 as i32];
        let a = [src.0 as i32, src.1 as i32];
        let seg = Segment { a, b };
        self.segments.push(seg);
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
    /// Upload this tick's segments to the GPU and clear them.
    ///
    /// Returns the threads needed as `(points per segment, segments)`, or `None` if nothing was painted.
    pub fn write_to_buffer(&mut self, queue: &wgpu::Queue) -> Option<(u32, u32)> {
        if self.segments.is_empty() {
            return None;
        }
        let count = self.segments.len() as u32;
        let max_steps = self.segments.iter().map(Segment::steps).max().unwrap_or(0);
        // Header: count, then 4 bytes padding so the segments start 8-byte aligned.
        queue.write_buffer(
            &self.paint_buffer_gpu,
            0,
            bytemuck::cast_slice(&[count, 0u32]),
        );
        queue.write_buffer(
            &self.paint_buffer_gpu,
            HEADER_BYTES,
            bytemuck::cast_slice(&self.segments),
        );
        self.segments.clear();
        // steps + 1 points per segment: both ends are inclusive.
        Some((max_steps + 1, count))
    }
}
