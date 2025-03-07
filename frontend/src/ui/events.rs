use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, HtmlTextAreaElement, MouseEvent};
use crate::state::APP_STATE;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use std::cell::RefCell;
use crate::models::NodeType;

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
        // Use the toggle_auto_fit method to toggle the auto-fit state
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.toggle_auto_fit();
        });
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
    
    let click_callback = Closure::wrap(Box::new(move |_event: Event| {
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.center_view();
        });
    }) as Box<dyn FnMut(_)>);
    
    center_button.add_event_listener_with_callback(
        "click",
        click_callback.as_ref().unchecked_ref(),
    )?;
    click_callback.forget();
    
    Ok(())
}

// Clear button handler
pub fn setup_clear_button_handler(document: &Document) -> Result<(), JsValue> {
    let clear_button = document.get_element_by_id("clear-button")
        .ok_or_else(|| JsValue::from_str("Clear button not found"))?;
    
    let click_callback = Closure::wrap(Box::new(move |_e: Event| {
        // Show confirmation dialog
        let window = web_sys::window().expect("no global window exists");
        let confirm = window.confirm_with_message("Are you sure you want to clear all nodes? This cannot be undone.")
            .unwrap_or(false);
        
        if confirm {
            // Clear state and storage
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.nodes.clear();
                state.latest_user_input_id = None;
                state.message_id_to_node_id.clear();
                state.viewport_x = 0.0;
                state.viewport_y = 0.0;
                state.zoom_level = 1.0;
                state.auto_fit = true;
                state.state_modified = true;
                
                state.draw_nodes();
                
                // Clear storage
                if let Err(e) = crate::storage::clear_storage() {
                    web_sys::console::error_1(&format!("Failed to clear storage: {:?}", e).into());
                }
            });
        }
    }) as Box<dyn FnMut(_)>);
    
    clear_button.add_event_listener_with_callback(
        "click",
        click_callback.as_ref().unchecked_ref(),
    )?;
    click_callback.forget();
    
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
        let modal = document.get_element_by_id("agent-modal").unwrap();
        modal.set_attribute("style", "display: none;").unwrap();
        
        // Auto-save any pending changes before closing
        let system_elem = document.get_element_by_id("system-instructions").unwrap();
        let system_textarea = system_elem.dyn_ref::<HtmlTextAreaElement>().unwrap();
        let system_instructions = system_textarea.value();
        
        // Save system instructions to the selected node
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(id) = &state.selected_node_id {
                let id_clone = id.clone();
                if let Some(node) = state.nodes.get_mut(&id_clone) {
                    node.system_instructions = Some(system_instructions.clone());
                    state.state_modified = true;
                    let _ = state.save_if_modified();
                }
            }
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
            
            if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                if let Some(system_textarea) = system_elem.dyn_ref::<HtmlTextAreaElement>() {
                    let system_instructions = system_textarea.value();
                    
                    // Save to APP_STATE
                    APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        if let Some(id) = &state.selected_node_id {
                            let id_clone = id.clone();
                            if let Some(node) = state.nodes.get_mut(&id_clone) {
                                node.system_instructions = Some(system_instructions.clone());
                                state.state_modified = true;
                                let _ = state.save_if_modified();
                                
                                // Show a temporary "Saved!" message
                                web_sys::console::log_1(&"System instructions saved".into());
                                
                                // Update button text temporarily
                                if let Some(save_btn) = document.get_element_by_id("save-agent") {
                                    let original_text = save_btn.inner_html();
                                    save_btn.set_inner_html("Saved!");
                                    
                                    // Reset button text after a delay
                                    let btn_clone = save_btn.clone();
                                    let text_clone = original_text.clone();
                                    let reset_btn = Closure::once_into_js(move || {
                                        btn_clone.set_inner_html(&text_clone);
                                    });
                                    
                                    window
                                        .set_timeout_with_callback_and_timeout_and_arguments(
                                            reset_btn.as_ref().unchecked_ref(),
                                            1500,  // 1.5 second delay
                                            &js_sys::Array::new(),
                                        )
                                        .expect("Failed to set timeout");
                                }
                            }
                        }
                    });
                }
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
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            
            if let Some(task_elem) = document.get_element_by_id("task-input") {
                if let Some(task_textarea) = task_elem.dyn_ref::<HtmlTextAreaElement>() {
                    let task_text = task_textarea.value();
                    
                    if !task_text.trim().is_empty() {
                        // Get the agent ID
                        APP_STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            if let Some(agent_id) = &state.selected_node_id {
                                let agent_id_clone = agent_id.clone();
                                
                                // Get model and system instructions
                                let _model = state.selected_model.clone();
                                let _system_instructions = state.nodes.get(&agent_id_clone)
                                    .and_then(|node| node.system_instructions.clone())
                                    .unwrap_or_default();
                                
                                // Create a message ID
                                let message_id = state.generate_message_id();
                                
                                // Create a response node
                                if let Some(agent_node) = state.nodes.get_mut(&agent_id_clone) {
                                    // Create a new message for the history
                                    let user_message = crate::models::Message {
                                        role: "user".to_string(),
                                        content: task_text.clone(),
                                        timestamp: js_sys::Date::now() as u64,
                                    };
                                    
                                    // Add to history if it exists
                                    if let Some(history) = &mut agent_node.history {
                                        history.push(user_message);
                                    } else {
                                        agent_node.history = Some(vec![user_message]);
                                    }
                                    
                                    // Update agent status
                                    agent_node.status = Some("processing".to_string());
                                }
                                
                                // Adjust viewport to fit all nodes
                                if state.auto_fit {
                                    state.fit_nodes_to_view();
                                }
                                
                                // Add a response node (this creates a visual node for the response)
                                let response_node_id = state.add_response_node(&agent_id_clone, "Processing...".to_string());
                                
                                // Track the message ID to node ID mapping
                                state.track_message(message_id.clone(), response_node_id);
                                
                                // Save state
                                let _ = state.save_if_modified();
                                
                                // Release the mutable borrow before making network requests
                                drop(state);
                                
                                // Send to backend (use the network module's implementation)
                                crate::network::send_text_to_backend(&task_text, message_id);
                                
                                // Clear the task input field
                                task_textarea.set_value("");
                                
                                // Show a success message
                                web_sys::console::log_1(&"Task sent to agent".into());
                                
                                // Close the modal after sending
                                if let Some(modal) = document.get_element_by_id("agent-modal") {
                                    modal.set_attribute("style", "display: none;").unwrap_or_else(|_| {
                                        web_sys::console::error_1(&"Failed to close modal".into());
                                    });
                                }
                            }
                        });
                    }
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
        
        // Save system instructions function
        let save_function = Closure::once_into_js(move || {
            // Save to APP_STATE
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                if let Some(id) = &state.selected_node_id {
                    let id_clone = id.clone();
                    if let Some(node) = state.nodes.get_mut(&id_clone) {
                        node.system_instructions = Some(system_instructions.clone());
                        state.state_modified = true;
                        let _ = state.save_if_modified();
                    }
                }
            });
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
    
    Ok(())
}

// Set up tab switching for the modal
fn setup_tab_handlers(document: &Document) -> Result<(), JsValue> {
    // Get tab buttons
    let system_tab = document.get_element_by_id("system-tab")
        .ok_or_else(|| JsValue::from_str("System tab not found"))?;
    let task_tab = document.get_element_by_id("task-tab")
        .ok_or_else(|| JsValue::from_str("Task tab not found"))?;
    let history_tab = document.get_element_by_id("history-tab")
        .ok_or_else(|| JsValue::from_str("History tab not found"))?;
    
    // System tab click handler
    let system_click = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Hide all content
        if let Some(system_content) = document.get_element_by_id("system-content") {
            system_content.set_attribute("style", "display: block;").unwrap();
        }
        if let Some(task_content) = document.get_element_by_id("task-content") {
            task_content.set_attribute("style", "display: none;").unwrap();
        }
        if let Some(history_content) = document.get_element_by_id("history-content") {
            history_content.set_attribute("style", "display: none;").unwrap();
        }
        
        // Update tab classes
        if let Some(system_tab) = document.get_element_by_id("system-tab") {
            system_tab.set_class_name("tab-button active");
        }
        if let Some(task_tab) = document.get_element_by_id("task-tab") {
            task_tab.set_class_name("tab-button");
        }
        if let Some(history_tab) = document.get_element_by_id("history-tab") {
            history_tab.set_class_name("tab-button");
        }
    }) as Box<dyn FnMut(_)>);
    
    system_tab.add_event_listener_with_callback("click", system_click.as_ref().unchecked_ref())?;
    system_click.forget();
    
    // Task tab click handler
    let task_click = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Hide all content
        if let Some(system_content) = document.get_element_by_id("system-content") {
            system_content.set_attribute("style", "display: none;").unwrap();
        }
        if let Some(task_content) = document.get_element_by_id("task-content") {
            task_content.set_attribute("style", "display: block;").unwrap();
        }
        if let Some(history_content) = document.get_element_by_id("history-content") {
            history_content.set_attribute("style", "display: none;").unwrap();
        }
        
        // Update tab classes
        if let Some(system_tab) = document.get_element_by_id("system-tab") {
            system_tab.set_class_name("tab-button");
        }
        if let Some(task_tab) = document.get_element_by_id("task-tab") {
            task_tab.set_class_name("tab-button active");
        }
        if let Some(history_tab) = document.get_element_by_id("history-tab") {
            history_tab.set_class_name("tab-button");
        }
    }) as Box<dyn FnMut(_)>);
    
    task_tab.add_event_listener_with_callback("click", task_click.as_ref().unchecked_ref())?;
    task_click.forget();
    
    // History tab click handler
    let history_click = Closure::wrap(Box::new(move |_event: Event| {
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        // Hide all content
        if let Some(system_content) = document.get_element_by_id("system-content") {
            system_content.set_attribute("style", "display: none;").unwrap();
        }
        if let Some(task_content) = document.get_element_by_id("task-content") {
            task_content.set_attribute("style", "display: none;").unwrap();
        }
        if let Some(history_content) = document.get_element_by_id("history-content") {
            history_content.set_attribute("style", "display: block;").unwrap();
        }
        
        // Update tab classes
        if let Some(system_tab) = document.get_element_by_id("system-tab") {
            system_tab.set_class_name("tab-button");
        }
        if let Some(task_tab) = document.get_element_by_id("task-tab") {
            task_tab.set_class_name("tab-button");
        }
        if let Some(history_tab) = document.get_element_by_id("history-tab") {
            history_tab.set_class_name("tab-button active");
        }
    }) as Box<dyn FnMut(_)>);
    
    history_tab.add_event_listener_with_callback("click", history_click.as_ref().unchecked_ref())?;
    history_click.forget();
    
    Ok(())
}

// Create Agent button handler
pub fn setup_create_agent_button_handler(document: &Document) -> Result<(), JsValue> {
    let create_agent_button = document.get_element_by_id("create-agent-button")
        .ok_or_else(|| JsValue::from_str("Create agent button not found"))?;
    
    let click_callback = Closure::wrap(Box::new(move |_event: MouseEvent| {
        // Create a new agent node
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Get viewport center coordinates
            let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
            let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
            
            let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - 75.0; // Center - half node width
            let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - 50.0; // Center - half node height
            
            // Create a new agent node
            let _node_id = state.add_node(
                "New Agent".to_string(),
                x,
                y,
                NodeType::AgentIdentity
            );
            
            // Draw the nodes
            state.draw_nodes();
            
            // Save state after adding the node
            if let Err(e) = state.save_if_modified() {
                web_sys::console::error_1(&format!("Failed to save state: {:?}", e).into());
            }
            
            web_sys::console::log_1(&"Created new agent node".into());
        });
    }) as Box<dyn FnMut(_)>);
    
    create_agent_button.add_event_listener_with_callback(
        "click", 
        click_callback.as_ref().unchecked_ref()
    )?;
    click_callback.forget();
    
    Ok(())
}
