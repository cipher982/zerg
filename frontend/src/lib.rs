use wasm_bindgen::prelude::*;
use web_sys::window;

mod models;
mod state;
mod canvas;
mod ui;
mod network;
mod favicon;
mod storage;

// Main entry point for the WASM application
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Set up the application
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    // Initialize the UI and canvas first
    ui::setup_ui(&document)?;
    ui::setup_canvas(&document)?;
    
    // Load state from localStorage
    let loaded_data = state::APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        match storage::load_state(&mut state) {
            Ok(data_loaded) => {
                // Call enforce_viewport_boundaries and handle the result properly
                if data_loaded {
                    // Only enforce boundaries if we loaded data
                    state.enforce_viewport_boundaries();
                }
                data_loaded
            },
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to load state: {:?}", e).into());
                false
            }
        }
    });
    
    // Draw nodes if we loaded any data
    if loaded_data {
        state::APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            // Make sure to fit nodes to view on initial load
            state.fit_nodes_to_view();
            state.draw_nodes();
        });
    }
    
    // Generate favicon
    favicon::generate_favicon()?;
    
    // Set up the WebSocket connection
    network::setup_websocket()?;
    
    // Set up auto-save timer (every 30 seconds)
    setup_auto_save_timer(30000)?;
    
    Ok(())
}

// Set up a timer to auto-save state periodically
fn setup_auto_save_timer(interval_ms: i32) -> Result<(), JsValue> {
    let window = window().expect("no global window exists");
    
    // Create a closure that will be called by setInterval
    let auto_save_callback = Closure::wrap(Box::new(move || {
        state::APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Err(e) = state.save_if_modified() {
                web_sys::console::warn_1(&format!("Auto-save failed: {:?}", e).into());
            } else {
                // Optional: log successful saves for debugging
                // web_sys::console::log_1(&"Auto-saved state".into());
            }
        });
    }) as Box<dyn FnMut()>);
    
    // Create interval timer
    let _ = window.set_interval_with_callback_and_timeout_and_arguments(
        auto_save_callback.as_ref().unchecked_ref(),
        interval_ms,
        &js_sys::Array::new(),
    )?;
    
    // Forget the closure to prevent it from being dropped
    auto_save_callback.forget();
    
    Ok(())
} 