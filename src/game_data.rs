//! The Game of Life grid on the GPU, and the compute pipeline that advances it.
//!
//! The grid lives in two GPU buffers (A and B). Each step reads one and writes the other,
//! then they swap roles. This is called **ping-pong** (lmao) (or double) buffering: a cell's new value
//! depends on its neighbours' *old* values, so we can't overwrite the grid while still reading it.

use rand::{Rng, rng, rngs::ThreadRng};
use wgpu::util::DeviceExt;

use crate::config::AppConfig;

/// GPU resources for the grid state and the simulation step.
///
/// wgpu terms used here:
/// - **Buffer**: a block of GPU memory. Its *usage flags* say what it may be used for.
/// - **BindGroupLayout**: the "shape" of the resources a shader expects in one `@group(N)`:
///   which `@binding` slots exist, what type each is, and which shader stages can see them.
/// - **BindGroup**: the actual buffers plugged into those slots. You can have several bind
///   groups with the same layout and switch between them cheaply, which is how we swap A and B.
pub struct GameData {
    rng: ThreadRng,
    /// One copy of the grid: `rows * cols` `u32`s, row by row from the bottom.
    /// 0 = dead, n ≥ 1 = alive with age n - 1 (capped at `color_steps`, used to pick its colour).
    pub game_state_buffer_a: wgpu::Buffer,
    /// The other copy of the grid.
    pub game_state_buffer_b: wgpu::Buffer,
    /// `true` when A holds the current generation and B is where the next one gets written.
    /// if `false` then B holds the current generation and A is where the next one gets written.
    pub is_a_current: bool,
    /// Compute bindings when A is current: binding 0 = A (read), binding 1 = B (write).
    pub game_state_bind_group_a: wgpu::BindGroup,
    /// Compute bindings when B is current: binding 0 = B (read), binding 1 = A (write).
    pub game_state_bind_group_b: wgpu::BindGroup,
    /// Render bindings: lets the render shader read buffer A.
    pub render_bind_group_a: wgpu::BindGroup,
    /// Render bindings: lets the render shader read buffer B.
    pub render_bind_group_b: wgpu::BindGroup,
    /// Grid dimensions (rows, cols) for the compute and paint shaders, at `@group(0)`.
    pub compute_uniform_bind_group: wgpu::BindGroup,
    /// The compiled `compute.wgsl` plus its bind group layouts, ready to dispatch to the GPU.
    pub compute_pipeline: wgpu::ComputePipeline,
}

/// Grid dimensions and the age cap (`color_steps`) for the compute shaders.
/// Matches `struct GridInfo` in `compute.wgsl` and `paint.wgsl`.
///
/// Sent as a **uniform** buffer: small, read-only data that's the same for every GPU thread.
/// (Compare with **storage** buffers, used for the grid: large, indexable, and can be written.)
/// Same layout rules as `RenderUniform` in `graphics.rs`: `#[repr(C)]`, and padded to 16 bytes.
/// The padding is not required for most GPU backends but WGSL still needs uniforms to be 16 byte
/// padded so we've added it + the `_pad` to fill the size
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ComputeUniform {
    rows: u32,        // 4 bytes
    cols: u32,        // 8 bytes
    color_steps: u32, // 12 bytes
    _pad: u32,        // 16 bytes
}

impl ComputeUniform {
    pub fn new(rows: u32, cols: u32, color_steps: usize) -> Self {
        Self {
            rows,
            cols,
            color_steps: color_steps as u32,
            _pad: 0,
        }
    }
}

impl GameData {
    /// Create the grid buffers, all bind groups, and the compute pipeline.
    pub fn new(device: &wgpu::Device, config: &AppConfig) -> Self {
        let rng = rng();

        //  Two grid buffers, both starting all-dead.
        // `create_buffer_init` creates a GPU buffer and fills it with our data in one go.
        let current_state = vec![0_u32; config.num_elements()];
        let next_state = current_state.clone();

        // `bytemuck::cast_slice` views our `&[u32]` as raw `&[u8]` without copying.
        // Usage flags: STORAGE = shaders can read/write it, COPY_DST = we can upload to it
        // from the CPU (reset/shuffle), COPY_SRC = it can be copied from.
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

        // Uniform buffer and its bind group (`@group(0)` in the shader).
        let compute_uniform =
            ComputeUniform::new(config.rows as u32, config.cols as u32, config.color_steps);
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

        // Two compute bind groups for `@group(1)`, with the same layout but the buffers
        // swapped. Picking one or the other each step is the ping-pong.
        let game_state_bind_group_layout = GameData::get_compute_bind_group_layout(device);
        // A is at binding 0, B at binding 1
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

        // B is at binding 0, A at binding 1
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

        // Two render bind groups, so the render shader can read whichever buffer is current.
        let render_bind_group_layout = GameData::get_render_bind_group_layout(device);

        // Render when A is current
        let render_bind_group_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group A"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: game_state_buffer_a.as_entire_binding(),
            }],
        });

        // Render when B is current
        let render_bind_group_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group B"),
            layout: &render_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: game_state_buffer_b.as_entire_binding(),
            }],
        });

        // Pipeline layout: the list of bind group layouts the pipeline uses, in `@group`
        // order. Index 0 = uniforms, index 1 = grid buffers.
        // `immediate_size` is for tiny per-draw constants; we don't use them.
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&compute_uniform_bind_group_layout),
                    Some(&game_state_bind_group_layout),
                ],
                immediate_size: 0,
            });

        // Shader module: the WGSL source, compiled for this GPU.
        // `include_str!` puts the file's text into the binary at compile time.
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/compute.wgsl").into()),
        });

        // Compute pipeline: shader + entry point (`fn main`) + layout, bundled into
        // something we can dispatch.
        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            cache: None,
            module: &shader,
            entry_point: Some("main"), // reflected in `compute.wgsl`
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

    /// Layout for the render shader's grid input (`@group(1) @binding(0)` in `render.wgsl`).
    pub fn get_render_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE | wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    // Rendering only reads the grid, so read-only is enough.
                    // (Painting writes through the compute bind group instead.)
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
    }
    /// Overwrite both grid buffers with `new_state`, so it's correct whichever one is current.
    ///
    /// `queue.write_buffer` uploads CPU data into a GPU buffer. The copy happens before
    /// the next submitted GPU work runs.
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
    /// Kill every cell.
    pub fn reset_grid_state(&mut self, config: &AppConfig, queue: &wgpu::Queue) {
        let new_state = vec![0_u32; config.num_elements()];
        self.update_grid_state(&new_state, queue);
    }
    /// Fill the grid randomly. Each cell is alive with probability `init_rand_threshold`.
    pub fn randomise_grid_state(&mut self, config: &AppConfig, queue: &wgpu::Queue) {
        let new_state = random_state(
            &mut self.rng,
            config.num_elements() as u32,
            config.init_rand_threshold,
        );
        self.update_grid_state(&new_state, queue);
    }

    /// Layout for the grid size uniform (`@group(0) @binding(0)` in `compute.wgsl` and
    /// `paint.wgsl`). Only compute shaders can see it.
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
                // count is None because we're only binding 1 resource not multiple
                count: None,
            }],
        })
    }
    /// Layout for the two grid buffers (`@group(1)` in `compute.wgsl`):
    /// binding 0 = current state, binding 1 = next state. Both read-write storage buffers.
    ///
    /// `paint.wgsl` reuses this layout and only touches binding 0.
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

    /// The compute bind group whose binding 0 is the *current* grid.
    pub fn get_current_compute_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_a_current {
            &self.game_state_bind_group_a
        } else {
            &self.game_state_bind_group_b
        }
    }
    /// The render bind group for the *current* grid.
    pub fn get_current_render_bind_group(&self) -> &wgpu::BindGroup {
        if self.is_a_current {
            &self.render_bind_group_a
        } else {
            &self.render_bind_group_b
        }
    }

    /// Flip which buffer is current. Called after each simulation step.
    pub fn swap_current(&mut self) {
        self.is_a_current = !self.is_a_current;
    }
}

/// A random grid: each cell is 1 with probability `init_rand_threshold`, otherwise 0.
fn random_state(rng: &mut ThreadRng, num_elements: u32, init_rand_threshold: f64) -> Vec<u32> {
    (0..num_elements)
        .map(|_| rng.random_bool(init_rand_threshold) as u32)
        .collect()
}
