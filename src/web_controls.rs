// here we will define the controls needed for the
// EventLoop proxy.
// these will be exported using wasm-bindgen

use crate::app::{AppEvents, EVENT_LOOP_PROXY};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "playPause")]
pub fn play_pause() {
    EVENT_LOOP_PROXY.with(|proxy| {
        if let Ok(guard) = proxy.lock() {
            if let Some(proxy) = &*guard {
                let _ = proxy.send_event(AppEvents::PlayPause);
            }
        }
    })
}

#[wasm_bindgen(js_name = "stepForward")]
pub fn step_forward() {
    EVENT_LOOP_PROXY.with(|proxy| {
        if let Ok(guard) = proxy.lock() {
            if let Some(proxy) = &*guard {
                let _ = proxy.send_event(AppEvents::StepForward);
            }
        }
    })
}

#[wasm_bindgen(js_name = "randomiseState")]
pub fn randomise_state() {
    EVENT_LOOP_PROXY.with(|proxy| {
        if let Ok(guard) = proxy.lock() {
            if let Some(proxy) = &*guard {
                let _ = proxy.send_event(AppEvents::RandomiseState);
            }
        }
    })
}

#[wasm_bindgen(js_name = "updateFps")]
pub fn update_fps(new_fps: usize) {
    EVENT_LOOP_PROXY.with(|proxy| {
        if let Ok(guard) = proxy.lock() {
            if let Some(proxy) = &*guard {
                let _ = proxy.send_event(AppEvents::UpdateFps(new_fps));

                let window = web_sys::window().unwrap_throw();
                // get the DOM in the window
                let document = window.document().unwrap_throw();
                // get the canvas element. We know our html has this ID for a HtmlCanvas type
                // however rust can't be sure. hence we need the unchecked_into()
                // to convert to a HtmlElement
                let fps_span = document.get_element_by_id("fpsValue").unwrap_throw();

                fps_span.set_text_content(Some(&new_fps.to_string()));
            }
        }
    })
}

#[wasm_bindgen(js_name = "resetState")]
pub fn reset_state(new_fps: usize) {
    EVENT_LOOP_PROXY.with(|proxy| {
        if let Ok(guard) = proxy.lock() {
            if let Some(proxy) = &*guard {
                let _ = proxy.send_event(AppEvents::ResetState);
            }
        }
    })
}
