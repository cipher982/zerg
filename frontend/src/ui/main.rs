use wasm_bindgen::prelude::*;
use web_sys::Document;


// Create the input panel with controls
pub fn create_input_panel(document: &Document) -> Result<web_sys::Element, JsValue> {
    // "Find everything" button (centers and zooms to fit all nodes)
    let find_button = document.create_element("button")?;
    find_button.set_attribute("type", "button")?;
    find_button.set_inner_html("Find Everything");
    // This ID must match the one `setup_center_view_handler` looks for.
    find_button.set_attribute("id", "center-view")?;

    // Build the input panel (minimal controls for now)
    let input_panel = document.create_element("div")?;
    input_panel.set_id("canvas-input-panel");
    input_panel.append_child(&find_button)?;

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
