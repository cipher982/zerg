use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, HtmlInputElement, HtmlTextAreaElement, MouseEvent};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use crate::{
    messages::Message,
    models::NodeType,
    state::APP_STATE,
};

// This will contain event handlers that we'll extract from ui.rs
// For now, we just have stubs

// Main function to set up all UI event handlers
pub fn setup_ui_event_handlers(document: &Document) -> Result<(), JsValue> {
    setup_auto_fit_button_handler(document)?;
    setup_center_view_handler(document)?;
    setup_clear_button_handler(document)?;
    setup_modal_handlers(document)?;
    setup_modal_action_handlers(document)?;
    setup_create_agent_button_handler(document)?;
    
    Ok(())
}

// Auto-fit toggle button handler
pub fn setup_auto_fit_button_handler(document: &Document) -> Result<(), JsValue> {
    // Get the auto-fit toggle switch
    if let Some(auto_fit_toggle) = document.get_element_by_id("auto-fit-toggle") {
        let change_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Use the new dispatch method with ToggleAutoFit message
            let (need_refresh, pending_call) = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::ToggleAutoFit)
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
            
            // Handle any pending network calls (shouldn't be any for ToggleAutoFit)
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
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
            // Use the new dispatch method with CenterView message
            let (need_refresh, pending_call) = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::CenterView)
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
            
            // Handle any pending network calls (shouldn't be any for CenterView)
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
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
                // Use the new dispatch method with ClearCanvas message
                let (need_refresh, pending_call) = APP_STATE.with(|state| {
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
                
                // Handle any pending network calls (shouldn't be any for ClearCanvas)
                if let Some((task_text, message_id)) = pending_call {
                    crate::network::send_text_to_backend(&task_text, message_id);
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
        
        let (need_refresh, pending_call) = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            let _ = state.dispatch(Message::CloseAgentModal);
            
            // Return values that match the expected return from other dispatch calls
            (true, None::<(String, String)>)
        });
        
        if need_refresh {
            let _ = crate::state::AppState::refresh_ui_after_state_change();
        }
        
        // Handle any pending network calls (shouldn't be any for CloseAgentModal)
        if let Some((task_text, message_id)) = pending_call {
            crate::network::send_text_to_backend(&task_text, message_id);
        }
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
            
            // Log to console to help with debugging
            web_sys::console::log_1(&"Save button clicked - processing agent details".into());
            
            let (need_refresh, pending_call) = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let _ = state.dispatch(Message::SaveAgentDetails {
                    name: name_value.clone(),
                    system_instructions: system_instructions.clone(),
                    task_instructions: task_instructions.clone(),
                });
                
                // Return values that match the expected return from other dispatch calls
                (true, None::<(String, String)>)
            });
            
            if need_refresh {
                let _ = crate::state::AppState::refresh_ui_after_state_change();
            }
            
            // Handle any pending network calls (shouldn't be any for SaveAgentDetails)
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
            
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
            if let Err(e) = crate::ui::modals::close_agent_modal(&document) {
                web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
            }
        }) as Box<dyn FnMut(_)>);
        
        save_button.add_event_listener_with_callback(
            "click",
            save_handler.as_ref().unchecked_ref(),
        )?;
        save_handler.forget();
    }
    
    // Set up tab switching
    setup_tab_handlers(document)?;
    
    Ok(())
}

// Set up tab switching for the modal
fn setup_tab_handlers(document: &Document) -> Result<(), JsValue> {
    // Get tab elements
    let main_tab = document.get_element_by_id("main-tab").ok_or(JsValue::from_str("Main tab not found"))?;
    let history_tab = document.get_element_by_id("history-tab").ok_or(JsValue::from_str("History tab not found"))?;
    
    // Main tab click handler
    let main_tab_handler = Closure::wrap(Box::new(move |_event: Event| {
        // Use the new message pattern
        let (need_refresh, pending_call) = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::SwitchToMainTab)
        });
        
        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
        
        // Handle any pending network calls (shouldn't be any for SwitchToMainTab)
        if let Some((task_text, message_id)) = pending_call {
            crate::network::send_text_to_backend(&task_text, message_id);
        }
    }) as Box<dyn FnMut(_)>);
    
    main_tab.add_event_listener_with_callback(
        "click",
        main_tab_handler.as_ref().unchecked_ref(),
    )?;
    main_tab_handler.forget();
    
    // History tab click handler
    let history_tab_handler = Closure::wrap(Box::new(move |_event: Event| {
        // Use the new message pattern
        let (need_refresh, pending_call) = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dispatch(Message::SwitchToHistoryTab)
        });
        
        // After borrowing mutably, we can refresh UI if needed in a separate borrow
        if need_refresh {
            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
            }
        }
        
        // Handle any pending network calls (shouldn't be any for SwitchToHistoryTab)
        if let Some((task_text, message_id)) = pending_call {
            crate::network::send_text_to_backend(&task_text, message_id);
        }
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
            
            // First create the agent in the API
            let agent_data = format!(
                r#"{{
                    "name": "{}",
                    "system_instructions": "You are a helpful AI assistant.",
                    "task_instructions": "Respond to user questions accurately and concisely.",
                    "model": "gpt-3.5-turbo"
                }}"#,
                agent_name
            );
            
            // This is the ONLY place where agents should be created in the API
            web_sys::console::log_1(&"Creating new agent via the proper API call".into());
            
            // Use async block to call the API
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::ApiClient::create_agent(&agent_data).await {
                    Ok(response) => {
                        // Parse the response to get the agent ID
                        if let Ok(json) = js_sys::JSON::parse(&response) {
                            if let Some(id) = js_sys::Reflect::get(&json, &"id".into()).ok()
                                .and_then(|v| v.as_f64()) 
                            {
                                let agent_id = id as u32;
                                web_sys::console::log_1(&format!("Successfully created agent with ID: {}", agent_id).into());
                                
                                // Now create a node in the UI for this agent
                                let (x, y) = APP_STATE.with(|state| {
                                    let state = state.borrow();
                                    (state.viewport_x + 200.0, state.viewport_y + 100.0)
                                });
                                
                                // Create the message with the agent node - correctly ID'ed as agent-{id}
                                let node_id = format!("agent-{}", agent_id);
                                
                                // Use AddNode with the assigned agent ID
                                APP_STATE.with(|state| {
                                    let mut state = state.borrow_mut();
                                    let node = crate::models::Node {
                                        node_id: node_id.clone(),
                                        agent_id: Some(agent_id),
                                        x,
                                        y,
                                        text: agent_name,
                                        width: 200.0,
                                        height: 80.0,
                                        color: "#ffecb3".to_string(), // Light amber color
                                        parent_id: None,
                                        node_type: NodeType::AgentIdentity,
                                        is_selected: false,
                                        is_dragging: false,
                                    };
                                    
                                    state.nodes.insert(node_id, node);
                                    state.state_modified = true;
                                    state.draw_nodes(); // Redraw to show the new node
                                });
                                
                                // Refresh the UI
                                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                    web_sys::console::warn_1(&format!("Failed to refresh UI after agent creation: {:?}", e).into());
                                }
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to create agent: {:?}", e).into());
                    }
                }
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

// Set up modal handlers for close, save, and send actions
pub fn setup_modal_action_handlers(document: &Document) -> Result<(), JsValue> {
    // Set up close button
    if let Some(close_button) = document.get_element_by_id("close-agent-modal") {
        let close_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Close the modal using the new message pattern
            let (need_refresh, pending_call) = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::CloseAgentModal)
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
            
            // Handle any pending network calls (shouldn't be any for CloseAgentModal)
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
        }) as Box<dyn FnMut(_)>);
        
        close_button.add_event_listener_with_callback(
            "click",
            close_handler.as_ref().unchecked_ref(),
        )?;
        close_handler.forget();
    }
    
    // Set up save button
    if let Some(save_button) = document.get_element_by_id("save-agent") {
        let save_handler = Closure::wrap(Box::new(move |_event: Event| {
            // Get fields from the form
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document on window");
            
            let name_input = document.get_element_by_id("agent-name")
                .expect("should have agent-name input")
                .dyn_into::<HtmlInputElement>()
                .expect("agent-name should be an input");
            
            let system_instructions_textarea = document.get_element_by_id("system-instructions")
                .expect("should have system-instructions textarea")
                .dyn_into::<HtmlTextAreaElement>()
                .expect("system-instructions should be a textarea");
            
            let default_task_textarea = document.get_element_by_id("default-task-instructions")
                .expect("should have default-task-instructions textarea")
                .dyn_into::<HtmlTextAreaElement>()
                .expect("default-task-instructions should be a textarea");
            
            let name_value = name_input.value();
            let system_instructions = system_instructions_textarea.value();
            let task_instructions = default_task_textarea.value();
            
            // Log to console to help with debugging
            web_sys::console::log_1(&"Save button clicked via action handlers - processing agent details".into());
            
            // Save agent details using the new message pattern
            let (need_refresh, pending_call) = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(Message::SaveAgentDetails {
                    name: name_value.clone(),
                    system_instructions: system_instructions.clone(),
                    task_instructions: task_instructions.clone(),
                })
            });
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
            
            // Handle any pending network calls (shouldn't be any for SaveAgentDetails)
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
            
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
            
            // Explicitly close the modal after saving
            if let Err(e) = crate::ui::modals::close_agent_modal(&document) {
                web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
            }
        }) as Box<dyn FnMut(_)>);
        
        save_button.add_event_listener_with_callback(
            "click",
            save_handler.as_ref().unchecked_ref(),
        )?;
        save_handler.forget();
    }
    
    Ok(())
}
