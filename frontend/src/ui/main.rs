use wasm_bindgen::prelude::*;
use web_sys::Document;
use crate::state::APP_STATE;
use crate::components::model_selector;
use crate::ui::events;

pub fn setup_ui(document: &Document) -> Result<(), JsValue> {
    // Find the app container
    let app_container = document
        .get_element_by_id("app-container")
        .ok_or(JsValue::from_str("Could not find app-container"))?;
    
    // Create "Create Agent" button instead of input field
    let create_agent_button = document.create_element("button")?;
    create_agent_button.set_inner_html("Create Agent");
    create_agent_button.set_attribute("id", "create-agent-button")?;
    
    // Create model selection dropdown
    let model_select = document.create_element("select")?;
    model_select.set_attribute("id", "model-select")?;
    
    // Initially populate with default models
    model_selector::update_model_dropdown(document)?;
    
    // Create auto-fit toggle switch instead of button
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
    
    // Create clear all button
    let clear_button = document.create_element("button")?;
    clear_button.set_inner_html("Clear All");
    clear_button.set_attribute("id", "clear-button")?;
    
    // Create input panel (controls)
    let input_panel = document.create_element("div")?;
    input_panel.set_id("input-panel");
    input_panel.append_child(&create_agent_button)?;
    input_panel.append_child(&model_select)?;
    input_panel.append_child(&auto_fit_container)?;
    input_panel.append_child(&center_button)?;
    input_panel.append_child(&clear_button)?;
    
    // Create canvas container
    let canvas_container = document.create_element("div")?;
    canvas_container.set_id("canvas-container");
    
    // Create canvas
    let canvas = document.create_element("canvas")?;
    canvas.set_id("node-canvas");
    canvas_container.append_child(&canvas)?;
    
    // Add elements to the app container
    app_container.append_child(&input_panel)?;
    app_container.append_child(&canvas_container)?;
    
    // Create agent modal component
    crate::components::agent_config_modal::AgentConfigModal::init(document)?;
    
    // Set up event handlers
    events::setup_ui_event_handlers(document)?;
    model_selector::setup_model_selector_handlers(document)?;
    
    // Fetch models from backend
    model_selector::fetch_models_from_backend(document)?;
    
    Ok(())
} 