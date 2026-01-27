pub mod app;

use crate::app::*;
use leptos::prelude::*;
use wasm_bindgen::prelude::wasm_bindgen;

/// This is the entry point for the browser/WASM side of the application.
/// The `hydrate` function "wakes up" the static HTML sent by the server
/// and attaches all the Rust event listeners (clicks, inputs, etc.)
#[wasm_bindgen]
pub fn hydrate() {
    // We only want this code to run when compiling for the browser (hydrate feature)
    #[cfg(feature = "hydrate")]
    {
        // Redirects Rust panic messages to the browser's console.log
        // Crucial for debugging WASM!
        console_error_panic_hook::set_once();

        // Optional: Initialize logging for the browser console
        _ = console_log::init_with_level(log::Level::Debug);

        // This attaches your <App /> component logic to the existing HTML body
        leptos::mount::hydrate_body(App);
    }
}
