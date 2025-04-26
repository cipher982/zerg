use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, MouseEvent, HtmlInputElement, HtmlTextAreaElement};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use crate::{messages::Message, state::dispatch_global_message};

// This will contain event handlers that we'll extract from ui.rs
// For now, we just have stubs

// NOTE: Modal-specific handlers (save / close / tabs) were migrated to
// `components::agent_config_modal`.  The legacy helpers have been removed to
// avoid dead code.

// Main function to set up all UI event handlers
pub fn setup_ui_event_handlers(document: &Document) -> Result<(), JsValue> {
    setup_auto_fit_button_handler(document)?;
    setup_center_view_handler(document)?;
    setup_clear_button_handler(document)?;
    setup_create_agent_button_handler(document)?;
    
    Ok(())
}

// Auto-fit toggle button handler
pub fn setup_auto_fit_button_handler(document: &Document) -> Result<(), JsValue> {
    // Get the auto-fit toggle switch
    if let Some(auto_fit_toggle) = document.get_element_by_id("auto-fit-toggle") {
        let change_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Use the global dispatch function which handles commands
            dispatch_global_message(Message::ToggleAutoFit);
        }) as Box<dyn FnMut(_)>);
        
        auto_fit_toggle.add_event_listener_with_callback(
            "change",
            change_handler.as_ref().unchecked_ref(),
        )?;
        
        // Keep the closure alive
        change_handler.forget();
    }
    
    Ok(())
}

// Center view button handler
pub fn setup_center_view_handler(document: &Document) -> Result<(), JsValue> {
    // Set up center view button click handler
    if let Some(center_button) = document.get_element_by_id("center-view") {
        let click_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
            // Use the global dispatch function which handles commands
            dispatch_global_message(Message::CenterView);
        }) as Box<dyn FnMut(_)>);
        
        center_button.add_event_listener_with_callback(
            "click",
            click_handler.as_ref().unchecked_ref(),
        )?;
        
        // Keep the closure alive
        click_handler.forget();
    }
    
    Ok(())
}

// Clear button handler
pub fn setup_clear_button_handler(document: &Document) -> Result<(), JsValue> {
    // Set up clear button click handler
    if let Some(clear_button) = document.get_element_by_id("clear-button") {
        let click_handler = Closure::wrap(Box::new(move |_e: Event| {
            // Show confirmation dialog
            let window = web_sys::window().expect("no global window exists");
            let confirm = window.confirm_with_message("Are you sure you want to clear all nodes? This cannot be undone.")
                .unwrap_or(false);
            
            if confirm {
                // Use the global dispatch function which handles commands
                dispatch_global_message(Message::ClearCanvas);
                
                // Clear storage
                if let Err(e) = crate::storage::clear_storage() {
                    web_sys::console::error_1(&format!("Failed to clear storage: {:?}", e).into());
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        clear_button.add_event_listener_with_callback(
            "click",
            click_handler.as_ref().unchecked_ref(),
        )?;
        
        // Keep the closure alive
        click_handler.forget();
    }
    
    Ok(())
}

// Setup modal handlers (close button and system instructions auto-save)
pub fn setup_modal_handlers(document: &Document) -> Result<(), JsValue> {
    // Close button
    let close_button = document.get_element_by_id("modal-close")
        .ok_or_else(|| JsValue::from_str("Modal close button not found"))?;
    
    let close_handler = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Get agent name from input
        let _name_value = if let Some(name_elem) = document.get_element_by_id("agent-name") {
            if let Some(name_input) = name_elem.dyn_ref::<HtmlInputElement>() {
                name_input.value()
            } else {
                String::new()
            }
        } else {
            String::new()
        };
        
        // Get system instructions
        let system_elem = document.get_element_by_id("system-instructions").unwrap();
        let system_textarea = system_elem.dyn_ref::<HtmlTextAreaElement>().unwrap();
        let _system_instructions = system_textarea.value();
        
        // Get default task instructions
        let default_task_elem = document.get_element_by_id("default-task-instructions").unwrap();
        let default_task_textarea = default_task_elem.dyn_ref::<HtmlTextAreaElement>().unwrap();
        let _task_instructions = default_task_textarea.value();
        
        // Use the global dispatch function which handles commands
        dispatch_global_message(Message::CloseAgentModal);
    }) as Box<dyn FnMut(_)>);
    
    close_button.add_event_listener_with_callback(
        "click",
        close_handler.as_ref().unchecked_ref(),
    )?;
    
    // Keep the closure alive
    close_handler.forget();
    
    Ok(())
}

// Set up tab switching for the modal
fn setup_tab_handlers(document: &Document) -> Result<(), JsValue> {
    // Get tab elements
    let main_tab = document.get_element_by_id("main-tab").ok_or(JsValue::from_str("Main tab not found"))?;
    let history_tab = document.get_element_by_id("history-tab").ok_or(JsValue::from_str("History tab not found"))?;
    
    // Main tab click handler
    let main_tab_handler = Closure::wrap(Box::new(move |_event: Event| {
        dispatch_global_message(Message::SwitchToMainTab);
    }) as Box<dyn FnMut(_)>);
    
    main_tab.add_event_listener_with_callback(
        "click",
        main_tab_handler.as_ref().unchecked_ref(),
    )?;
    main_tab_handler.forget();
    
    // History tab click handler
    let history_tab_handler = Closure::wrap(Box::new(move |_event: Event| {
        dispatch_global_message(Message::SwitchToHistoryTab);
    }) as Box<dyn FnMut(_)>);
    
    history_tab.add_event_listener_with_callback(
        "click",
        history_tab_handler.as_ref().unchecked_ref(),
    )?;
    history_tab_handler.forget();
    
    Ok(())
}

// Create Agent button handler
pub fn setup_create_agent_button_handler(document: &Document) -> Result<(), JsValue> {
    // Get the create agent button
    if let Some(create_agent_button) = document.get_element_by_id("create-agent-button") {
        let create_agent_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
            // Create a new agent node message
            let window = web_sys::window().expect("no global window exists");
            let _document = window.document().expect("should have a document on window");
            
            // Generate a random agent name
            let agent_name = format!("Agent {}", (js_sys::Math::random() * 10000.0).round());
            
            // ------------------------------------------------------------------
            // NEW simplified flow: delegate to the global message handler so the
            //    standard RequestCreateAgent → update.rs → NetworkCall path is
            //    reused.  This guarantees the selected model (fetched from the
            //    backend list) is used and avoids hand‑rolled JSON.
            // ------------------------------------------------------------------

            dispatch_global_message(crate::messages::Message::RequestCreateAgent {
                name: agent_name,
                system_instructions: "You are a helpful AI assistant.".to_string(),
                task_instructions: "Respond to user questions accurately and concisely.".to_string(),
            });
        }) as Box<dyn FnMut(_)>);
        
        create_agent_button.add_event_listener_with_callback(
            "click", 
            create_agent_handler.as_ref().unchecked_ref()
        )?;
        create_agent_handler.forget();
    }
    
    Ok(())
}

// Setup modal action handlers (save button and task sending)
pub fn setup_modal_action_handlers(document: &Document) -> Result<(), JsValue> {
    // Set up save button
    if let Some(save_button) = document.get_element_by_id("save-agent") {
        let save_handler = Closure::wrap(Box::new(move |_event: Event| {
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            
            // Get agent name from input
            let name_value = if let Some(name_elem) = document.get_element_by_id("agent-name") {
                if let Some(name_input) = name_elem.dyn_ref::<HtmlInputElement>() {
                    name_input.value()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            
            // Get system instructions
            let system_instructions = if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                if let Some(system_textarea) = system_elem.dyn_ref::<HtmlTextAreaElement>() {
                    system_textarea.value()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            
            // Get task instructions
            let task_instructions = if let Some(task_elem) = document.get_element_by_id("default-task-instructions") {
                if let Some(task_textarea) = task_elem.dyn_ref::<HtmlTextAreaElement>() {
                    task_textarea.value()
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            
            // Retrieve schedule cron string
            let schedule_value = if let Some(schedule_elem) = document.get_element_by_id("agent-schedule") {
                if let Some(schedule_input) = schedule_elem.dyn_ref::<HtmlInputElement>() {
                    let v = schedule_input.value();
                    if v.trim().is_empty() { None } else { Some(v) }
                } else { None }
            } else { None };

            // Retrieve run_on_schedule checkbox state
            let run_on_schedule = if let Some(enable_elem) = document.get_element_by_id("agent-run-on-schedule") {
                if let Some(enable_checkbox) = enable_elem.dyn_ref::<HtmlInputElement>() {
                    enable_checkbox.checked()
                } else { false }
            } else { false };

            // Log to console to help with debugging
            web_sys::console::log_1(&"Save button clicked - processing agent details".into());
            
            // Fetch the currently selected model from global state
            let selected_model = crate::state::APP_STATE.with(|state| {
                let state = state.borrow();
                state.selected_model.clone()
            });

            // Use the global dispatch function which handles commands
            dispatch_global_message(Message::SaveAgentDetails {
                name: name_value,
                system_instructions,
                task_instructions,
                model: selected_model,
                schedule: schedule_value,
                run_on_schedule,
            });
            
            // Show a visual feedback that the save was successful
            if let Some(save_btn) = document.get_element_by_id("save-agent") {
                let original_text = save_btn.inner_html();
                save_btn.set_inner_html("Saved!");
                
                // Reset button text after a delay
                let btn_clone = save_btn.clone();
                let text_clone = original_text.clone();
                let reset_btn = Closure::once_into_js(move || {
                    btn_clone.set_inner_html(&text_clone);
                });
                window.set_timeout_with_callback_and_timeout_and_arguments_0(
                    reset_btn.as_ref().unchecked_ref(), 
                    1500
                ).expect("Failed to set timeout");
            }
            
            // Explicitly close the modal after saving
            if let Err(e) = crate::components::agent_config_modal::AgentConfigModal::close(&document) {
                web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
            }
        }) as Box<dyn FnMut(_)>);
        
        save_button.add_event_listener_with_callback(
            "click",
            save_handler.as_ref().unchecked_ref(),
        )?;
        save_handler.forget();
    }
    
    // Set up send task button
    if let Some(send_button) = document.get_element_by_id("send-task") {
        let send_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Use the global dispatch function which handles commands
            dispatch_global_message(Message::SendTaskToAgent);
        }) as Box<dyn FnMut(_)>);
        
        send_button.add_event_listener_with_callback(
            "click",
            send_handler.as_ref().unchecked_ref(),
        )?;
        send_handler.forget();
    }
    
    Ok(())
}
