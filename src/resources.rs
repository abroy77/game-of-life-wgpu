use wgpu;

pub struct BindGroups {
    pub uniform: wgpu::BindGroup,
}

pub struct Layouts {
    pub uniform: wgpu::BindGroupLayout,
}

impl Layouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { 
                    ty: wgpu::BufferBindingType::Uniform, 
                    has_dynamic_offset: false, 
                    min_binding_size: None 
                },
                count: None
            }]
        });

        Self {
            uniform: uniform_bind_group_layout,
        }
    }
}

impl BindGroups {
    pub fn new(device: &wgpu::Device, layouts: &Layouts, buffers: &crate::buffers::Buffers) -> Self {
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor{
            label: Some("Uniform Bind Group"),
            layout: &layouts.uniform,
            entries: &[wgpu::BindGroupEntry{
                binding: 0, 
                resource: buffers.uniform.as_entire_binding()
            }]
        });

        Self {
            uniform: uniform_bind_group,
        }
    }
}
