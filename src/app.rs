//! The application: owns all the state and reacts to events from winit's event loop.
//!
//! winit (the windowing library) runs an **event loop**: it waits for things to happen
//! (window resized, mouse moved, etc) and calls our [`App`]'s methods
//! from the [`ApplicationHandler`] trait. The process is:
//!
//! 1. [`App::new`] – load config. No window or GPU yet.
//! 2. `resumed` – the platform is ready: create the window and connect to the GPU.
//! 3. `window_event` – input, resizes, and `RedrawRequested` (→ render).
//! 4. `about_to_wait` – the loop is about to sleep. We check our two timers here
//!    (paint and simulate) and decide when to wake up next.
//!
//! On the web, connecting to the GPU must be async and can't block, so it finishes later and
//! reports back through a custom event ([`AppEvent::NewGraphicsContext`]).

use crate::{
    config::{AppConfig, load_config},
    game_data::GameData,
    graphics::{self, GraphicsContext},
    paint::MousePainter,
    render_data::RenderData,
};

use std::cmp;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseButton, Touch, TouchPhase, WindowEvent},
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
    std::cell::RefCell,
    wasm_bindgen::prelude::*,
    web_sys,
    web_time::{Duration, Instant},
    winit::event_loop::EventLoopProxy,
};

/// Our own custom events, sent into the event loop with an `EventLoopProxy` and handled in
/// `user_event`. Used for things that don't come from the window like async GPU setup
/// finishing, and HTML button presses on the webapp.
pub enum AppEvent {
    /// Web only: async GPU setup finished. Carries the ready [`GraphicsContext`].
    NewGraphicsContext(GraphicsContext),
    PlayPause,
    /// New simulation speed in steps per second.
    UpdateFps(usize),
    RandomiseState,
    ResetState,
    StepForward,
}

// Web only: a global handle to the event loop, so the functions in `web_controls`
// (called from JavaScript) can send `AppEvent` into the app.
// `thread_local!` + `RefCell` is a simple way to have a mutable global in single-threaded wasm.
#[cfg(target_arch = "wasm32")]
thread_local! {pub static EVENT_LOOP_PROXY: RefCell<Option<EventLoopProxy<AppEvent>>> = const { RefCell::new(None) };}

/// All app state.
///
/// The GPU-related fields are `Option` because they can't exist until the window and GPU are
/// ready (in `resumed`, or later on the web). Until then they're `None`, and the event
/// handlers do nothing.
pub struct App {
    /// Web only: lets the async GPU setup send its result back to the event loop,
    /// so the page doesn't freeze while it waits.
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<AppEvent>>,
    graphics_context: Option<GraphicsContext>,
    render_data: Option<RenderData>,
    game_data: Option<GameData>,
    /// When the next simulation step is due.
    next_frame: Instant,
    /// When the next paint upload is due.
    next_paint_frame: Instant,
    mouse: Option<MousePainter>,
    config: AppConfig,
}

impl App {
    /// Load the config and set up the timers. The window and GPU are created later, in `resumed`.
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<AppEvent>,
    ) -> anyhow::Result<Self> {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        let config = load_config();

        let next_frame = Instant::now() + config.frame_duration;
        let next_paint_frame = Instant::now() + config.paint_frame_duration;

        Ok(Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            graphics_context: None,
            render_data: None,
            game_data: None,
            next_frame,
            next_paint_frame,
            config,
            mouse: None,
        })
    }
    /// Web: start connecting to the GPU in the background.
    ///
    /// Browsers don't allow blocking, so `spawn_local` runs the async setup as a JS promise.
    /// When it finishes, the result is sent back as [`AppEvent::NewGraphicsContext`] and
    /// handled in `user_event`.
    #[cfg(target_arch = "wasm32")]
    pub fn setup_graphics_context(&mut self, window: Arc<Window>) {
        if let Some(proxy) = self.proxy.take() {
            wasm_bindgen_futures::spawn_local(async move {
                assert!(
                    proxy
                        .send_event(
                            // The event type is our `AppEvent` enum, so we can send the
                            // whole GraphicsContext through the event loop.
                            AppEvent::NewGraphicsContext(
                                GraphicsContext::new(window)
                                    .await
                                    .expect("Could not set up graphics context")
                            )
                        )
                        .is_ok()
                )
            });
        }
    }
    /// Desktop: connect to the GPU right away.
    ///
    /// `pollster::block_on` is a tiny async runtime: it simply waits for the future to finish.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_graphics_context(&mut self, window: Arc<Window>) {
        self.graphics_context = Some(pollster::block_on(GraphicsContext::new(window)).unwrap());
    }

    /// Toggle the simulation between running and paused.
    fn play_pause(&mut self) {
        self.config.is_paused = !self.config.is_paused;
    }
    /// Change the simulation speed, clamped to 1..=60 steps per second.
    fn update_fps(&mut self, new_fps: usize) {
        self.config.frame_duration = Duration::from_millis((1000 / new_fps.clamp(1, 60)) as u64);
        self.next_frame = Instant::now() + self.config.frame_duration;
    }

    /// Web: make the HTML Play/Pause button match `is_paused` (its CSS class sets the colour).
    #[cfg(target_arch = "wasm32")]
    fn update_play_pause_ui(&self) {
        use wasm_bindgen::UnwrapThrowExt;

        let window = web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let button = document.get_element_by_id("playPause").unwrap_throw();

        if self.config.is_paused {
            // Simulation is paused - show red (paused state)
            button.set_attribute("class", "paused").unwrap_throw();
            button.set_text_content(Some("▶ Play"));
        } else {
            // Simulation is playing - show green (playing state)
            button.set_attribute("class", "playing").unwrap_throw();
            button.set_text_content(Some("⏸ Pause"));
        }
    }

    /// Desktop has no button, so there's nothing to update.
    #[cfg(not(target_arch = "wasm32"))]
    fn update_play_pause_ui(&self) {
        // No-op for desktop version
    }

    /// Advance exactly one generation. Only works while paused; when running,
    /// the timer is already stepping.
    fn step_forward(&mut self) {
        if !self.config.is_paused {
            return;
        }
        if let (Some(gc), Some(game_data)) = (&mut self.graphics_context, &mut self.game_data) {
            gc.update(game_data, &self.config);
            gc.request_redraw();
        }
    }
    /// Kill every cell and redraw straight away.
    ///
    /// Both buffers are overwritten, so we can safely reset `is_a_current` to A.
    fn reset_state(&mut self) {
        if let (Some(game_data), Some(graphics_context), Some(render_data)) = (
            &mut self.game_data,
            &mut self.graphics_context,
            &mut self.render_data,
        ) {
            game_data.reset_grid_state(&self.config, &graphics_context.queue);
            game_data.is_a_current = true;
            graphics_context.render(
                render_data,
                game_data.get_current_render_bind_group(),
                &self.config,
            );
        }
    }

    /// Fill the grid randomly and redraw straight away.
    fn randomise_state(&mut self) {
        if let (Some(game_data), Some(graphics_context), Some(render_data)) = (
            &mut self.game_data,
            &mut self.graphics_context,
            &mut self.render_data,
        ) {
            game_data.randomise_grid_state(&self.config, &graphics_context.queue);
            game_data.is_a_current = true;
            graphics_context.render(
                render_data,
                game_data.get_current_render_bind_group(),
                &self.config,
            );
        }
    }
    /// Keyboard shortcuts
    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            (KeyCode::Space, true) => self.play_pause(),
            (KeyCode::ArrowRight, true) => self.step_forward(),
            (KeyCode::KeyR, true) => self.reset_state(),
            (KeyCode::KeyS, true) => self.randomise_state(),
            (_, _) => (),
        }
    }

    /// Create the grid, the render pipeline, and the painter once the GPU is ready.
    /// Does nothing if the [`GraphicsContext`] doesn't exist yet.
    fn setup_game_and_render_data(&mut self) {
        if let Some(graphics_context) = &self.graphics_context {
            let device = &graphics_context.device;
            // Update the cell size and spacing configuration according to the window provided
            // by the graphics context.
            self.config
                .update_cell_configuration(&graphics_context.window);
            self.game_data = Some(GameData::new(device, &self.config));

            // Render pipeline. It needs the surface's pixel format and the grid's bind
            // group layout.
            self.render_data = Some(
                RenderData::new(
                    device,
                    &graphics_context.surface_config,
                    &GameData::get_render_bind_group_layout(device),
                    &self.config,
                )
                .unwrap(),
            );

            // Painter. It reuses the grid's layouts so it can write into the same buffers.
            self.mouse = Some(MousePainter::new(
                device,
                &GameData::get_compute_bind_group_layout(device),
                &GameData::get_compute_uniform_bind_group_layout(device),
                &self.config,
                graphics_context.window.clone(),
            ));
            log::info!("got the gc set up and all");

            // Initialize the play/pause button UI state
            self.update_play_pause_ui();
        }
    }
    /// Desktop: replace the mouse cursor with a solid square about the size of one cell,
    /// so you can see exactly what you'll paint. Called again on resize because cell size changes.
    #[cfg(not(target_arch = "wasm32"))]
    fn reset_cursor(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(graphics_context) = &self.graphics_context {
            let (window_width, window_height) = graphics_context.get_window_size();
            let scale_factor = graphics_context.window.scale_factor() as f32;
            let (window_width, window_height) = (
                window_width as f32 / scale_factor,
                window_height as f32 / scale_factor,
            );
            // cell_size is in clip space, where the window is 2.0 wide, so
            // pixels = window_pixels * cell_size / 2.
            let cursor_width = ((window_width * self.config.cell_size.0) / 2.0) as u16;
            let cursor_height = ((window_height * self.config.cell_size.1) / 2.0) as u16;

            // One RGBA colour repeated for every pixel; the hotspot (click point) is the centre.
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

/// The event loop calls these methods. `AppEvent` is our custom event type for `user_event`.
impl ApplicationHandler<AppEvent> for App {
    /// The platform is ready for a window (at startup; on Android also after returning to the app).
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        print!("resumed");
        // 1. Create the window.
        let window = graphics::get_window(event_loop, &self.config);
        // 2. Connect to the GPU. Desktop: finishes right here. Web: finishes later in `user_event`.
        self.setup_graphics_context(window);

        // 3. Build the grid and pipelines. On the web the GPU isn't ready yet, so this does
        //    nothing now and runs again once `NewGraphicsContext` arrives.
        self.setup_game_and_render_data();

        // 4. Sleep until the first simulation step is due.
        self.next_frame = Instant::now() + self.config.frame_duration;
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(self.next_frame));
    }
    /// Handles our custom [`AppEvent`]: everything sent with `proxy.send_event(...)` ends up here.
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        match event {
            AppEvent::NewGraphicsContext(mut graphics_context) => {
                #[cfg(target_arch = "wasm32")]
                {
                    // The canvas may already have its final size, so no `Resized` event will
                    // come. Resize manually to configure the surface.
                    let (width, height) = graphics_context.get_window_size();
                    graphics_context.resize(width, height);
                    graphics_context.request_redraw();
                }
                // The GPU is ready: store it and build everything that depends on it.
                // (This event is only ever sent on the web; desktop sets up in `resumed`.)
                self.graphics_context = Some(graphics_context);
                self.setup_game_and_render_data();
            }
            AppEvent::PlayPause => {
                self.play_pause();
                self.update_play_pause_ui();
            }
            AppEvent::UpdateFps(new_fps) => self.update_fps(new_fps),
            AppEvent::StepForward => {
                // Stepping pauses the simulation first if it's running.
                if !self.config.is_paused {
                    self.play_pause();
                    self.update_play_pause_ui();
                }
                self.step_forward()
            }
            AppEvent::RandomiseState => self.randomise_state(),
            AppEvent::ResetState => self.reset_state(),
        }
    }
    /// Handles events from the window: close, resize, redraw, keyboard, mouse and touch.
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // Ignore window events until the GPU and painter are set up.
        let (graphics_context, mouse) = match (&mut self.graphics_context, &mut self.mouse) {
            (Some(gc), Some(mouse)) => (gc, mouse),
            (_, _) => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            // The window changed size: resize the surface and update the pixel → cell mapping.
            // The grid itself keeps its rows/cols; the cells just stretch.
            WindowEvent::Resized(size) => {
                graphics_context.resize(size.width, size.height);
                mouse.configure(&graphics_context.window, &self.config);
                #[cfg(not(target_arch = "wasm32"))]
                self.reset_cursor(event_loop);
            }
            // Moved to a screen with a different DPI: logical pixel sizes changed.
            WindowEvent::ScaleFactorChanged {
                scale_factor: _,
                inner_size_writer: _,
            } => {
                mouse.configure(&graphics_context.window, &self.config);
            }
            // Sent after `request_redraw()`. All drawing happens here.
            WindowEvent::RedrawRequested => {
                graphics_context.render(
                    self.render_data.as_ref().unwrap(),
                    self.game_data
                        .as_ref()
                        .unwrap()
                        .get_current_render_bind_group(),
                    &self.config,
                );
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
            // --- Mouse painting ---
            // Track whether the cursor is over the window and the left button is held.
            // While both are true, each cursor move marks the cell under it in the paint mask.
            WindowEvent::CursorEntered { device_id: _ } => {
                mouse.in_grid = true;
            }
            WindowEvent::CursorLeft { device_id: _ } => {
                mouse.in_grid = false;
            }
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Released,
                button: MouseButton::Left,
            } => {
                mouse.is_pressed = false;
            }
            WindowEvent::MouseInput {
                device_id: _,
                state: ElementState::Pressed,
                button: MouseButton::Left,
            } => {
                mouse.is_pressed = true;
                mouse.add_to_buffer(&self.config);
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position: phys_pos,
            } => {
                // Always track the position (in logical pixels), but only paint while pressed.
                if mouse.in_grid {
                    let logical_pos = phys_pos.to_logical(graphics_context.window.scale_factor());

                    mouse.pos = logical_pos;
                    if mouse.is_pressed {
                        mouse.add_to_buffer(&self.config);
                    }
                }
            }
            // --- Touch painting ---
            // Same idea as the mouse, but we follow one finger (by its id) from Started to Ended,
            // so other fingers touching the screen are ignored.
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Started,
                location,
                id: finger_id,
                ..
            }) => {
                if !mouse.is_pressed {
                    // if we're not already pressing somewhere else, then we register this finger
                    mouse.finger_id = Some(finger_id);
                    mouse.pos = location.to_logical(graphics_context.window.scale_factor());
                    mouse.is_pressed = true;
                    mouse.in_grid = true;
                }
            }
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Ended,
                id: finger_id,
                ..
            }) => {
                // if we get a touch phase ended event for the registered id, we say we're not pressing anymore
                if let Some(mouse_finger_id) = mouse.finger_id
                    && mouse_finger_id == finger_id
                {
                    mouse.finger_id = None;
                    mouse.is_pressed = false;
                    mouse.in_grid = false;
                }
            }
            WindowEvent::Touch(Touch {
                phase: TouchPhase::Moved,
                id: finger_id,
                location,
                ..
            }) => {
                if let Some(mouse_finger_id) = mouse.finger_id
                    && mouse_finger_id == finger_id
                    && mouse.is_pressed
                    && mouse.in_grid
                {
                    mouse.pos = location.to_logical(graphics_context.window.scale_factor());
                    mouse.add_to_buffer(&self.config);
                }
            }

            _ => {}
        }
    }
    /// Called when the event loop has handled all pending events and is about to sleep.
    ///
    /// This is our "game loop". Two independent timers:
    /// - **paint** (`paint_fps`): upload painted cells to the GPU. Runs even when paused.
    /// - **simulate** (`fps`): advance one generation. Only runs when not paused.
    ///
    /// Each one requests a redraw so the change shows up. Then we tell winit to sleep until
    /// whichever timer is due next (`ControlFlow::WaitUntil`), instead of spinning the CPU.
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        // Paint timer pushes painted cells into the current grid.
        if now >= self.next_paint_frame {
            if let (Some(gc), Some(mouse), Some(game_data)) = (
                &mut self.graphics_context,
                &mut self.mouse,
                &mut self.game_data,
            ) {
                gc.paint(
                    mouse,
                    &game_data.compute_uniform_bind_group,
                    game_data.get_current_compute_bind_group(),
                    &self.config,
                );
                gc.request_redraw();
            }
            self.next_paint_frame = now + self.config.paint_frame_duration;
        }
        // Simulation timer computes the next generation, then redraw.
        if now >= self.next_frame && !self.config.is_paused {
            if let (Some(gc), Some(game_data)) = (&mut self.graphics_context, &mut self.game_data) {
                gc.update(game_data, &self.config);
                gc.request_redraw();
            }
            self.next_frame = now + self.config.frame_duration;
        }

        // Sleep until the sooner of the two timers.
        let next_deadline = cmp::min(self.next_frame, self.next_paint_frame);
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(next_deadline));
    }
}

/// Start the app: set up logging, create the event loop, and hand it our [`App`].
pub fn run() -> anyhow::Result<()> {
    // Logging to the terminal on desktop, to the browser console on the web.
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    // An event loop that can also carry our custom `AppEvent` (see `user_event`).
    let event_loop = EventLoop::<AppEvent>::with_user_event().build()?;

    // Run it. On desktop `run_app` blocks until the window closes.
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new()?;
        event_loop.run_app(&mut app)?;
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Store a proxy globally so JS button handlers (`web_controls`) can send events.
        let proxy = event_loop.create_proxy();
        EVENT_LOOP_PROXY.with_borrow_mut(|p| *p = Some(proxy));
        let app = App::new(&event_loop)?;
        // The browser owns the real loop, so we can't block. `spawn_app` hands the app
        // to the browser's event loop and returns immediately.
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }

    Ok(())
}

/// Web entry point. `#[wasm_bindgen(start)]` makes this run automatically when `index.js`
/// calls `init()` to load the wasm module.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    // Show Rust panic messages in the browser console instead of a vague "unreachable" error.
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
