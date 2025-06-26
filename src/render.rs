use crate::buffers::Buffers;
use crate::resources::{BindGroups, Layouts};
use crate::pipeline::Pipelines;
use crate::constants::{ROWS, COLS};

pub struct Renderer {
    pub buffers: Buffers,
    pub bind_groups: BindGroups,
    pub pipelines: Pipelines,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let buffers = Buffers::new(device);
        let layouts = Layouts::new(device);
        let bind_groups = BindGroups::new(device, &layouts, &buffers);
        let pipelines = Pipelines::new(device, surface_format, &layouts);

        Self {
            buffers,
            bind_groups,
            pipelines,
        }
    }

    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipelines.render);
        render_pass.set_bind_group(0, &self.bind_groups.uniform, &[]);
        render_pass.set_vertex_buffer(0, self.buffers.vertex.slice(..));
        render_pass.set_vertex_buffer(1, self.buffers.instance.slice(..));
        render_pass.set_index_buffer(self.buffers.index.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.buffers.num_indices, 0, 0..(ROWS * COLS) as u32);
    }
}
