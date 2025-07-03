use std::time::{Duration, Instant};
use std::sync::Arc;
#[cfg(target_arch="wasm32")]
use web_sys::HtmlCanvasElement;
use winit::application::ApplicationHandler;
use winit::keyboard::PhysicalKey;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use winit::{
    event_loop::EventLoop, keyboard::KeyCode,
    window::Window
};

use crate::graphics::GraphicsContext;


pub struct Simulation {
    last_update: Instant,
    update_interval: Duration,
    paused: bool,
}

impl Simulation {
    fn new(millis: u64) -> Self {
        Self {
            last_update: Instant::now(),
            update_interval: Duration::from_millis(millis),
            paused: false,
        }
    }

    fn check_for_update(&mut self) -> bool {
        if self.paused {
            return false;
        }

        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.update_interval {
            self.last_update = now;
            true
        } else {
            false
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn set_speed(&mut self, update_interval: u64) {
        self.update_interval = Duration::from_millis(update_interval);
    }
}

pub struct InputHandler;

impl InputHandler {
    fn new() -> Self {
        Self
    }

    fn handle_key(&mut self, code: KeyCode, is_pressed: bool, control_flow: &mut winit::event_loop::ControlFlow) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if code == KeyCode::Escape && is_pressed {
                // winit 0.30+ does not have Exit, so just set to Wait (or Poll) and rely on CloseRequested for exit
                *control_flow = winit::event_loop::ControlFlow::Wait;
            }
        }
        // On wasm, do nothing for Escape
        // Add more key handling as needed
    }
}

// use wasm_bindgen::JsCast;
// use winit::{platform::web::WindowAttributesExtWebSys, window::WindowAttributes};
// let document = web_sys::window().unwrap().document().unwrap();
// let canvas = document.get_element_by_id("canvas").unwrap();
// let window_attributes = WindowAttributes::default()
//     .with_canvas(Some(canvas.dyn_into().unwrap()))
//     .with_title("Game of Life Boyyyyyys");
//     // .with_canvas(Some(canvas.dyn_into().unwrap()))
// let window = event_loop.
// let window = Arc::new(window);
// let mut graphics = Some(GraphicsContext::new(window.clone()).await.unwrap());
// let mut simulation = Simulation::new(100);
// let mut input_handler = InputHandler::new();


// Variables are now managed by AppHandler below.
#[cfg(target_arch="wasm32")]
fn get_canvas() -> HtmlCanvasElement{
    use wasm_bindgen::JsCast;
    use web_sys::HtmlCanvasElement;
    use winit::platform::web as winit_web;
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas_element = document.get_element_by_id("canvas").unwrap();
    document.dyn_into().unwrap()
}

pub struct State {
    graphics: Option<GraphicsContext>,
    simulation: Simulation,
    input_handler: InputHandler,
    window: Arc<Window>    
}

pub struct App {
    #[cfg(target_arch="wasm32")]
    pub proxy: Option<EventLoopProxy<State>>,
    pub state: Option<State>
}

impl winit::application::ApplicationHandler<()> for App {

    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.graphics.is_none() {
            #[cfg(target_arch = "wasm32")] {{
                let canvas = get_canvas();

            }}
           let window_attributes = winit::window::WindowAttributes::default()
                .with_inner_size(winit::dpi::LogicalSize::new(1000.0, 1000.0));
            let window = std::sync::Arc::new(_event_loop.create_window(window_attributes).unwrap());
            self.graphics = Some(pollster::block_on(GraphicsContext::new(window)).unwrap());
        }
    }
    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        use winit::event::{WindowEvent, KeyEvent};
        match event {
            WindowEvent::CloseRequested => {
                // Set control flow to Wait (exit)
            },
            WindowEvent::Resized(size) => {
                if let Some(graphics) = &mut self.graphics {
                    graphics.resize(size.width, size.height);
                }
            },
            WindowEvent::RedrawRequested => {
                if let Some(graphics) = &mut self.graphics {
                    if self.simulation.check_for_update() {
                        graphics.update_game_state();
                    }
                    let _ = graphics.render();
                }
            },
            WindowEvent::KeyboardInput { event: KeyEvent { physical_key: winit::keyboard::PhysicalKey::Code(code), state: key_state, .. }, .. } => {
                self.input_handler.handle_key(code, key_state.is_pressed(), &mut winit::event_loop::ControlFlow::Poll);
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(graphics) = &mut self.graphics {
            graphics.window.request_redraw();
        }
    }
}


