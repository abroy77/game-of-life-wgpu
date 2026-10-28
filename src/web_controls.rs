//! Web only: Rust functions that the HTML buttons call.
//!
//! The path from a button click to the app:
//! 1. `index.html` button `onclick` → `window.playPause()` (set up in `index.js`).
//! 2. `#[wasm_bindgen(js_name = "...")]` exports these Rust functions to JavaScript under that name.
//! 3. Each one sends an `AppEvent` through the global `EVENT_LOOP_PROXY`.
//! 4. winit delivers it to `App::user_event`, which changes the app state.
//!
//! We send events instead of touching `App` directly because the event loop owns the `App`.

use crate::app::{AppEvent, EVENT_LOOP_PROXY};
use wasm_bindgen::prelude::*;

/// Toggle play/pause, then update the button's label to match.
#[wasm_bindgen(js_name = "playPause")]
pub fn play_pause() {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(AppEvent::PlayPause);
        }
    })
}

/// Advance one generation (pausing first if running).
#[wasm_bindgen(js_name = "stepForward")]
pub fn step_forward() {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(AppEvent::StepForward);
        }
    })
}

/// Fill the grid with random cells.
#[wasm_bindgen(js_name = "randomiseState")]
pub fn randomise_state() {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(AppEvent::RandomiseState);
        }
    })
}

/// Set the simulation speed from the slider and show the new value next to it.
#[wasm_bindgen(js_name = "updateFps")]
pub fn update_fps(new_fps: usize) {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(AppEvent::UpdateFps(new_fps));

            let window = web_sys::window().unwrap_throw();
            // get the DOM in the window
            let document = window.document().unwrap_throw();
            // The <span id="fpsValue"> next to the slider that shows the current speed.
            let fps_span = document.get_element_by_id("fpsValue").unwrap_throw();

            fps_span.set_text_content(Some(&new_fps.to_string()));
        }
    })
}

/// Kill every cell.
#[wasm_bindgen(js_name = "resetState")]
pub fn reset_state() {
    EVENT_LOOP_PROXY.with_borrow(|proxy| {
        if let Some(proxy) = proxy {
            let _ = proxy.send_event(AppEvent::ResetState);
        }
    })
}
