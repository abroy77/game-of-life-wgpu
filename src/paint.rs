use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::dpi::LogicalPosition;
use winit::window::Window;

use crate::config::AppConfig;

pub struct MousePainter {
    pub in_grid: bool,
    pub is_pressed: bool,
    pub pos: LogicalPosition<f64>,
    pub paint_buffer_cpu: Vec<u32>,
    pub paint_buffer_gpu: wgpu::Buffer,
    pub array_div_factor: (f32, f32),
    pub painter_pipeline: wgpu::ComputePipeline,
    pub painter_buffer_bind_group: wgpu::BindGroup,
    pub finger_id: Option<u64>,
}

fn get_window_logical_size(window: &Arc<Window>) -> (f32, f32) {
    let physical_size = window.inner_size();
    let scale_factor = window.scale_factor();
    let log = physical_size.to_logical::<f32>(scale_factor);
    let (x, y) = (log.width, log.height);

    log::info!(
        "Window dimensions - Physical: {:?}, Scale factor: {}, Logical: {}x{}",
        physical_size,
        scale_factor,
        x,
        y
    );

    (x, y)
}
impl MousePainter {
    pub fn new(
        device: &wgpu::Device,
        //render buffer is bound at 0
        compute_bind_group_layout: &wgpu::BindGroupLayout,
        // compute uniform bound at 0
        compute_uniform_bind_group_layout: &wgpu::BindGroupLayout,
        config: &AppConfig,
        window: Arc<Window>,
    ) -> Self {
        let array_div_factor = MousePainter::calc_array_div_factor(&window, config);
        // so here we don't need a premade buffer. we will make it on the fly from our slice.
        // do we need a buffer layout? nah
        // we do need a bind group so we can bind 2 things in our paint shader
        // 1. the uniform
        // 2. the paint buffer
        // 3. the current render buffer
        //
        // in our shader we will write to the current render_buffer, and the
        // written value is the union of the render_buffer and the paint buffer.
        // then we will re render the page.
        //
        //
        // The buffer writing will take place at 60fps. decoupled from the update fps.

        let paint_buffer = vec![0; config.num_elements()];
        let painter_buffer_gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Painter Buffer"),
            contents: bytemuck::cast_slice(&paint_buffer),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

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

        let painter_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Painter Pipeline Layout"),
                bind_group_layouts: &[
                    compute_uniform_bind_group_layout,
                    compute_bind_group_layout,
                    &painter_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

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
    // set the cell array position to 1
    pub fn add_to_buffer(&mut self, config: &AppConfig) {
        // we need to convert the physical coords into the array index for the cell
        let (div_x, div_y) = self.array_div_factor;
        let x = (self.pos.x as f32 / div_x) as usize;
        if x >= config.cols {
            return;
        }
        // we do this because NDC is from down to up in y.
        // but the window coordinates are top to bottom
        // need to do a checked subtraction to prevent overflow issues
        let y = (config.rows - 1).checked_sub((self.pos.y as f32 / div_y) as usize);

        log::info!(
            "Paint calculation - Pos: {:?}, Div factor: {:?}, Grid coords: ({}, {:?})",
            self.pos,
            self.array_div_factor,
            x,
            y
        );

        if let Some(y) = y {
            // now get the array_pos:
            let array_pos = x + config.cols * y;
            log::info!(
                "Paint buffer - Array pos: {}, Buffer len: {}",
                array_pos,
                self.paint_buffer_cpu.len()
            );

            if array_pos < self.paint_buffer_cpu.len() {
                self.paint_buffer_cpu[array_pos] = 1;
                log::info!("Successfully painted at grid position ({}, {})", x, y);
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
    pub fn clear_buffer(&mut self) {
        self.paint_buffer_cpu.iter_mut().for_each(|x| *x = 0);
    }
    pub fn calc_array_div_factor(window: &Arc<Window>, config: &AppConfig) -> (f32, f32) {
        let window_size = get_window_logical_size(window);
        let div_factor = (
            window_size.0 / config.cols as f32,
            window_size.1 / config.rows as f32,
        );

        log::info!(
            "Array div factor calculation - Window: {}x{}, Grid: {}x{}, Div factor: {:?}",
            window_size.0,
            window_size.1,
            config.cols,
            config.rows,
            div_factor
        );

        div_factor
    }
    pub fn configure(&mut self, window: &Arc<Window>, config: &AppConfig) {
        self.array_div_factor = MousePainter::calc_array_div_factor(window, config);
    }
    // configure the buffer size and the div factor when we resize the window
    // or change num elements.
    pub fn write_to_buffer(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.paint_buffer_gpu,
            0,
            bytemuck::cast_slice(&self.paint_buffer_cpu),
        );
    }
}
