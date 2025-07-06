use std::sync::Arc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    application::ApplicationHandler,
    event::KeyEvent,
    event::WindowEvent,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

#[cfg(target_arch = "wasm32")]
use winit::event_loop::EventLoopProxy;

pub fn say_hi() {
    println!("hello");
}

// pub struct State {
//     window: Arc<Window>,
// }

// impl State {
//     pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
//         Ok(State { window })
//     }

//     pub fn request_redraw(&mut self) {
//         self.window.request_redraw();
//     }

//     pub fn resize(&mut self, width: u32, height: u32) {
//         todo!()
//     }
//     pub fn render(&mut self) {
//         todo!()
//     }
// }
//

pub enum AppEvents {
    NewWindow(Arc<Window>),
}

pub struct App {
    // EventLoopProxy allows for Async code which is needed on the web so the
    //page does not hang when waiting on resources.
    #[cfg(target_arch = "wasm32")]
    proxy: Option<EventLoopProxy<AppEvents>>,
    window: Option<Arc<Window>>,
}

impl App {
    pub fn new(
        #[cfg(target_arch = "wasm32")] event_loop: &EventLoop<AppEvents>,
    ) -> anyhow::Result<Self> {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Ok(Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            window: None,
        })
    }
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

impl ApplicationHandler<AppEvents> for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            // here is where we will attach the window to the HTML canvas on the web
            window_attributes = setup_window_with_canvas(window_attributes);
        }
        // a winit window requires a an event loop to create it
        // we use Arc to have multiple references to this window
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // Now getting the window in wgpu is an asynchronous task because we're asking the GPU to get
        // it for us, then we will use it
        // This differs on web and desktop so we need two variants of this.
        #[cfg(not(target_arch = "wasm32"))]
        {
            // On desktop we're using pollster which is a very simple async runner
            self.window = Some(window);
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
                                AppEvents::NewWindow(window)
                            )
                            .is_ok()
                    )
                });
            }
        }
    }
    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, event: AppEvents) {
        // This is where we handle events in the proxy.
        // 'event' is state because of how we've parameterised the App.
        // This is where the proxy.send_event() ends up
        match event {
            AppEvents::NewWindow(window) => {
                #[cfg(target_arch = "wasm32")]
                {
                    window.request_redraw();
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
                self.window = Some(window);
            }
        }
    }
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // let state = match &mut self.state {
        //     Some(state) => state,
        //     None => return,
        // };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {

                // state.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
                // state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => match (code, state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
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
