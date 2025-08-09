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
