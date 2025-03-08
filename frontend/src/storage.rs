use wasm_bindgen::prelude::*;
use serde_json::{to_string, from_str};
use crate::state::AppState;
use crate::models::Node;
use std::collections::HashMap;

// Constants for storage keys
const NODES_KEY: &str = "zerg_nodes";
const VIEWPORT_KEY: &str = "zerg_viewport";
const SELECTED_MODEL_KEY: &str = "zerg_selected_model";
const ACTIVE_VIEW_KEY: &str = "zerg_active_view";

// Structure to store viewport data
#[derive(serde::Serialize, serde::Deserialize)]
struct ViewportData {
    x: f64,
    y: f64,
    zoom: f64,
}

// Store the active view (Dashboard or Canvas)
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ActiveView {
    Dashboard,
    Canvas,
}

/// Save current app state to localStorage
pub fn save_state(app_state: &AppState) -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    // Save nodes
    let nodes_json = to_string(&app_state.nodes)
        .map_err(|e| JsValue::from_str(&format!("Error serializing nodes: {}", e)))?;
    storage.set_item(NODES_KEY, &nodes_json)?;
    
    // Save viewport data
    let viewport_data = ViewportData {
        x: app_state.viewport_x,
        y: app_state.viewport_y,
        zoom: app_state.zoom_level,
    };
    let viewport_json = to_string(&viewport_data)
        .map_err(|e| JsValue::from_str(&format!("Error serializing viewport: {}", e)))?;
    storage.set_item(VIEWPORT_KEY, &viewport_json)?;
    
    // Save selected model
    storage.set_item(SELECTED_MODEL_KEY, &app_state.selected_model)?;
    
    // Save active view
    let active_view = if app_state.active_view == ActiveView::Dashboard { "dashboard" } else { "canvas" };
    storage.set_item(ACTIVE_VIEW_KEY, active_view)?;
    
    web_sys::console::log_1(&"State saved to localStorage".into());
    
    Ok(())
}

/// Load app state from localStorage
pub fn load_state(app_state: &mut AppState) -> Result<bool, JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    let mut data_loaded = false;
    
    // Load nodes
    if let Some(nodes_json) = storage.get_item(NODES_KEY)? {
        match from_str::<HashMap<String, Node>>(&nodes_json) {
            Ok(nodes) => {
                app_state.nodes = nodes;
                data_loaded = true;
                web_sys::console::log_1(&format!("Loaded {} nodes from storage", app_state.nodes.len()).into());
            },
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing nodes: {}", e)));
                // Continue loading other data even if nodes fail
            }
        }
    }
    
    // Load viewport
    if let Some(viewport_json) = storage.get_item(VIEWPORT_KEY)? {
        match from_str::<ViewportData>(&viewport_json) {
            Ok(viewport) => {
                app_state.viewport_x = viewport.x;
                app_state.viewport_y = viewport.y;
                app_state.zoom_level = viewport.zoom;
                data_loaded = true;
            },
            Err(e) => {
                web_sys::console::warn_1(&JsValue::from_str(&format!("Error parsing viewport: {}", e)));
            }
        }
    }
    
    // Load selected model
    if let Some(model) = storage.get_item(SELECTED_MODEL_KEY)? {
        app_state.selected_model = model;
        data_loaded = true;
    }
    
    // Load active view
    if let Some(view) = storage.get_item(ACTIVE_VIEW_KEY)? {
        app_state.active_view = if view == "dashboard" { ActiveView::Dashboard } else { ActiveView::Canvas };
        data_loaded = true;
    }
    
    Ok(data_loaded)
}

/// Clear all stored data
pub fn clear_storage() -> Result<(), JsValue> {
    let window = web_sys::window().expect("no global window exists");
    let storage = window.local_storage()?.expect("no localStorage exists");
    
    storage.remove_item(NODES_KEY)?;
    storage.remove_item(VIEWPORT_KEY)?;
    storage.remove_item(SELECTED_MODEL_KEY)?;
    storage.remove_item(ACTIVE_VIEW_KEY)?;
    
    Ok(())
} 