use wgpu::BindGroupLayoutDescriptor;

pub struct RenderLayouts {
    pub uniform: wgpu::BindGroupLayout,
    pub game_state: wgpu::BindGroupLayout,
}

pub struct ComputeLayouts {
    pub game_state: wgpu::BindGroupLayout,
    pub uniform: wgpu::BindGroupLayout,
}


impl RenderLayouts {
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

        let game_state_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Game State Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { 
                    ty: wgpu::BufferBindingType::Storage { read_only: true }, 
                    has_dynamic_offset: false, 
                    min_binding_size: None 
                },
                count: None
            }]
        });

        Self {
            uniform: uniform_bind_group_layout,
            game_state: game_state_bind_group_layout,
        }
    }
}


impl ComputeLayouts {
    pub fn new(device: &wgpu::Device) -> Self {
        let current_state_entry = wgpu::BindGroupLayoutEntry{
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
            count: None

        };


        let next_state_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
            count: None
        };

        let game_state_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                label: Some("Game state bind group layout"),
                entries: &[
                    current_state_entry,
                    next_state_entry
                ]
            }
        );

        let uniform_layout = device.create_bind_group_layout(
            &BindGroupLayoutDescriptor {
                label: Some("Compute uniform bind group layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry{
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                        count: None
                    }
                ]
            }
        );
        return Self { game_state: game_state_layout, uniform: uniform_layout }
    }
}
