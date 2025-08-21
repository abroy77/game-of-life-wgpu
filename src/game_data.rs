// use crate::config::CONFIG;
use rand::{Rng, rng, rngs::ThreadRng};
use wgpu::util::DeviceExt;

use crate::config::AppConfig;

pub struct GameData {
    rng: ThreadRng,
    pub game_state_buffer_a: wgpu::Buffer,
    pub game_state_buffer_b: wgpu::Buffer,
    pub is_a_current: bool,
    pub game_state_bind_group_a: wgpu::BindGroup,
    pub game_state_bind_group_b: wgpu::BindGroup,
    pub render_bind_group_a: wgpu::BindGroup,
    pub render_bind_group_b: wgpu::BindGroup,
    pub compute_uniform_bind_group: wgpu::BindGroup,
    pub compute_pipeline: wgpu::ComputePipeline,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ComputeUniform {
    rows: u32,
    cols: u32,
    _pad: [u32; 2],
}

impl ComputeUniform {
    pub fn new(rows: u32, cols: u32) -> Self {
        Self {
            rows,
            cols,
            _pad: [0; 2],
        }
    }
}

impl GameData {
    pub fn new(device: &wgpu::Device, config: &AppConfig) -> Self {
        let mut rng = rng();

        let current_state = random_state(
            &mut rng,
            config.num_elements() as u32,
            config.init_rand_threshold,
        );
        let next_state = current_state.clone();

        let game_state_buffer_a = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Current State Buffer"),
            contents: bytemuck::cast_slice(&current_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let game_state_buffer_b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("next State Buffer"),
            contents: bytemuck::cast_slice(&next_state),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

        let compute_uniform = ComputeUniform::new(config.rows as u32, config.cols as u32);
        let compute_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&compute_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let compute_uniform_bind_group_layout =
            GameData::get_compute_uniform_bind_group_layout(device);
        let compute_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Uniform Bind Group"),
            layout: &compute_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: compute_uniform_buffer.as_entire_binding(),
            }],
        });

        let game_state_bind_group_layout = GameData::get_compute_bind_group_layout(device);
        let game_state_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Game state bind group A"),
            layout: &game_state_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: game_state_buffer_a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: game_state_buffer_b.as_entire_binding(),
                },
            ],
        });

        let game_state_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Game state bind group B"),
            layout: &game_state_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: game_state_buffer_b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: game_state_buffer_a.as_entire_binding(),
                },
            ],
        });

        let render_bind_group_layout = GameData::get_render_bind_group_layout(device);

        let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group A"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: game_state_buffer_a.as_entire_binding(),
            }],
        });

        let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group B"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: game_state_buffer_b.as_entire_binding(),
            }],
        });
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[
                    &compute_uniform_bind_group_layout,
                    &game_state_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/compute.wgsl").into()),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            cache: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        });

        Self {
            rng,
            game_state_buffer_a,
            is_a_current: true,
            game_state_buffer_b,
            render_bind_group_a,
            render_bind_group_b,
            game_state_bind_group_a,
            game_state_bind_group_b,
            compute_uniform_bind_group,
            compute_pipeline,
        }
    }

    pub fn get_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    // we need to be able to write to it to paint
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
    pub fn update_grid_state(&self, new_state: &[u32], queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.game_state_buffer_a,
            0,
            bytemuck::cast_slice(new_state),
        );
        queue.write_buffer(
            &self.game_state_buffer_b,
            0,
            bytemuck::cast_slice(new_state),
        );
    }
    pub fn reset_grid_state(&mut self, config: &AppConfig, queue: &wgpu::Queue) {
        let new_state = vec![0_u32; config.num_elements()];
        queue.write_buffer(
            &self.game_state_buffer_a,
            0,
            bytemuck::cast_slice(&new_state),
        );
        queue.write_buffer(
            &self.game_state_buffer_b,
            0,
            bytemuck::cast_slice(&new_state),
        );
    }
    pub fn randomise_grid_state(&mut self, config: &AppConfig, queue: &wgpu::Queue) {
        let new_state = random_state(
            &mut self.rng,
            config.num_elements() as u32,
            config.init_rand_threshold,
        );
        queue.write_buffer(
            &self.game_state_buffer_a,
            0,
            bytemuck::cast_slice(&new_state),
        );
        queue.write_buffer(
            &self.game_state_buffer_b,
            0,
            bytemuck::cast_slice(&new_state),
        );
    }

    pub fn get_compute_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Uniform Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
    pub fn get_compute_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Game State Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    pub fn get_current_compute_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_a_current {
            &self.game_state_bind_group_a
        } else {
            &self.game_state_bind_group_b
        }
    }
    pub fn get_current_render_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_a_current {
            &self.render_bind_group_a
        } else {
            &self.render_bind_group_b
        }
    }

    pub fn swap_current(&mut self) {
        self.is_a_current = !self.is_a_current;
    }
}

fn random_state(rng: &mut ThreadRng, num_elements: u32, init_rand_threshold: f64) -> Vec<u32> {
    (0..num_elements)
        .map(|_| rng.random_bool(init_rand_threshold) as u32)
        .collect()
}
