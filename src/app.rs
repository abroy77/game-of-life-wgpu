use crate::{
    config::{AppConfig, load_config},
    game_data::GameData,
    graphics::{self, GraphicsContext},
    mouse_handler::MousePainter,
    render_data::RenderData,
};

use log::info;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(not(target_arch = "wasm32"))]
use {
    std::{
        iter::repeat_n,
        time::{Duration, Instant},
    },
    winit::window::CustomCursor,
};

#[cfg(target_arch = "wasm32")]
use {
    std::sync::Mutex,
    wasm_bindgen::prelude::*,
    web_time::{Duration, Instant},
    winit::event_loop::EventLoopProxy,
};

pub enum AppEvents {
    NewGraphicsContext(GraphicsContext),
    PlayPause,
    UpdateFps(usize),
    RandomiseState,
    StepForward,
    UpdateRows(usize),
    UpdateCols(usize),
}

// use std::sync::Mutex;
// use winit::event_loop::EventLoopProxy;

// This thread local will allow us to send events from our JS functions to control
// the state of the app
#[cfg(target_arch = "wasm32")]
thread_local! {pub static EVENT_LOOP_PROXY: Mutex<Option<EventLoopProxy<AppEvents>>> = Mutex::new(None);}

pub struct App {
    // EventLoopProxy allows for Async code which is needed on the web so the
    //page does not hang when waiting on resources.
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<AppEvents>>,
    graphics_context: Option<GraphicsContext>,
    render_data: Option<RenderData>,
    game_data: Option<GameData>,
    next_frame: Instant,
    mouse: MousePainter,
    config: AppConfig,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<AppEvents>,
    ) -> anyhow::Result<Self> {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        let config = load_config();

        let next_frame = Instant::now() + config.frame_duration;
        let mouse = MousePainter::new();

        Ok(Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            graphics_context: None,
            render_data: None,
            game_data: None,
            next_frame,
            config,
            mouse,
        })
    }
    #[cfg(target_arch = "wasm32")]
    pub fn setup_graphics_context(&mut self, window: Arc<Window>) {
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
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_graphics_context(&mut self, window: Arc<Window>) {
        // On desktop we're using pollster which is a very simple async runnowner
        self.graphics_context = Some(pollster::block_on(GraphicsContext::new(window)).unwrap());
    }

    fn play_pause(&mut self) {
        self.config.is_paused = !self.config.is_paused;
    }
    fn update_fps(&mut self, new_fps: usize) {
        // set limits on fps to be within 0 and 60. clip the values at those limits
        self.config.frame_duration = Duration::from_millis((1000 / new_fps.clamp(1, 60)) as u64);
        self.next_frame = Instant::now() + self.config.frame_duration;
    }

    fn step_forward(&mut self) {
        // need to check if we're paused, and if so, run a single compute update
        // and render pass
        if !self.config.is_paused {
            return;
        }
        if let (Some(gc), Some(game_data)) = (&mut self.graphics_context, &mut self.game_data) {
            gc.update(game_data, &self.config);
            gc.request_redraw();
        }
    }

    fn randomise_state(&mut self) {
        if let (Some(game_data), Some(graphics_context), Some(render_data)) = (
            &mut self.game_data,
            &mut self.graphics_context,
            &mut self.render_data,
        ) {
            game_data.randomise_grid_state(&self.config, &graphics_context.queue);
            game_data.is_a_current = true;
            match graphics_context.render(
                render_data,
                game_data.get_current_render_bind_group(),
                &self.config,
            ) {
                Ok(_) => {
                    info!("shuffled rendered");
                }
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    let (width, height) = graphics_context.get_size().unwrap();
                    graphics_context.resize(width, height);
                }
                Err(e) => {
                    log::error!("Unable to render {e}");
                }
            }
        }
    }
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Space, true) => self.play_pause(),
            (KeyCode::ArrowRight, true) => self.step_forward(),
            (_, _) => (),
        }
    }

    fn setup_game_and_render_data(&mut self) {
        if let Some(graphics_context) = &self.graphics_context {
            let device = &graphics_context.device;

            self.game_data = Some(GameData::new(device, &self.config));
            // now that the graphics context is setup we can setup the render_pipeline if it's not there already
            // setup the render stuff now that the window and surface configurations are made

            let surface_config = &graphics_context.surface_config;
            self.render_data = Some(
                RenderData::new(
                    device,
                    surface_config,
                    &GameData::get_render_bind_group_layout(device),
                    &self.config,
                )
                .unwrap(),
            );
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn reset_cursor(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(graphics_context) = &self.graphics_context {
            let (window_width, window_height) = graphics_context.get_window_size();
            let scale_factor = graphics_context.window.scale_factor() as f32;
            let (window_width, window_height) = (
                window_width as f32 / scale_factor,
                window_height as f32 / scale_factor,
            );
            let cursor_width = ((window_width * self.config.cell_size) / 2.0) as u16;
            let cursor_height = ((window_height * self.config.cell_size) / 2.0) as u16;

            let rgba_buffer: Vec<u8> = repeat_n(
                self.config.cursor_color,
                (cursor_width * cursor_height) as usize,
            )
            .flatten()
            .collect();
            let custom_cursor_source = CustomCursor::from_rgba(
                rgba_buffer,
                cursor_width,
                cursor_height,
                cursor_width / 2,
                cursor_height / 2,
            )
            .unwrap();
            let custom_cursor = event_loop.create_custom_cursor(custom_cursor_source);
            graphics_context.window.set_cursor(custom_cursor);
        }
    }
}

impl ApplicationHandler<AppEvents> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        print!("resumed");
        let window = graphics::get_window(event_loop);
        // Now getting the window in wgpu is an asynchronous task because we're asking the GPU to get
        // it for us, then we will use it
        // This differs on web and desktop so we need two variants of this.
        self.setup_graphics_context(window);

        // we fill do these things after we have window size on web
        self.setup_game_and_render_data();

        self.next_frame = Instant::now() + self.config.frame_duration;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(self.next_frame));
    }
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvents) {
        // This is where we handle events in the proxy.
        // 'event' is state because of how we've parameterised the App.
        // This is where the proxy.send_event() ends up
        match event {
            AppEvents::NewGraphicsContext(mut graphics_context) => {
                #[cfg(target_arch = "wasm32")]
                {
                    // On web, manually trigger resize to configure the surface
                    let (width, height) = graphics_context.get_window_size();
                    graphics_context.resize(width, height);
                    graphics_context.request_redraw();
                }
                // no logic needed for the desktop
                // at this point our app is now setup once it's gotten the window and integrated it
                // in the web / desktop
                // in web after this I don't think we need the proxy because the async steps of getting the
                // window  / canvas are done. The app is ready!
                self.graphics_context = Some(graphics_context);
                self.setup_game_and_render_data();
            }
            AppEvents::PlayPause => self.play_pause(),
            AppEvents::UpdateFps(new_fps) => self.update_fps(new_fps),
            AppEvents::StepForward => self.step_forward(),
            AppEvents::RandomiseState => self.randomise_state(),
            _ => todo!(),
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
                self.mouse.configure(
                    (self.config.cols, self.config.rows),
                    (size.width as usize, size.height as usize),
                );
                #[cfg(not(target_arch = "wasm32"))]
                self.reset_cursor(event_loop);
            }
            WindowEvent::RedrawRequested => {
                match graphics_context.render(
                    self.render_data.as_ref().unwrap(),
                    self.game_data
                        .as_ref()
                        .unwrap()
                        .get_current_render_bind_group(),
                    &self.config,
                ) {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let (width, height) = graphics_context.get_size().unwrap();
                        graphics_context.resize(width, height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {e}");
                    }
                }
                // graphics_context.update(self.render_data.as_mut().unwrap());
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
            WindowEvent::CursorEntered { device_id: _ } => self.mouse.in_grid = true,
            WindowEvent::CursorLeft { device_id: _ } => self.mouse.in_grid = false,
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Released,
                button: MouseButton::Left,
            } => self.mouse.is_pressed = false,
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Pressed,
                button: MouseButton::Left,
            } => {
                // if we're in the grid, then we want to add stuff to the buffer. but the points we want to add are going to not just be
                self.mouse.is_pressed = true;
                if self.mouse.in_grid {
                    self.mouse.add_to_buffer();
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position: phys_pos,
            } => {
                // we only want to add positions to the buffer if in grid and pressed
                if self.mouse.is_pressed && self.mouse.in_grid {
                    self.mouse.pos = phys_pos;
                    self.mouse.add_to_buffer();
                }
            }

            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        // paint logic here?
        if now >= self.mouse.next_buffer_clear {
            // we put this data into a staging buffer and add to our current_state_buffer
        }
        if now >= self.next_frame && !self.config.is_paused {
            // if we're ready for next frame and not paused then we update and send redraw command
            if let (Some(gc), Some(game_data)) = (&mut self.graphics_context, &mut self.game_data) {
                gc.update(game_data, &self.config);
                gc.request_redraw();
            }
            self.next_frame = now + self.config.frame_duration;
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new()?;
        event_loop.run_app(&mut app)?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        let proxy = event_loop.create_proxy();
        EVENT_LOOP_PROXY.with(|p| *p.lock().unwrap() = Some(proxy));
        let app = App::new(&event_loop)?;
        // On web, run_app doesn't return normally, so we handle it differently
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();

    match run() {
        Ok(_) => {
            log::info!("Game of Life initialized successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Failed to initialize Game of Life: {}", e);
            Err(format!("Initialization error: {}", e).into())
        }
    }
}
