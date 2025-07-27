use crate::{config::CONFIG, render_data::RenderData, vertex::INDICES};
use std::sync::Arc;
use winit::{event_loop::ActiveEventLoop, window::Window};

// putting these all in a seperate struct because
// they are related and building them requires async functionality
// I want an AppEvent to be able to send these back when ready.
pub struct GraphicsContext {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    window: Arc<Window>,
}
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniform {
    pub cell_size: f32,
    pub _pad: [f32; 3],
    pub _pad_more: [f32; 4],
}

impl Default for RenderUniform {
    fn default() -> Self {
        Self {
            cell_size: CONFIG.cell_size,
            _pad: [0.0; 3],
            _pad_more: [0.0; 4],
        }
    }
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        // make a wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        //Shader code assumes sRGB surface textures.
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        let window_size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: surface_capabilities.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        Ok(GraphicsContext {
            surface,
            device,
            queue,
            surface_config,
            is_surface_configured: false,
            window,
        })
    }
    pub fn request_redraw(&mut self) {
        self.window.request_redraw();
    }
    pub fn get_size(&self) -> Option<(u32, u32)> {
        if self.is_surface_configured {
            let size = self.window.inner_size();
            Some((size.width, size.height))
        } else {
            None
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }
    pub fn update(&mut self, render_data: &mut RenderData) {
        // this is probably where our ping pong logic will begin! we want
        // to have a random initial state.
        render_data.game_state.update();
        self.queue.write_buffer(
            &render_data.game_state_buffer,
            0,
            bytemuck::cast_slice(render_data.game_state.get_state()),
        );
    }

    pub fn render(&mut self, render_data: &RenderData) -> Result<(), wgpu::SurfaceError> {
        // self.request_redraw();

        if !self.is_surface_configured {
            // don't render unless surface is configured
            Ok(())
        } else {
            // This is the texture on which we will draw
            let output = self.surface.get_current_texture()?;
            // we need a TextureView
            let view = output
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

            // We want to clear the screen to a single colour at the start of our render pass.
            // For this we need a RenderPassColorAttachment that writes to our output texture view
            // ie `view`.
            //
            let clear_color_attachment = wgpu::RenderPassColorAttachment {
                // the view we're writing to. our output texture ie the whole
                // app screen
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // operations we're performing on that view
                    // LOAD: load the clear color onto each element of the view
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
                    // STORE: keep the data so it's seen on the screen
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            };
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(clear_color_attachment)],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                render_pass.set_pipeline(&render_data.pipeline);
                render_pass.set_bind_group(0, &render_data.render_bind_group, &[]);
                render_pass.set_vertex_buffer(0, render_data.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, render_data.instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                // here is where we will randomly choose which instances to draw in different
                // draw calls.
                // let instance_indeces = render_data.get_random_instances();

                render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..CONFIG.num_elements as u32);
            } // using std::iter::once to make a simple iterable that yields
            // a single item. This means I don't need to make a vec or array.
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();

            Ok(())
        }
    }
}

pub fn get_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    #[allow(unused_mut)]
    let mut window_attributes = Window::default_attributes();

    #[cfg(target_arch = "wasm32")]
    {
        // here is where we will attach the window to the HTML canvas on the web
        window_attributes = setup_window_with_canvas(window_attributes);
    }
    // a winit window requires a an event loop to create it
    // we use Arc to have multiple references to this window
    Arc::new(event_loop.create_window(window_attributes).unwrap())
}

///This function is only for the web to revrieve the
/// HTML canvas on which we will draw
#[cfg(target_arch = "wasm32")]
fn setup_window_with_canvas(window_attributes: WindowAttributes) -> WindowAttributes {
    // JsCast brings trait to cast rust dtypes to js safely or unsafely
    use wasm_bindgen::{JsCast, UnwrapThrowExt};
    // allows winit to inceract with web elements
    use winit::platform::web::WindowAttributesExtWebSys;

    // get the browser window
    let window = wgpu::web_sys::window().unwrap_throw();
    // get the DOM in the window
    let document = window.document().unwrap_throw();
    // get the canvas element. We know our html has this ID for a HtmlCanvas type
    // however rust can't be sure. hence we need the unchecked_into()
    // to convert to a HtmlElement
    let canvas = document.get_element_by_id("canvas").unwrap_throw();
    // attach the canvas to the window attributes for window creation
    window_attributes.with_canvas(Some(canvas.unchecked_into()))
}
