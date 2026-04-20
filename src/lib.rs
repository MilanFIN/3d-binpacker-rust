pub mod common;
pub mod solver;
pub mod optimizer;

#[cfg(target_arch = "wasm32")]
pub mod wasm_api;

// Better panic messages in the browser console during WASM execution.
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
