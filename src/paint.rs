//! Painting live cells with the mouse or a finger.
//!
//! Input events mark cells in a CPU-side **paint mask** (`Vec<u32>`, same layout as the grid).
//! Every `paint_fps` run / tick, [`crate::graphics::GraphicsContext::paint()`] uploads the mask and runs
//! `paint.wgsl`, which ORs it into the current grid. The mask is then cleared.
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
        }
    }
    /// Mark the cell under the cursor (`self.pos`) in the paint mask.
    pub fn add_to_buffer(&mut self, config: &AppConfig) {
        // Convert the logical pixel position into a column and row.
        let (div_x, div_y) = self.array_div_factor;
        let x = (self.pos.x as f32 / div_x) as usize;
        if x >= config.cols {
            return;
        }
        // Window y goes top → bottom, but the grid's row 0 is at the bottom (clip space y points up),
        // so we flip it. `checked_sub` avoids underflow if the cursor is below the bottom most row.
        let y = (config.rows - 1).checked_sub((self.pos.y as f32 / div_y) as usize);

        if let Some(y) = y {
            // Same row-by-row indexing as the grid buffers.
            let array_pos = x + config.cols * y;

            if array_pos < self.paint_buffer_cpu.len() {
                self.paint_buffer_cpu[array_pos] = 1;
            } else {
                log::warn!(
                    "Paint position out of bounds: array_pos {} >= buffer_len {}",
                    array_pos,
                    self.paint_buffer_cpu.len()
                );
            }
        } else {
            log::warn!("Invalid Y coordinate calculation");
        }
    }
    /// Zero the paint mask after it has been uploaded.
    /// #TODO: this is currently quite a heavy opertion to be doing for each paint pass.
    /// Need to find a better way to encode and send the paint buffer points to GPU. maybe as indeces perhaps?
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
