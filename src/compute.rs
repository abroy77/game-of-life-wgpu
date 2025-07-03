use crate::constants::{COLS, ROWS};

const WORKGROUP_SIZE: u32 = 16;

pub struct ComputePass;

impl ComputePass {
    pub fn run(
        encoder: &mut wgpu::CommandEncoder,
        compute_pipeline: &wgpu::ComputePipeline,
        compute_uniform_bind_group: &wgpu::BindGroup,
        compute_state_bind_group: &wgpu::BindGroup,
    ) {
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Game of Life Compute"),
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(compute_pipeline);
        compute_pass.set_bind_group(0, compute_uniform_bind_group, &[]);
        compute_pass.set_bind_group(1, compute_state_bind_group, &[]);

        let dispatch_x = (COLS as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        let dispatch_y = (ROWS as u32 + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;
        compute_pass.dispatch_workgroups(dispatch_x, dispatch_y, 1);
    }
}
