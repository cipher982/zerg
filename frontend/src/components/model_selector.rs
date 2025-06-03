use wasm_bindgen::prelude::*;
use web_sys::{Document, Event};
use crate::state::APP_STATE;
use wasm_bindgen_futures::spawn_local;
use crate::network::api_client::ApiClient;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[allow(dead_code)] // experimental component – not wired into DOM yet
pub fn update_model_dropdown(document: &Document) -> Result<(), JsValue> {
    if let Some(select_el) = document.get_element_by_id("model-select") {
        // Clear existing options
        select_el.set_inner_html("");
        
        // Get available models from state
        let models = APP_STATE.with(|state| {
            let state = state.borrow();
            state.available_models.clone()
        });
        
        // Skip if we have no models yet – list will be populated once the
        // backend fetch completes.
        if models.is_empty() {
            return Ok(());
        }

        // Get selected model
        let selected_model = APP_STATE.with(|state| {
            let state = state.borrow();
            state.selected_model.clone()
        });
        
        // Add options to dropdown
        for (value, label) in models.iter() {
            let option = document.create_element("option")?;
            option.set_attribute("value", value)?;
            
            // Set selected if it matches current selection
            if value == &selected_model {
                option.set_attribute("selected", "selected")?;
            }
            
            option.set_inner_html(label);
            select_el.append_child(&option)?;
        }
    }
    
    Ok(())
}

#[allow(dead_code)]
pub fn fetch_models_from_backend(document: &Document) -> Result<(), JsValue> {
    let document_clone = document.clone();
    
    // Fetch models asynchronously
    spawn_local(async move {
        if let Ok(response) = ApiClient::fetch_available_models().await {
            // Parse the response and update state
            if let Ok(models_json) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(models) = models_json.get("models").and_then(|m| m.as_array()) {
                    let models: Vec<(String, String)> = models.iter()
                        .filter_map(|m| m.as_str())
                        .map(|m| (m.to_string(), m.to_string()))
                        .collect();
                    
                    // Update state with available models
                    APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.available_models = models;
                    });
                    
                    // Update the dropdown with fetched models
                    let _ = update_model_dropdown(&document_clone);
                }
            }
        }
    });
    
    Ok(())
}

#[allow(dead_code)]
pub fn setup_model_selector_handlers(document: &Document) -> Result<(), JsValue> {
    let select_el = document.get_element_by_id("model-select")
        .ok_or_else(|| JsValue::from_str("Model select element not found"))?;
    
    let change_handler = Closure::wrap(Box::new(move |event: Event| {
        let select = event.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        let model = select.value();
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.selected_model = model;
            state.state_modified = true; // Mark as modified
            let _ = state.save_if_modified();
        });
    }) as Box<dyn FnMut(_)>);
    
    select_el.add_event_listener_with_callback(
        "change",
        change_handler.as_ref().unchecked_ref(),
    )?;
    change_handler.forget();
    
    Ok(())
}
