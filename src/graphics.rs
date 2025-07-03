use std::sync::Arc;
use rand::Rng;
use wgpu::RequestAdapterOptions;
use winit::window::Window;

use crate::{constants::{COLS,  ROWS}, render::Renderer};
// use crate::constant::INITIAL_STATE;

pub struct GraphicsContext {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pub window: Arc<Window>,
    is_surface_configured: bool,
    renderer: Renderer,
}

impl GraphicsContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // make an instance to get the backend eg wgpu, metal, vulkan etc
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        // make the surface we will be rendering to. we're taking it from winit
        let surface = instance.create_surface(window.clone())?;

        // get an adapter. the physical GPU / logical GPU
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;
        log::info!("Adapter device type {:?}", adapter.get_info().device_type);
        log::info!("Adapter device backend {:?}", adapter.get_info().backend);
        log::info!("Adapter device name {:?}", adapter.get_info().name);
        log::info!("Adapter device {:?}", adapter.get_info().device);
        log::info!("Adapter device vendor {:?}", adapter.get_info().vendor);

        // Now we get a device. which is the connection to the GPU
        // and we get the queue, which is where we send commands
        // to the GPU
        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        // gets the capabilities of the surface when used
        // with the given adapter
        let surface_caps = surface.get_capabilities(&adapter);

        // Now we make the surface format the fits within the capabilities'
        // constraints
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            // if none are sRGB, then take the first
            .unwrap_or(surface_caps.formats[0]);
        log::warn!("Surface format is sRGB? {:?}", surface_format.is_srgb());

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Convert initial state to u32 for consistency
        // let initial_state_u32: Vec<u32> = INITIAL_STATE
        //     .iter()
        //     .flat_map(|row| row.iter().map(|&b| b as u32))
        //     .collect();
        
        let mut rng = rand::rng();
        let initial_state_u32: Vec<u32> = (0..ROWS*COLS).map(|_| rng.random_bool(0.5) as u32).collect();

        let renderer = Renderer::new(&device, surface_format, &initial_state_u32);

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            is_surface_configured: false,
            window,
            renderer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }

    pub fn update_game_state(&mut self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Game State Update"),
            });

        self.renderer.run_compute(&mut encoder);
        self.queue.submit(std::iter::once(encoder.finish()));

        // Swap buffers after compute
        self.renderer.step_simulation();
    }

    pub fn render(&mut self) -> anyhow::Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();
        if !self.is_surface_configured {
            return Ok(());
        }
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.renderer.render(&mut encoder, &view);

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
