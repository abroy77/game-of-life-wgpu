use crate::{compute::ComputePass, constants::{COLS, ROWS, BACKGROUND_COLOR}, gpu_resources::GpuResources};


pub struct Renderer {
    gpu_resources: GpuResources,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, initial_state: &[u32]) -> Self {
        let gpu_resources = GpuResources::new(device, surface_format, initial_state);
        Self { gpu_resources }
    }

    pub fn run_compute(&self, encoder: &mut wgpu::CommandEncoder) {
        ComputePass::run(
            encoder,
            &self.gpu_resources.compute_pipeline,
            &self.gpu_resources.compute_uniform_bind_group,
            &self.gpu_resources.compute_state_bind_group,
        );
    }

    pub fn render(&self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Game of Life Render"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.gpu_resources.render_pipeline);
        render_pass.set_bind_group(0, &self.gpu_resources.render_bind_group, &[]);
        render_pass.set_bind_group(1, &self.gpu_resources.render_game_state_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.gpu_resources.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.gpu_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.gpu_resources.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.gpu_resources.num_indices, 0, 0..(ROWS * COLS) as u32);
    }

    pub fn step_simulation(&mut self) {
        self.gpu_resources.swap_buffers();
    }
}
