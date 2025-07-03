use wgpu::util::DeviceExt;
use crate::{
    vertex::{CELL_VERTICES, INDICES, get_instances},
    uniforms::{RenderUniforms, ComputeUniforms},
    pipeline::{create_render_pipeline, create_compute_pipeline},
    resources::{RenderLayouts, ComputeLayouts},
};

pub struct GpuResources {
    // Buffers
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub render_uniform_buffer: wgpu::Buffer,
    pub compute_uniform_buffer: wgpu::Buffer,
    pub current_state_buffer: wgpu::Buffer,
    pub next_state_buffer: wgpu::Buffer,
    
    // Bind groups
    pub render_bind_group: wgpu::BindGroup,
    pub compute_uniform_bind_group: wgpu::BindGroup,
    pub compute_state_bind_group: wgpu::BindGroup,
    pub render_game_state_bind_group: wgpu::BindGroup,
    
    // Pipelines
    pub render_pipeline: wgpu::RenderPipeline,
    pub compute_pipeline: wgpu::ComputePipeline,
    
    // Metadata
    pub num_indices: u32,

    // Add layouts for recreating bind groups
    render_layouts: RenderLayouts,
    compute_layouts: ComputeLayouts,
    device: wgpu::Device,
}

impl GpuResources {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, initial_state: &[u32]) -> Self {
        
        // Create buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(CELL_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let instances = get_instances();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let render_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Render Uniform Buffer"),
            contents: bytemuck::cast_slice(&[RenderUniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let compute_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Compute Uniform Buffer"),
            contents: bytemuck::cast_slice(&[ComputeUniforms::new()]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let current_state_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Current State Buffer"),
            contents: bytemuck::cast_slice(initial_state),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let next_state_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Next State Buffer"),
            size: (initial_state.len() * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Create layouts
        let render_layouts = RenderLayouts::new(device);
        let compute_layouts = ComputeLayouts::new(device);

        // Create bind groups
        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_layouts.uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: render_uniform_buffer.as_entire_binding(),
            }],
        });

        let compute_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Uniform Bind Group"),
            layout: &compute_layouts.uniform,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: compute_uniform_buffer.as_entire_binding(),
            }],
        });

        let compute_state_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute State Bind Group"),
            layout: &compute_layouts.game_state,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: current_state_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: next_state_buffer.as_entire_binding(),
                },
            ],
        });

        let render_game_state_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Game State Bind Group"),
            layout: &render_layouts.game_state,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: current_state_buffer.as_entire_binding(),
            }],
        });

        // Create pipelines
        let render_pipeline = create_render_pipeline(device, surface_format, &render_layouts);
        let compute_pipeline = create_compute_pipeline(device, &compute_layouts);

        Self {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            render_uniform_buffer,
            compute_uniform_buffer,
            current_state_buffer,
            next_state_buffer,
            render_bind_group,
            compute_uniform_bind_group,
            compute_state_bind_group,
            render_game_state_bind_group,
            render_layouts,
            compute_layouts,
            device: device.clone(),
            render_pipeline,
            compute_pipeline,
            num_indices: INDICES.len() as u32,
        }
    }

    pub fn swap_buffers(&mut self) {
        // Swap the actual buffers
        std::mem::swap(&mut self.current_state_buffer, &mut self.next_state_buffer);
        
        // Recreate bind groups with swapped buffers
        self.compute_state_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute State Bind Group"),
            layout: &self.compute_layouts.game_state,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.current_state_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.next_state_buffer.as_entire_binding(),
                },
            ],
        });

        self.render_game_state_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Game State Bind Group"),
            layout: &self.render_layouts.game_state,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: self.current_state_buffer.as_entire_binding(),
            }],
        });
    }
}
