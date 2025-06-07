use wasm_bindgen::prelude::*;
use web_sys::Document;


// Create the input panel with controls
pub fn create_input_panel(document: &Document) -> Result<web_sys::Element, JsValue> {
    // "Find everything" button (centres + zooms to fit content)
    let find_button = document.create_element("button")?;
    find_button.set_attribute("type", "button")?;
    find_button.set_inner_html("Find Everything");
    // Match handler ID
    find_button.set_attribute("id", "center-view")?;
    find_button.set_attribute("class", "toolbar-btn")?;

    // ------------------------------
    // More (…) dropdown with Clear
    // ------------------------------
    // <details><summary>⋮</summary><button id="clear-button">Clear Canvas</button></details>
    let more_details = document.create_element("details")?;
    more_details.set_attribute("id", "more-menu")?;
    let summary = document.create_element("summary")?;
    summary.set_inner_html("&#x22EE;"); // Unicode vertical ellipsis
    more_details.append_child(&summary)?;

    let clear_btn = document.create_element("button")?;
    clear_btn.set_attribute("type", "button")?;
    clear_btn.set_inner_html("Clear Canvas");
    clear_btn.set_attribute("id", "clear-button")?;

    more_details.append_child(&clear_btn)?;

    // Build the input panel (minimal controls for now)
    let input_panel = document.create_element("div")?;
    input_panel.set_id("canvas-input-panel");
    input_panel.append_child(&find_button)?;
    input_panel.append_child(&more_details)?;

    Ok(input_panel)
}

// Setup base UI components
pub fn setup_ui(document: &Document) -> Result<(), JsValue> {
    // Create agent modal component (shared between views)
    crate::components::agent_config_modal::AgentConfigModal::init(document)?;

    // Workflow tab bar (initial render)
    crate::components::workflow_switcher::init(document)?;
    
    // Setup event handlers that are common across views
    crate::ui::events::setup_ui_event_handlers(document)?;
    
    Ok(())
}
