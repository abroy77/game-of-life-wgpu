use crate::{
    config::AppConfig, game_data::GameData, paint::MousePainter, render_data::RenderData,
    vertex::INDICES,
};
// use log::info;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::PhysicalSize;
#[cfg(target_arch = "wasm32")]
use {
    // JsCast brings trait to cast rust dtypes to js safely or unsafely
    wasm_bindgen::{JsCast, UnwrapThrowExt},
    // allows winit to inceract with web elements
    winit::{platform::web::WindowAttributesExtWebSys, window::WindowAttributes},
};

use winit::{event_loop::ActiveEventLoop, window::Window};

pub struct GraphicsContext {
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub is_surface_configured: bool,
    pub window: Arc<Window>,
}
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniform {
    pub cell_size: [f32; 2],
    pub _pad: [f32; 2],
}

impl RenderUniform {
    pub fn new(cell_size: (f32, f32)) -> Self {
        Self {
            cell_size: [cell_size.0, cell_size.1],
            _pad: [0.0; 2],
        }
    }
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        //backend selection
        let backends = {
            #[cfg(target_arch = "wasm32")]
            {
                // can't use webgl because it does not support compute shaders
                wgpu::Backends::BROWSER_WEBGPU
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                wgpu::Backends::PRIMARY
            }
        };
        // make a wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
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

    pub fn get_window_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        println!("Resize called: {width}x{height}");
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
            println!("Surface configured successfully");
        }
    }
    pub fn update(&mut self, game_data: &mut GameData, config: &AppConfig) {
        if !self.is_surface_configured {
        } else {
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Encoder"),
                });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&game_data.compute_pipeline);
                compute_pass.set_bind_group(0, &game_data.compute_uniform_bind_group, &[]);
                compute_pass.set_bind_group(1, game_data.get_current_compute_bind_group(), &[]);
                compute_pass.dispatch_workgroups(
                    config.compute_dispatches[0] as u32,
                    config.compute_dispatches[1] as u32,
                    1,
                );
            } // using std::iter::once to make a simple iterable that yields
            // a single item. This means I don't need to make a vec or array.
            // let buffer_size =
            //     (config.num_elements() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
            // let staging_buffer_a = self.device.create_buffer(&wgpu::BufferDescriptor {
            //     label: Some("Staging Buffer A"),
            //     size: buffer_size,
            //     usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            //     mapped_at_creation: false,
            // });
            // encoder.copy_buffer_to_buffer(
            //     &game_data.game_state_buffer_a,
            //     0,
            //     &staging_buffer_a,
            //     0,
            //     buffer_size,
            // );
            // let staging_buffer_b = self.device.create_buffer(&wgpu::BufferDescriptor {
            //     label: Some("Staging Buffer B"),
            //     size: buffer_size,
            //     usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            //     mapped_at_creation: false,
            // });
            // encoder.copy_buffer_to_buffer(
            //     &game_data.game_state_buffer_b,
            //     0,
            //     &staging_buffer_b,
            //     0,
            //     buffer_size,
            // );
            self.queue.submit(std::iter::once(encoder.finish()));

            // let buffer_slice_a = staging_buffer_a.slice(..);
            // buffer_slice_a.map_async(wgpu::MapMode::Read, |result| {
            //     if result.is_ok() {
            //     }
            // });

            // _ = self.device.poll(wgpu::PollType::Wait);
            // let data = buffer_slice_a.get_mapped_range();
            // let cells: &[u32] = bytemuck::cast_slice(&data);
            // println!("Current Data A: {:?}", cells);
            // drop(data);
            // staging_buffer_a.unmap();

            // let buffer_slice_b = staging_buffer_b.slice(..);
            // buffer_slice_b.map_async(wgpu::MapMode::Read, |result| {
            //     if result.is_ok() {
            //     }
            // });

            // _ = self.device.poll(wgpu::PollType::Wait);
            // let data = buffer_slice_b.get_mapped_range();
            // let cells: &[u32] = bytemuck::cast_slice(&data);
            // println!("Current Data B: {:?}", cells);
            // drop(data);
            // staging_buffer_b.unmap();

            game_data.swap_current();
        }
    }

    pub fn render(
        &mut self,
        render_data: &RenderData,
        game_state_render_bind_group: &wgpu::BindGroup,
        config: &AppConfig,
    ) -> Result<(), wgpu::SurfaceError> {
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
                    load: wgpu::LoadOp::Clear(config.background_color),
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
                render_pass.set_bind_group(0, &render_data.render_uniform_bind_group, &[]);
                render_pass.set_bind_group(1, game_state_render_bind_group, &[]);
                render_pass.set_vertex_buffer(0, render_data.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, render_data.instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );

                // here is where we will choose which instances to draw in different
                // draw calls.
                // Our current state_buffer in the game_state_bind group will control which
                // cells are shown as alive.
                render_pass.draw_indexed(
                    0..INDICES.len() as u32,
                    0,
                    0..config.num_elements() as u32,
                );
            } // using std::iter::once to make a simple iterable that yields
            // a single item. This means I don't need to make a vec or array.
            self.queue.submit(std::iter::once(encoder.finish()));
            output.present();

            Ok(())
        }
    }
    pub fn paint(
        &mut self,
        mouse_painter: &mut MousePainter,
        compute_uniform_bind_group: &wgpu::BindGroup,
        game_state_render_bind_group: &wgpu::BindGroup,
        config: &AppConfig,
    ) -> Result<(), wgpu::SurfaceError> {
        if !self.is_surface_configured {
            // don't render unless surface is configured
            println!("Surface not ready for painting");
            Ok(())
        } else {
            //
            mouse_painter.write_to_buffer(&self.queue);
            mouse_painter.clear_buffer();

            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Painter Encoder"),
                });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Compute Pass"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&mouse_painter.painter_pipeline);
                compute_pass.set_bind_group(0, compute_uniform_bind_group, &[]);
                compute_pass.set_bind_group(1, game_state_render_bind_group, &[]);
                compute_pass.set_bind_group(2, &mouse_painter.painter_buffer_bind_group, &[]);
                compute_pass.dispatch_workgroups(
                    config.compute_dispatches[0] as u32,
                    config.compute_dispatches[1] as u32,
                    1,
                );
            } // using std::iter::once to make a simple iterable that yields
            // a single item. This means I don't need to make a vec or array.
            // let buffer_size =
            //     (config.num_elements() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
            // let staging_buffer_a = self.device.create_buffer(&wgpu::BufferDescriptor {
            //     label: Some("Staging Buffer A"),
            //     size: buffer_size,
            //     usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            //     mapped_at_creation: false,
            // });
            // encoder.copy_buffer_to_buffer(
            //     &game_data.game_state_buffer_a,
            //     0,
            //     &staging_buffer_a,
            //     0,
            //     buffer_size,
            // );
            // let staging_buffer_b = self.device.create_buffer(&wgpu::BufferDescriptor {
            //     label: Some("Staging Buffer B"),
            //     size: buffer_size,
            //     usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            //     mapped_at_creation: false,
            // });
            // encoder.copy_buffer_to_buffer(
            //     &game_data.game_state_buffer_b,
            //     0,
            //     &staging_buffer_b,
            //     0,
            //     buffer_size,
            // );
            self.queue.submit(std::iter::once(encoder.finish()));
            Ok(())
        }
    }
}

pub fn get_window(event_loop: &ActiveEventLoop) -> Arc<Window> {
    #[allow(unused_mut)]
    let mut window_attributes = Window::default_attributes();

    #[cfg(target_arch = "wasm32")]
    {
        // For WASM, set up with actual canvas dimensions
        window_attributes = setup_window_with_canvas(window_attributes);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // For native, use a reasonable default size
        window_attributes = window_attributes.with_inner_size(PhysicalSize::new(800, 600));
    }

    // a winit window requires a an event loop to create it
    // we use Arc to have multiple references to this window
    let window = event_loop.create_window(window_attributes).unwrap();
    Arc::new(window)
}

///This function is only for the web to revrieve the
/// HTML canvas on which we will draw

#[cfg(target_arch = "wasm32")]
pub fn get_html_canvas() -> web_sys::HtmlCanvasElement {
    // get the browser window
    let window = wgpu::web_sys::window().unwrap_throw();
    // get the DOM in the window
    let document = window.document().unwrap_throw();
    // get the canvas element. We know our html has this ID for a HtmlCanvas type
    // however rust can't be sure. hence we need the unchecked_into()
    // to convert to a HtmlElement
    let canvas = document.get_element_by_id("canvas").unwrap_throw();

    // Cast to HtmlCanvasElement specifically
    canvas.dyn_into().unwrap_throw()
}

#[cfg(target_arch = "wasm32")]
pub fn set_canvas_size(canvas: &mut web_sys::HtmlCanvasElement) {
    let width = canvas.client_width();
    let height = canvas.client_height();
    log::info!("Canvas client dimensions: {}x{}", width, height);

}

#[cfg(target_arch = "wasm32")]
fn setup_window_with_canvas(window_attributes: WindowAttributes) -> WindowAttributes {
    let mut canvas = get_html_canvas();
    // set up the dimensions correctly
    set_canvas_size(&mut canvas);

    // Get the actual canvas dimensions and convert to logical size
    // log::info!("setup window with canvas:\n {}x{}", physical_width, physical_height);

    // Get device pixel ratio to convert to logical size
    // let window = web_sys::window().unwrap();
    // let device_pixel_ratio = window.device_pixel_ratio();

    // let logical_width = physical_width as f64 / device_pixel_ratio;
    // let logical_height = physical_height as f64 / device_pixel_ratio;

    // log::info!("Canvas dimensions - Physical: {}x{}, Device pixel ratio: {}, Logical: {}x{}",
    //    physical_width, physical_height, device_pixel_ratio, logical_width, logical_height);
    // log::info!("Canvas dimensions setting to {}x{}", physical_width, physical_height);

    // attach the canvas to the window attributes for window creation using logical size
    window_attributes.with_canvas(Some(canvas))
    // .with_inner_size(winit::dpi::PhysicalSize::new(
    //     physical_width,
    //     physical_height,
    // ))
}
