use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, HtmlTextAreaElement, MouseEvent, HtmlInputElement};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;
use crate::{
    messages::Message,
    models::NodeType,
    state::APP_STATE,
};
use web_sys::HtmlElement;

// This will contain event handlers that we'll extract from ui.rs
// For now, we just have stubs

// Main function to set up all UI event handlers
pub fn setup_ui_event_handlers(document: &Document) -> Result<(), JsValue> {
    setup_auto_fit_button_handler(document)?;
    setup_center_view_handler(document)?;
    setup_clear_button_handler(document)?;
    setup_modal_handlers(document)?;
    setup_create_agent_button_handler(document)?;
    
    Ok(())
}

// Auto-fit toggle button handler
pub fn setup_auto_fit_button_handler(document: &Document) -> Result<(), JsValue> {
    let auto_fit_toggle = document.get_element_by_id("auto-fit-toggle")
        .ok_or_else(|| JsValue::from_str("Auto-fit toggle not found"))?;
    
    let change_handler = Closure::wrap(Box::new(move |_event: Event| {
        // Use the new dispatch method with ToggleAutoFit message
        let need_refresh = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::ToggleAutoFit)
        });

        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    auto_fit_toggle.add_event_listener_with_callback(
        "change",
        change_handler.as_ref().unchecked_ref(),
    )?;
    
    // Keep the closure alive
    change_handler.forget();
    
    Ok(())
}

// Center view button handler
pub fn setup_center_view_handler(document: &Document) -> Result<(), JsValue> {
    let center_button = document.get_element_by_id("center-button")
        .ok_or_else(|| JsValue::from_str("Center button not found"))?;
    
    let click_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
        // Use the new dispatch method with CenterView message
        let need_refresh = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::CenterView)
        });

        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    center_button.add_event_listener_with_callback(
        "click",
        click_handler.as_ref().unchecked_ref(),
    )?;
    
    // Keep the closure alive
    click_handler.forget();
    
    Ok(())
}

// Clear button handler
pub fn setup_clear_button_handler(document: &Document) -> Result<(), JsValue> {
    let clear_button = document.get_element_by_id("clear-button")
        .ok_or_else(|| JsValue::from_str("Clear button not found"))?;
    
    let click_handler = Closure::wrap(Box::new(move |_e: Event| {
        // Show confirmation dialog
        let window = web_sys::window().expect("no global window exists");
        let confirm = window.confirm_with_message("Are you sure you want to clear all nodes? This cannot be undone.")
            .unwrap_or(false);
        
        if confirm {
            // Use the new dispatch method with ClearCanvas message
            let need_refresh = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::ClearCanvas)
            });
            
            // Clear storage
            if let Err(e) = crate::storage::clear_storage() {
                web_sys::console::error_1(&format!("Failed to clear storage: {:?}", e).into());
            }

            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    clear_button.add_event_listener_with_callback(
        "click",
        click_handler.as_ref().unchecked_ref(),
    )?;
    
    // Keep the closure alive
    click_handler.forget();
    
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
        let name_value = if let Some(name_elem) = document.get_element_by_id("agent-name") {
            if let Some(name_input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
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
        let system_instructions = system_textarea.value();
        
        // Get default task instructions
        let default_task_elem = document.get_element_by_id("default-task-instructions").unwrap();
        let default_task_textarea = default_task_elem.dyn_ref::<HtmlTextAreaElement>().unwrap();
        let task_instructions = default_task_textarea.value();
        
        // We'll save and close the modal in a single operation to prevent RefCell borrow errors
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // First save the agent details by directly calling the update function
            crate::update::update(&mut state, Message::SaveAgentDetails {
                name: name_value,
                system_instructions,
                task_instructions,
            });
            
            // Then close the modal using the dispatch method (which will handle UI refresh)
            let _ = state.dispatch(Message::CloseAgentModal);
        });
    }) as Box<dyn FnMut(_)>);
    
    close_button.add_event_listener_with_callback(
        "click",
        close_handler.as_ref().unchecked_ref(),
    )?;
    close_handler.forget();
    
    // Set up save button
    if let Some(save_button) = document.get_element_by_id("save-agent") {
        let save_handler = Closure::wrap(Box::new(move |_event: Event| {
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            
            // Get agent name from input
            let name_value = if let Some(name_elem) = document.get_element_by_id("agent-name") {
                if let Some(name_input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
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
            
            // Save agent details using the new message pattern
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let _ = state.dispatch(Message::SaveAgentDetails {
                    name: name_value,
                    system_instructions,
                    task_instructions,
                });
            });
            
            // Show a temporary "Saved!" message
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
        }) as Box<dyn FnMut(_)>);
        
        save_button.add_event_listener_with_callback(
            "click",
            save_handler.as_ref().unchecked_ref(),
        )?;
        save_handler.forget();
    }
    
    // Set up send button
    if let Some(send_button) = document.get_element_by_id("send-to-agent") {
        let send_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Use the new message pattern
            let need_refresh = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::SendTaskToAgent)
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        send_button.add_event_listener_with_callback(
            "click",
            send_handler.as_ref().unchecked_ref(),
        )?;
        send_handler.forget();
    }
    
    // Set up tab switching
    setup_tab_handlers(document)?;
    
    // Add input listener to system instructions to auto-save after a delay
    let system_textarea = document.get_element_by_id("system-instructions")
        .ok_or_else(|| JsValue::from_str("System instructions textarea not found"))?;
    
    // Create a new timeout ID reference
    let timeout_id_ref = Rc::new(RefCell::new(None::<i32>));
    
    let input_timeout_id = timeout_id_ref.clone();
    let input_handler = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Get the current value from the textarea
        let system_elem = document.get_element_by_id("system-instructions").unwrap();
        let system_textarea = system_elem.dyn_ref::<HtmlTextAreaElement>().unwrap();
        let system_instructions = system_textarea.value();
        
        // Clear previous timeout if it exists
        if let Some(id) = *input_timeout_id.borrow() {
            window.clear_timeout_with_handle(id);
        }
        
        // Save system instructions function with debounce
        let system_instructions_clone = system_instructions.clone();
        let save_function = Closure::once_into_js(move || {
            // Use the new message pattern
            let need_refresh = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::UpdateSystemInstructions(system_instructions_clone))
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        });
        
        // Set new timeout (500ms debounce)
        let new_timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments(
                save_function.as_ref().unchecked_ref(),
                500,  // 500ms debounce
                &js_sys::Array::new(),
            )
            .expect("Failed to set timeout");
        
        *input_timeout_id.borrow_mut() = Some(new_timeout_id);
    }) as Box<dyn FnMut(_)>);
    
    system_textarea.add_event_listener_with_callback(
        "input",
        input_handler.as_ref().unchecked_ref(),
    )?;
    input_handler.forget();
    
    // Add input listener to agent name field to auto-save after a delay
    let agent_name_input = document.get_element_by_id("agent-name")
        .ok_or_else(|| JsValue::from_str("Agent name input not found"))?;
    
    // Create a new timeout ID reference for agent name
    let name_timeout_id_ref = Rc::new(RefCell::new(None::<i32>));
    
    let name_input_timeout_id = name_timeout_id_ref.clone();
    let name_input_handler = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Get the current value from the input field
        let name_elem = document.get_element_by_id("agent-name").unwrap();
        let name_input = name_elem.dyn_ref::<web_sys::HtmlInputElement>().unwrap();
        let name_value = name_input.value();
        
        // Only proceed if name is not empty
        if name_value.trim().is_empty() {
            return;
        }
        
        // Clear previous timeout if it exists
        if let Some(id) = *name_input_timeout_id.borrow() {
            window.clear_timeout_with_handle(id);
        }
        
        // Save agent name function with debounce
        let name_value_clone = name_value.clone();
        let save_function = Closure::once_into_js(move || {
            // Use the new message pattern
            let need_refresh = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::UpdateAgentName(name_value_clone))
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
        });
        
        // Set new timeout (500ms debounce)
        let new_timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments(
                save_function.as_ref().unchecked_ref(),
                500,  // 500ms debounce
                &js_sys::Array::new(),
            )
            .expect("Failed to set timeout");
        
        *name_input_timeout_id.borrow_mut() = Some(new_timeout_id);
    }) as Box<dyn FnMut(_)>);
    
    agent_name_input.add_event_listener_with_callback(
        "input",
        name_input_handler.as_ref().unchecked_ref(),
    )?;
    name_input_handler.forget();
    
    Ok(())
}

// Set up tab switching for the modal
fn setup_tab_handlers(document: &Document) -> Result<(), JsValue> {
    // For each tab button, add click handler to show the corresponding content
    let main_tab = document.get_element_by_id("main-tab")
        .ok_or_else(|| JsValue::from_str("Main tab button not found"))?;
    
    let history_tab = document.get_element_by_id("history-tab")
        .ok_or_else(|| JsValue::from_str("History tab button not found"))?;
    
    // Main tab click handler
    let main_click = Closure::wrap(Box::new(move |_event: Event| {
        // Use the new message pattern
        let need_refresh = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::SwitchToMainTab)
        });
        
        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    main_tab.add_event_listener_with_callback(
        "click",
        main_click.as_ref().unchecked_ref(),
    )?;
    main_click.forget();
    
    // History tab click handler
    let history_click = Closure::wrap(Box::new(move |_event: Event| {
        // Use the new message pattern
        let need_refresh = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::SwitchToHistoryTab)
        });
        
        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    history_tab.add_event_listener_with_callback(
        "click",
        history_click.as_ref().unchecked_ref(),
    )?;
    history_click.forget();
    
    Ok(())
}

// Create Agent button handler
pub fn setup_create_agent_button_handler(document: &Document) -> Result<(), JsValue> {
    let create_agent_button = document.get_element_by_id("create-agent-button")
        .ok_or_else(|| JsValue::from_str("Create agent button not found"))?;
    
    let click_callback = Closure::wrap(Box::new(move |_event: MouseEvent| {
        // Use the new dispatch method with AddNode message
        let need_refresh = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::AddNode {
                text: "New Agent".to_string(),
                // Calculate position - we need to get the viewport center
                // This logic is now handled in the update function
                x: 0.0, // Will be calculated in update
                y: 0.0, // Will be calculated in update
                node_type: NodeType::AgentIdentity
            })
        });
        
        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    create_agent_button.add_event_listener_with_callback(
        "click", 
        click_callback.as_ref().unchecked_ref()
    )?;
    click_callback.forget();
    
    Ok(())
}
