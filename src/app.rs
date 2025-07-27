#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use crate::{
    config::CONFIG,
    graphics::{self, GraphicsContext},
    render_data::RenderData,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
};

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;

pub enum AppEvents {
    NewGraphicsContext(GraphicsContext),
}

pub struct App {
    // EventLoopProxy allows for Async code which is needed on the web so the
    //page does not hang when waiting on resources.
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<AppEvents>>,
    graphics_context: Option<GraphicsContext>,
    render_data: Option<RenderData>,
    next_frame: Instant,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<AppEvents>,
    ) -> anyhow::Result<Self> {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        let next_frame = Instant::now() + CONFIG.frame_duration;
        Ok(Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            graphics_context: None,
            render_data: None,
            next_frame,
        })
    }
}

impl ApplicationHandler<AppEvents> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = graphics::get_window(event_loop);
        // Now getting the window in wgpu is an asynchronous task because we're asking the GPU to get
        // it for us, then we will use it
        // This differs on web and desktop so we need two variants of this.
        #[cfg(not(target_arch = "wasm32"))]
        {
            // On desktop we're using pollster which is a very simple async runnowner
            self.graphics_context = Some(pollster::block_on(GraphicsContext::new(window)).unwrap());
        }
        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the proxy
            // to send results to the main event loop

            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                // so we can send event because we parameterised our proxy in the 'App'
                                // to be able to send 'State' as an event!
                                AppEvents::NewGraphicsContext(
                                    GraphicsContext::new(window)
                                        .await
                                        .expect(" Could not set up graphics context")
                                )
                            )
                            .is_ok()
                    )
                });
            }
        }
        // now that the graphics context is setup we can setup the render_pipeline if it's not there already
        if self.render_data.is_none() {
            // setup the render stuff now that the window and surface configurations are made
            self.render_data = Some(
                RenderData::new(
                    &self.graphics_context.as_ref().unwrap().device,
                    &self.graphics_context.as_ref().unwrap().surface_config,
                )
                .unwrap(),
            );
        }

        self.next_frame = Instant::now() + CONFIG.frame_duration;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(self.next_frame));
    }
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvents) {
        // This is where we handle events in the proxy.
        // 'event' is state because of how we've parameterised the App.
        // This is where the proxy.send_event() ends up
        match event {
            AppEvents::NewGraphicsContext(mut graphics_context) => {
                #[cfg(target_arch = "wasm32")]
                {
                    graphics_context.request_redraw();
                    // state.resize(
                    //     state.window.inner_size().width,
                    //     state.window.inner_size().height,
                    // );
                }
                // no logic needed for the desktop
                // at this point our app is now setup once it's gotten the window and integrated it
                // in the web / desktop
                // in web after this I don't think we need the proxy because the async steps of getting the
                // window  / canvas are done. The app is ready!
                self.graphics_context = Some(graphics_context);
            }
        }
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let graphics_context = match &mut self.graphics_context {
            Some(gc) => gc,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                graphics_context.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                // state.render();
                graphics_context.update(self.render_data.as_mut().unwrap());
                match graphics_context.render(self.render_data.as_ref().unwrap()) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let (width, height) = graphics_context.get_size().unwrap();
                        graphics_context.resize(width, height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {e}");
                    }
                }
                self.next_frame = Instant::now() + CONFIG.frame_duration;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => self.handle_key(event_loop, code, state.is_pressed()),
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame {
            if let Some(gc) = &mut self.graphics_context {
                gc.request_redraw();
            }
            self.next_frame += CONFIG.frame_duration;
        }
    }
}

impl App {
    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if let (KeyCode::Escape, true) = (code, is_pressed) {
            event_loop.exit()
        }
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

    // re-emphasising that the 'event' is our state. we're calling a change to our state the event in the loop
    let event_loop = EventLoop::<AppEvents>::with_user_event().build()?;

    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    )?;

    event_loop.run_app(&mut app)?;

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    run().unwrap_throw();
    Ok(())
}
