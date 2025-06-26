use crate::vertex::{INDICES, VERTICES, Instance, get_instances};
use crate::uniforms::Uniforms;
use wgpu::util::DeviceExt;

pub struct Buffers {
    pub vertex: wgpu::Buffer,
    pub index: wgpu::Buffer,
    pub instance: wgpu::Buffer,
    pub uniform: wgpu::Buffer,
    pub num_indices: u32,
}

impl Buffers {
    pub fn new(device: &wgpu::Device) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
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
   
        let uniform = Uniforms::new();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST
        });

        let num_indices = INDICES.len() as u32;

        Self {
            vertex: vertex_buffer,
            index: index_buffer,
            instance: instance_buffer,
            uniform: uniform_buffer,
            num_indices,
        }
    }
}
