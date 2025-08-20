use log::info;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::window::Window;

use crate::config::AppConfig;

// const PAINTER_BUFFER_CLEAR_INTERVAL: Duration = Duration::from_millis(16); // 16 fps

pub struct MousePainter {
    pub in_grid: bool,
    pub is_pressed: bool,
    pub pos: PhysicalPosition<f64>,
    pub paint_buffer_cpu: Vec<u32>,
    pub paint_buffer_gpu: wgpu::Buffer,
    pub array_div_factor: (usize, usize),
    pub painter_pipeline: wgpu::ComputePipeline,
    pub painter_buffer_bind_group: wgpu::BindGroup,
    pub scale_factor: f64,
}

fn get_window_logical_size(window: &Arc<Window>) -> (f32, f32) {
    // let log = window.inner_size().to_logical(window.scale_factor());
    let log = window.inner_size();
    let (x, y) = (log.width as f32, log.height as f32);
    return (x, y);
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
        let window_size = get_window_logical_size(&window);
        info!("new mousey size {} {}", window_size.0, window_size.1);
        let array_div_factor = (
            window_size.0 as usize / config.cols,
            window_size.1 as usize / config.rows,
        );
        let scale_factor = window.scale_factor();
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
                    &compute_bind_group_layout,
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

        Self {
            in_grid: false,
            is_pressed: false,
            pos: PhysicalPosition { x: 0.0, y: 0.0 },
            paint_buffer_cpu: paint_buffer,
            paint_buffer_gpu: painter_buffer_gpu,
            array_div_factor,
            painter_pipeline,
            painter_buffer_bind_group,
            scale_factor,
        }
    }
    // set the cell array position to 1
    pub fn add_to_buffer(&mut self, config: &AppConfig) {
        // we need to convert the physical coords into the array index for the cell
        let (div_x, div_y) = self.array_div_factor;
        let x = self.pos.x.round() as usize / div_x;
        // we do this because NDC is from down to up in y.
        // but the window coordinates are top to bottow
        let y = config.rows - 1 - self.pos.y.round() as usize / div_y;
        // now get the array_pos:
        let array_pos = x + config.cols * y;
        if array_pos < self.paint_buffer_cpu.len() {
            self.paint_buffer_cpu[array_pos] = 1;
        }
    }
    pub fn clear_buffer(&mut self) {
        self.paint_buffer_cpu.iter_mut().for_each(|x| *x = 0);
    }
    pub fn configure(&mut self, window: &Arc<Window>, config: &AppConfig) {
        let window_size = get_window_logical_size(window);
        self.array_div_factor = (
            window_size.0 as usize / config.cols,
            window_size.1 as usize / config.rows,
        );
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
