
use game_of_life::app::App;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use winit::event_loop::EventLoopBuilder;

    env_logger::init();
    let event_loop = EventLoopBuilder {
       
    }


}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() {
    console_error_panic_hook::set_once();
    App::run().await;
}
