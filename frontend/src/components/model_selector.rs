use wasm_bindgen::prelude::*;
use web_sys::{Document, Event};
use crate::state::APP_STATE;
use wasm_bindgen_futures::spawn_local;
use crate::network::ws_client_v2::fetch_available_models;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

pub fn update_model_dropdown(document: &Document) -> Result<(), JsValue> {
    if let Some(select_el) = document.get_element_by_id("model-select") {
        // Clear existing options
        select_el.set_inner_html("");
        
        // Get available models from state
        let models = APP_STATE.with(|state| {
            let state = state.borrow();
            state.available_models.clone()
        });
        
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

pub fn fetch_models_from_backend(document: &Document) -> Result<(), JsValue> {
    let document_clone = document.clone();
    
    // Fetch models asynchronously
    spawn_local(async move {
        if let Ok(()) = fetch_available_models().await {
            // Update the dropdown with fetched models
            let _ = update_model_dropdown(&document_clone);
        }
    });
    
    Ok(())
}

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
