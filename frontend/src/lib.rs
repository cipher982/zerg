use wasm_bindgen::prelude::*;

mod models;
mod state;
mod canvas;
mod ui;
mod network;

// Main entry point for the WASM application
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Set up the application
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    // Initialize the UI and canvas
    ui::setup_ui(&document)?;
    ui::setup_canvas(&document)?;
    
    // Set up the WebSocket connection
    network::setup_websocket()?;
    
    Ok(())
} 