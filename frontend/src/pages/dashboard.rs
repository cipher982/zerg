// frontend/src/pages/dashboard.rs
//
// This file contains the Dashboard page component, responsible for
// mounting and unmounting the dashboard view.
//
use web_sys::{Document, Element};
use wasm_bindgen::JsValue;

/// Mount the dashboard view by creating necessary DOM elements
/// This function is called when switching to the dashboard view
pub fn mount_dashboard(document: &Document) -> Result<(), JsValue> {
    web_sys::console::log_1(&"DASHBOARD: Starting mount".into());
    
    // Get app container for adding dashboard
    let app_container = document
        .get_element_by_id("app-container")
        .ok_or(JsValue::from_str("Could not find app-container"))?;
    
    // Ensure we're using the default layout (not canvas-view)
    app_container.set_class_name("");

    // Create dashboard container if needed
    let dashboard_container = if let Some(container) = document.get_element_by_id("dashboard-container") {
        web_sys::console::log_1(&"DASHBOARD: Found existing container".into());
        container
    } else {
        // Create the container
        web_sys::console::log_1(&"DASHBOARD: Creating new container".into());
        let container = document.create_element("div")?;
        container.set_id("dashboard-container");
        container.set_class_name("dashboard-container");
        
        // Create dashboard element
        let dashboard = document.create_element("div")?;
        dashboard.set_id("dashboard");
        dashboard.set_class_name("dashboard");
        container.append_child(&dashboard)?;
        
        app_container.append_child(&container)?;
        container
    };

    // Ensure the dashboard is visible with proper styling
    web_sys::console::log_1(&"DASHBOARD: Making visible".into());
    dashboard_container.set_attribute("style", "display: block;")?;
    
    // Verify the dashboard inner element exists
    if document.get_element_by_id("dashboard").is_none() {
        web_sys::console::log_1(&"DASHBOARD: Creating inner element".into());
        let dashboard = document.create_element("div")?;
        dashboard.set_id("dashboard");
        dashboard.set_class_name("dashboard");
        dashboard_container.append_child(&dashboard)?;
    }
    
    // Update tab styling
    if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
        dashboard_tab.set_class_name("tab-button active");
    }
    
    if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
        canvas_tab.set_class_name("tab-button");
    }
    
    // Initialize or refresh the dashboard content
    web_sys::console::log_1(&"DASHBOARD: Refreshing content".into());
    crate::components::dashboard::refresh_dashboard(document)?;
    
    web_sys::console::log_1(&"DASHBOARD: Mount complete".into());
    Ok(())
}

/// Unmount the dashboard view by removing it from the DOM
/// This function is called when switching away from the dashboard view
pub fn unmount_dashboard(document: &Document) -> Result<(), JsValue> {
    if let Some(container) = document.get_element_by_id("dashboard-container") {
        // Option 1: Remove container completely from DOM
        if let Some(parent) = container.parent_node() {
            parent.remove_child(&container)?;
        }
    }
    
    Ok(())
}

/// Get a reference to the dashboard container element if it exists
pub fn get_dashboard_container(document: &Document) -> Option<Element> {
    document.get_element_by_id("dashboard-container")
}

/// Check if the dashboard is currently mounted
pub fn is_dashboard_mounted(document: &Document) -> bool {
    document.get_element_by_id("dashboard-container").is_some()
} 