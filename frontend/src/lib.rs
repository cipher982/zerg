use wasm_bindgen::prelude::*;
use web_sys::window;
use wasm_bindgen_futures::spawn_local;

mod models;
mod state;
mod canvas;
mod ui;
// mod ui_old; // Temporarily commented out to confirm everything is refactored
mod network;
mod favicon;
mod storage;
mod components;

// Main entry point for the WASM application
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Initialize better panic messages
    console_error_panic_hook::set_once();

    // Get the document
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    // Create base UI elements (header and status bar)
    ui::setup::create_base_ui(&document)?;
    
    // Set up the WebSocket connection
    network::setup_websocket()?;
    
    // Set up the UI components and canvas
    ui::main::setup_ui(&document)?;
    components::canvas_editor::setup_canvas(&document)?;
    
    // Load state from localStorage
    let loaded_data = state::APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        match storage::load_state(&mut state) {
            Ok(data_loaded) => {
                if data_loaded {
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
            state.fit_nodes_to_view();
            state.draw_nodes();
        });
    }
    
    // Generate favicon
    favicon::generate_favicon()?;
    
    // Set up auto-save timer
    setup_auto_save_timer(30000)?;
    
    // Load available models
    spawn_local(async {
        if let Err(e) = network::fetch_available_models().await {
            web_sys::console::log_1(&format!("Error fetching models: {:?}", e).into());
        }
    });
    
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