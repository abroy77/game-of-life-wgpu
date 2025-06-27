use crate::vertex::{INDICES, CELL_VERTICES, get_instances};
use crate::uniforms::RenderUniforms;
use crate::constants::{COLS, INITIAL_STATE, ROWS};
use wgpu::util::DeviceExt;

pub struct Buffers {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub instance: wgpu::Buffer,
    pub uniform: wgpu::Buffer,
    pub current_state: wgpu::Buffer,
    pub next_state: wgpu::Buffer,
    pub num_indices: u32,
}

impl Buffers {
    pub fn new(device: &wgpu::Device) -> Self {
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
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instances),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
   
        let uniform = RenderUniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        let num_indices = INDICES.len() as u32;

        // Flatten INITIAL_STATE (assumed [[bool; COLS]; ROWS]) into a Vec<u8>
        let flat_initial_state: Vec<u8> = INITIAL_STATE
            .iter()
            .flat_map(|row| row.iter().map(|&b| b as u8))
            .collect();

        let current_state = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("current state buffer"),
            contents: bytemuck::cast_slice(&flat_initial_state),
            usage: wgpu::BufferUsages::STORAGE 
        });

        let next_state = device.create_buffer(&wgpu::BufferDescriptor{
            label: Some("next state buffer"),
            size: flat_initial_state.len() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false
        });

        Self {
            vertex: vertex_buffer,
            index: index_buffer,
            instance: instance_buffer,
            uniform: uniform_buffer,
            num_indices,
            current_state,
            next_state

        }
    }
}
