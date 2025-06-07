//! WASM wrappers around the Weave API.

use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, tapestry-weave!");
}

pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}
