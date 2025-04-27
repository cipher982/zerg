use wasm_bindgen::prelude::*;
use web_sys::Document;
use crate::state::APP_STATE;

// Create the input panel with controls
pub fn create_input_panel(document: &Document) -> Result<web_sys::Element, JsValue> {
    // Create auto-fit toggle switch
    let auto_fit_container = document.create_element("div")?;
    auto_fit_container.set_attribute("id", "auto-fit-container")?;
    auto_fit_container.set_attribute("class", "toggle-container")?;
    
    let auto_fit_label = document.create_element("label")?;
    auto_fit_label.set_attribute("for", "auto-fit-toggle")?;
    auto_fit_label.set_attribute("class", "toggle-label")?;
    
    let auto_fit_input = document.create_element("input")?;
    auto_fit_input.set_attribute("type", "checkbox")?;
    auto_fit_input.set_attribute("id", "auto-fit-toggle")?;
    auto_fit_input.set_attribute("class", "toggle-checkbox")?;
    
    // Check if auto-fit is enabled in current state
    let auto_fit_enabled = APP_STATE.with(|state| {
        let state = state.borrow();
        state.auto_fit
    });
    
    // Set the initial checked state based on the app state
    if auto_fit_enabled {
        auto_fit_input.set_attribute("checked", "")?;
    }
    
    let toggle_slider = document.create_element("span")?;
    toggle_slider.set_attribute("class", "toggle-slider")?;
    
    auto_fit_label.append_child(&auto_fit_input)?;
    auto_fit_label.append_child(&toggle_slider)?;
    auto_fit_container.append_child(&auto_fit_label)?;
    
    // Create center view button
    let center_button = document.create_element("button")?;
    center_button.set_inner_html("Center View");
    center_button.set_attribute("id", "center-button")?;
    center_button.set_class_name(""); // Remove any accidental class overrides

    // Create clear all button
    let clear_button = document.create_element("button")?;
    clear_button.set_inner_html("Clear All");
    clear_button.set_attribute("id", "clear-button")?;
    
    // Create input panel
    let input_panel = document.create_element("div")?;
    input_panel.set_id("input-panel");
    input_panel.append_child(&auto_fit_container)?;
    input_panel.append_child(&center_button)?;
    input_panel.append_child(&clear_button)?;
    
    Ok(input_panel)
}

// Setup base UI components
pub fn setup_ui(document: &Document) -> Result<(), JsValue> {
    // Create agent modal component (shared between views)
    crate::components::agent_config_modal::AgentConfigModal::init(document)?;
    
    // Setup event handlers that are common across views
    crate::ui::events::setup_ui_event_handlers(document)?;
    
    Ok(())
} 