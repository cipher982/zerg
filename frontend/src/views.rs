// frontend/src/views.rs
//
// This file contains functions to render different parts of the UI
// based on the current application state.
//
use web_sys::Document;
use crate::state::AppState;
use crate::storage::ActiveView;
use wasm_bindgen::JsValue;
use wasm_bindgen::JsCast;
use crate::constants::{DEFAULT_SYSTEM_INSTRUCTIONS, DEFAULT_TASK_INSTRUCTIONS};


// Handle agent modal display
pub fn hide_agent_modal(document: &Document) -> Result<(), JsValue> {
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        modal.set_attribute("style", "display: none;")?;
    }
    
    Ok(())
}

// Add a function to display the agent modal
pub fn show_agent_modal(state: &AppState, document: &Document) -> Result<(), JsValue> {
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        modal.set_attribute("style", "display: block;")?;
        
        // Update modal title with agent name if an agent is selected
        if let Some(node_id) = &state.selected_node_id {
            // Try to get the node data
            let node_data = state.nodes.get(node_id);
            
            // Variables to store agent data
            let mut agent_name = String::new();
            let mut system_instructions = String::new();
            let mut task_instructions = String::new();
            
            // First try to get data directly from the node
            if let Some(node) = node_data {
                agent_name = node.text.clone();
                
                if let Some(sys_instr) = node.system_instructions() {
                    system_instructions = sys_instr;
                }
                
                if let Some(task_instr) = node.task_instructions() {
                    task_instructions = task_instr;
                }
                
                // If node has an agent_id, try to get more data from that agent
                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get(&agent_id) {
                        // If we don't have a name yet, use the agent's name
                        if agent_name.is_empty() {
                            agent_name = agent.name.clone();
                        }
                        
                        // If we don't have system instructions yet, use the agent's
                        if system_instructions.is_empty() {
                            if let Some(sys_instr) = &agent.system_instructions {
                                system_instructions = sys_instr.clone();
                            }
                        }
                    }
                }
            } else {
                // If no node found, see if the node_id looks like "agent-{id}"
                // and try to get the agent data directly
                if let Some(id_str) = node_id.strip_prefix("agent-") {
                    if let Ok(agent_id) = id_str.parse::<u32>() {
                        if let Some(agent) = state.agents.get(&agent_id) {
                            agent_name = agent.name.clone();
                            
                            if let Some(sys_instr) = &agent.system_instructions {
                                system_instructions = sys_instr.clone();
                            }
                        }
                    }
                }
            }
            
            // Set modal data using the combined information
            if let Some(modal_title) = document.get_element_by_id("modal-title") {
                modal_title.set_inner_html(&format!("Agent: {}", agent_name));
            }
            
            // Set agent name in input field
            if let Some(name_elem) = document.get_element_by_id("agent-name") {
                if let Some(name_input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
                    name_input.set_value(&agent_name);
                }
            }
            
            // Set system instructions in textarea
            if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                if let Some(system_textarea) = system_elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                    if system_instructions.trim().is_empty() {
                        system_textarea.set_value(DEFAULT_SYSTEM_INSTRUCTIONS);
                    } else {
                        system_textarea.set_value(&system_instructions);
                    }
                }
            }
            
            // Set task instructions in textarea
            if let Some(task_elem) = document.get_element_by_id("default-task-instructions") {
                if let Some(task_textarea) = task_elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                    if task_instructions.trim().is_empty() {
                        task_textarea.set_value(DEFAULT_TASK_INSTRUCTIONS);
                    } else {
                        task_textarea.set_value(&task_instructions);
                    }
                }
            }
            
            // Store the original node_id for when we save changes
            modal.set_attribute("data-node-id", node_id)?;
        }
    }
    
    Ok(())
}

// Render the appropriate view based on the explicit view type
// This avoids requiring a reference to AppState, preventing potential borrow issues
pub fn render_active_view_by_type(view_type: &ActiveView, document: &Document) -> Result<(), JsValue> {
    // First log which view we're switching to for debugging
    web_sys::console::log_1(&format!("Switching to view: {:?}", view_type).into());
    
    // Start fresh by unmounting ALL views first
    web_sys::console::log_1(&"Unmounting all views".into());
    
    // Unmount dashboard if it's mounted
    if crate::pages::dashboard::is_dashboard_mounted(document) {
        web_sys::console::log_1(&"Unmounting dashboard".into());
        crate::pages::dashboard::unmount_dashboard(document)?;
    }
    
    // Unmount canvas if it's mounted
    if crate::pages::canvas::is_canvas_mounted(document) {
        web_sys::console::log_1(&"Unmounting canvas".into());
        crate::pages::canvas::unmount_canvas(document)?;
    }
    
    // Hide chat view if it exists
    if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
        web_sys::console::log_1(&"Hiding chat view".into());
        chat_container.set_attribute("style", "display: none;")?;
    }
    
    // Now mount the requested view
    match view_type {
        ActiveView::Dashboard => {
            web_sys::console::log_1(&"Mounting dashboard view".into());
            crate::pages::dashboard::mount_dashboard(document)?;
        },
        ActiveView::Canvas => {
            web_sys::console::log_1(&"Mounting canvas view".into());
            crate::pages::canvas::mount_canvas(document)?;
        },
        ActiveView::ChatView => {
            web_sys::console::log_1(&"Mounting chat view".into());
            // Setup the chat view if needed
            crate::components::chat_view::setup_chat_view(document)?;
            
            // Show chat view container
            if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
                chat_container.set_attribute("style", "display: flex;")?;
            }
        }
    }
    
    // Update tab styling based on active view
    update_tab_styling(view_type, document)?;
    
    Ok(())
}

// Helper function to update tab styling based on active view
fn update_tab_styling(view_type: &ActiveView, document: &Document) -> Result<(), JsValue> {
    // Reset all tabs to inactive state first
    if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
        dashboard_tab.set_class_name("tab-button");
    }
    
    if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
        canvas_tab.set_class_name("tab-button");
    }
    
    // Now set the active tab based on the current view
    match view_type {
        ActiveView::Dashboard => {
            if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
                dashboard_tab.set_class_name("tab-button active");
            }
        },
        ActiveView::Canvas => {
            if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
                canvas_tab.set_class_name("tab-button active");
            }
        },
        ActiveView::ChatView => {
            // Chat doesn't have a tab currently, could add one in the future
        }
    }
    
    Ok(())
}