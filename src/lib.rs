pub mod constants;
pub mod pipeline;
pub mod vertex;
pub mod uniforms;
pub mod buffers;
pub mod resources;
pub mod render;

use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::UnwrapThrowExt;
use wgpu::RequestAdapterOptions;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};
use render::Renderer;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    renderer: Renderer,
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());

        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";
            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            // If we are not on the web we can use pollster to await
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously
            // and use the proxy to send
            // results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(State::new(window).await.expect("Unable to create canvas!"))
                            .is_ok()
                    )
                });
            }
        }
    }
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // This is where proxy.send_event() ends up
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event)
    }
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                _ = state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}
impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
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

        // Create the renderer - this now manages buffers, bind groups and pipelines
        let renderer = Renderer::new(&device, surface_format);

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
    
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            self.is_surface_configured = true;
        }
    }
    
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }
    
    fn render(&mut self) -> anyhow::Result<(), wgpu::SurfaceError> {
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
            
        // Use the renderer to render the scene
        self.renderer.render(&mut encoder, &view);
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;

    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );
    event_loop.run_app(&mut app)?;
    Ok(())
}
