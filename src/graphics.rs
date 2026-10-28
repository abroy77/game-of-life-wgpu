//! The connection to the GPU, and the three things we ask it to do each frame.
//!
//! [`GraphicsContext::new`] walks through the standard wgpu setup recipe
//! (instance → surface → adapter → device + queue → surface config).
//!
//! Then, every frame, the app calls:
//! - [`GraphicsContext::paint`]: copy painted cells into the grid (compute pass),
//! - [`GraphicsContext::update`]: advance the simulation one step (compute pass),
//! - [`GraphicsContext::render`]: draw the grid to the window (render pass).
//!
//! The window helpers at the bottom create the winit window (or attach to the HTML `<canvas>`).

use crate::{
    config::AppConfig, game_data::GameData, paint::MousePainter, render_data::RenderData,
    vertex::INDICES,
};
use std::sync::Arc;
use wgpu::{CurrentSurfaceTexture, ExperimentalFeatures};
#[cfg(not(target_arch = "wasm32"))]
use winit::dpi::PhysicalSize;
#[cfg(target_arch = "wasm32")]
use {
    // JsCast lets us cast JS objects to more specific types (e.g. Element -> HtmlCanvasElement)
    wasm_bindgen::{JsCast, UnwrapThrowExt},
    // lets winit attach its window to an existing HTML <canvas>
    winit::{platform::web::WindowAttributesExtWebSys, window::WindowAttributes},
};

use winit::{event_loop::ActiveEventLoop, window::Window};

/// Everything needed to talk to the GPU and draw to the window.
///
/// The two most-used handles are:
/// - **Device**: our logical connection to the GPU. Used to *create* things
///   (buffers, shaders, pipelines, command encoders).
/// - **Queue**: where we *submit* work for the GPU to run, and upload data to buffers.
pub struct GraphicsContext {
    /// The drawable area of the window / canvas as the GPU sees it.
    surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// How the surface's textures are set up: pixel format, size, etc.
    pub surface_config: wgpu::SurfaceConfiguration,
    /// The surface can't be drawn to until it's configured with a non-zero size.
    /// On the web that only happens after the first resize, so we track it here.
    pub is_surface_configured: bool,
    /// `Arc` because both the surface and the app need to hold on to the window.
    pub window: Arc<Window>,
}

/// Data the render shader needs that is the same for every cell: the cell size.
///
/// Matches `struct RenderUniform` in `render.wgsl`. The memory layout must match exactly:
/// see [`ComputeUniform`](crate::game_data::ComputeUniform) for more about `Pod` and `#[repr(C, align(16))]`.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderUniform {
    pub cell_size: [f32; 2], // 8 bytes
    pub _pad: [f32; 2],      // 16 bytes
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
    /// Connect to the GPU and prepare the window for drawing.
    ///
    /// This is `async` because asking for an adapter and a device can take a while.
    /// On desktop we just block until it's done; on the web the browser requires us to await it.
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        // Choose which graphics APIs ("backends") wgpu may use.
        let backends = {
            #[cfg(target_arch = "wasm32")]
            {
                // Browser WebGPU only. WebGL can't run compute shaders, which we need.
                wgpu::Backends::BROWSER_WEBGPU
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // The main modern APIs: Vulkan, Metal (macOS), DirectX 12 (Windows).
                wgpu::Backends::PRIMARY
            }
        };
        // Instance: the entry point to wgpu. Used to find GPUs and create surfaces.
        // A display handle is only needed for the OpenGL backend, which we don't use.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });

        // Surface: the part of the window we can draw into.
        let surface = instance.create_surface(window.clone()).unwrap();

        // Adapter: a handle to one physical GPU. We ask for a fast one that can
        // draw to our surface.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance, // overkill but eh.
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                // Only useful for hiding GPU details from untrusted code (e.g. in a browser engine).
                apply_limit_buckets: false,
            })
            .await?;
        // Device + Queue: open a connection to the adapter. We need no special features
        // and the default limits, so this works on almost any GPU.
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: ExperimentalFeatures::disabled(),
            })
            .await?;

        // Ask what the surface supports on this GPU, and pick a pixel format.
        // We prefer an sRGB format, so the GPU converts our colours to what screens expect.
        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_capabilities.formats[0]);
        // Describe how the surface should be set up. It isn't applied yet: that happens in
        // `resize`, once we know the window has a real (non-zero) size.
        let window_size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            // We'll render into these textures.
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            // Present mode = how frames are synced to the screen.
            // We take the first mode this surface supports.
            present_mode: surface_capabilities.present_modes[0],
            // How many frames the CPU may queue up ahead of the screen.
            desired_maximum_frame_latency: 2, // default value
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
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
    /// Ask winit to send a `RedrawRequested` event, which calls [`GraphicsContext::render()`].
    pub fn request_redraw(&mut self) {
        self.window.request_redraw();
    }
    /// The window's size in physical pixels.
    pub fn get_window_size(&self) -> (u32, u32) {
        let size = self.window.inner_size();
        (size.width, size.height)
    }

    /// Apply the stored `surface_config` to the surface.
    fn apply_surface_config(&mut self) {
        self.surface.configure(&self.device, &self.surface_config);
        self.is_surface_configured = true;
        log::info!("Surface configured successfully");
    }

    /// Set the surface to a new size in physical pixels. A size of zero (e.g. a minimised
    /// window) is ignored, because a surface can't be configured with a zero size.
    pub fn resize(&mut self, width: u32, height: u32) {
        log::info!("Resize called: {width}x{height}");
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.apply_surface_config();
        }
    }

    /// Resize the surface to match the window's current size.
    fn resize_to_window(&mut self) {
        let (width, height) = self.get_window_size();
        self.resize(width, height);
    }
    /// Advance the simulation by one generation on the GPU.
    ///
    /// Runs `compute.wgsl` once per cell: read the current grid, write the next grid.
    /// Then swaps which buffer counts as "current" (see [`GameData`]).
    pub fn update(&mut self, game_data: &mut GameData, config: &AppConfig) {
        if !self.is_surface_configured {
            // early return
            return;
        }
        // A CommandEncoder records GPU commands into a list. Nothing runs yet.
        let mut encoder: wgpu::CommandEncoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Compute Encoder"),
                });

        // Record a compute pass. The extra scope ends the pass by dropping it.
        // The implementation of `Drop` on the ComputePass ends it and allows us to call
        // `encoder.finish()`.
        // begin compute pass scope
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            // Which shader to run, and which resources go in each `@group(N)`.
            compute_pass.set_pipeline(&game_data.compute_pipeline);
            compute_pass.set_bind_group(0, &game_data.compute_uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, game_data.get_current_compute_bind_group(), &[]);
            // Launch enough 16x16 workgroups to cover every cell.
            compute_pass.dispatch_workgroups(
                config.compute_dispatches[0] as u32,
                config.compute_dispatches[1] as u32,
                1,
            );
            // end compute pass scope. Drop runs on ComputePass, completing it.
        }
        // Finish recording and send the commands to the GPU to run.
        // `submit` takes an iterator of command buffers; `once` makes a one-item iterator.
        let command_buffer = encoder.finish();
        self.queue.submit(std::iter::once(command_buffer));

        // The "next" buffer we just wrote becomes "current" for the next step.
        game_data.swap_current();
    }

    /// Draw the current grid to the window.
    ///
    /// Gets the next texture from the surface, draws `rows * cols` squares into it with
    /// one instanced draw call, and presents it on screen.
    pub fn render(
        &mut self,
        render_data: &RenderData,
        game_state_render_bind_group: &wgpu::BindGroup,
        config: &AppConfig,
    ) {
        if !self.is_surface_configured {
            // don't render unless surface is configured
            return;
        }
        // Get the texture from the surface to draw this frame into.
        // A suboptimal texture is still drawn, then the surface is reconfigured after presenting.
        let (output, suboptimal) = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => (texture, false),
            CurrentSurfaceTexture::Suboptimal(texture) => (texture, true),
            // Window hidden, or the GPU was too slow: skip this frame and try again next time.
            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return,
            // The surface no longer matches the window (e.g. it was resized): set it up again.
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                self.resize_to_window();
                return;
            }
            CurrentSurfaceTexture::Validation => {
                log::error!("Validation error while acquiring surface texture");
                return;
            }
        };
        // A TextureView says how to read/write a texture. The default view (the whole
        // texture) is all we need to render into it.
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // A colour attachment is the texture a render pass draws into.
        // `load` says what happens to it at the start of the pass (clear it to the
        // background colour), `store` says to keep the result so it can be shown / drawn on top of
        // Think of it like preprocessing of your frame.
        // 1. clear the texture with this background color everywhere
        // 2. After the pass, store the result so that it can be displayed
        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &view,
            resolve_target: None,
            ops: wgpu::Operations {
                // LOAD: load the clear color onto each element of the view
                load: wgpu::LoadOp::Clear(config.background_color),
                // STORE: keep the data so it's seen on the screen
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        };
        // Record the render pass. The scope ends the pass before `encoder.finish()`.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(color_attachment)],
                // No 3D depth testing, GPU timing queries, or VR multiview needed.
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            // Bind the pipeline and everything its shaders read:
            // group 0 = cell size uniform, group 1 = the current grid state.
            render_pass.set_pipeline(&render_data.pipeline);
            render_pass.set_bind_group(0, &render_data.render_uniform_bind_group, &[]);
            render_pass.set_bind_group(1, game_state_render_bind_group, &[]);
            // Slot 0: the 4 corners of one square. Slot 1: one position per cell.
            render_pass.set_vertex_buffer(0, render_data.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, render_data.instance_buffer.slice(..));
            render_pass.set_index_buffer(
                render_data.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            // One draw call for the whole grid: draw the 6-index square once per cell
            // (instances 0..rows*cols). The fragment shader reads the grid state to decide
            // whether each cell is alive (white) or dead (transparent).
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..config.num_elements() as u32);
        }
        // Send the commands to the GPU, then hand the finished frame to the screen.
        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);
        // if the surface texture was suboptimal, run resize and configure the surface again.
        if suboptimal {
            self.resize_to_window();
        }
    }
    /// Push the cells painted since the last call into the grid on the GPU.
    ///
    /// Uploads the CPU paint mask, clears it ready for new strokes, then runs `paint.wgsl`,
    /// which sets every painted cell alive (`state |= paint`).
    ///
    /// `game_state_compute_bind_group` is [`GameData::get_current_compute_bind_group`]: the
    /// compute bindings, because the paint shader writes to the grid.
    pub fn paint(
        &mut self,
        mouse_painter: &mut MousePainter,
        compute_uniform_bind_group: &wgpu::BindGroup,
        game_state_compute_bind_group: &wgpu::BindGroup,
        config: &AppConfig,
    ) {
        if !self.is_surface_configured {
            // don't render unless surface is configured
            log::warn!("Surface not ready for painting");
            return;
        }
        // Upload the paint mask to the GPU, then clear the CPU copy for the next strokes.
        mouse_painter.write_to_buffer(&self.queue);
        mouse_painter.clear_buffer();

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Painter Encoder"),
            });

        // A compute pass like in `update`, with a third bind group for the paint mask.
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&mouse_painter.painter_pipeline);
            compute_pass.set_bind_group(0, compute_uniform_bind_group, &[]);
            compute_pass.set_bind_group(1, game_state_compute_bind_group, &[]);
            compute_pass.set_bind_group(2, &mouse_painter.painter_buffer_bind_group, &[]);
            compute_pass.dispatch_workgroups(
                config.compute_dispatches[0] as u32,
                config.compute_dispatches[1] as u32,
                1,
            );
        }
        // Submit. `queue.write_buffer()` above is guaranteed to finish before this runs.
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}

/// Create the app window:  attached to the page's `<canvas id="canvas">` on the web.
pub fn get_window(event_loop: &ActiveEventLoop, config: &AppConfig) -> Arc<Window> {
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
        window_attributes = window_attributes.with_inner_size(PhysicalSize::new(
            config.desktop_size[0],
            config.desktop_size[1],
        ));
    }

    // winit windows can only be created from inside the running event loop.
    // We use Arc to have multiple references to this window
    let window = event_loop.create_window(window_attributes).unwrap();
    Arc::new(window)
}

/// Web only: find the HTML `<canvas id="canvas">` that we draw into.
#[cfg(target_arch = "wasm32")]
pub fn get_html_canvas() -> web_sys::HtmlCanvasElement {
    // get the browser window
    let window = wgpu::web_sys::window().unwrap_throw();
    // get the DOM in the window
    let document = window.document().unwrap_throw();
    // Find the element by its ID. This gives a generic `Element`: Rust can't know that
    // it's a canvas, so we cast it below.
    let canvas = document.get_element_by_id("canvas").unwrap_throw();

    // Cast to HtmlCanvasElement specifically
    canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap_throw()
}

/// Web only: tell winit to use our existing canvas instead of creating a new one.
#[cfg(target_arch = "wasm32")]
fn setup_window_with_canvas(window_attributes: WindowAttributes) -> WindowAttributes {
    let canvas = get_html_canvas();
    window_attributes.with_canvas(Some(canvas))
}
