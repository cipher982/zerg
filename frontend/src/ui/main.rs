use wasm_bindgen::prelude::*;
use web_sys::Document;
use serde_json;


// Create the input panel with controls
pub fn create_input_panel(document: &Document) -> Result<web_sys::Element, JsValue> {
    // "Find everything" button (centres + zooms to fit content)
    let find_button = document.create_element("button")?;
    find_button.set_attribute("type", "button")?;
    find_button.set_inner_html("Center View");
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

    // Attach click handler for find_button now that element exists (in case
    // global event setup ran before the panel was created).
    {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            crate::state::dispatch_global_message(crate::messages::Message::CenterView);
        }));
        find_button.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }

    Ok(input_panel)
}

// Setup base UI components
pub fn setup_ui(document: &Document) -> Result<(), JsValue> {
    // Create agent modal component (shared between views)
    crate::components::agent_config_modal::AgentConfigModal::init(document)?;

    // Ensure workflows are loaded on initial app startup – especially when
    // landing directly on the Canvas page without going through the view
    // toggle reducer.
    wasm_bindgen_futures::spawn_local(async {
        match crate::network::api_client::ApiClient::get_workflows().await {
            Ok(json_str) => {
                if let Ok(api_wfs) = serde_json::from_str::<Vec<crate::models::ApiWorkflow>>(&json_str) {
                    let workflows: Vec<crate::models::Workflow> = api_wfs.into_iter().map(|w| w.into()).collect();
                    crate::state::dispatch_global_message(crate::messages::Message::WorkflowsLoaded(workflows));
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to fetch workflows on startup: {:?}", e).into());
            }
        }
    });
    
    // Setup event handlers that are common across views
    crate::ui::events::setup_ui_event_handlers(document)?;
    
    Ok(())
}
