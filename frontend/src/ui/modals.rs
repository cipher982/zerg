use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlInputElement, HtmlTextAreaElement};
use wasm_bindgen::JsCast;
use crate::state::APP_STATE;

/// Opens the agent modal with the data from the specified node
pub fn open_agent_modal(document: &Document, node_id: &str) -> Result<(), JsValue> {
    // Get agent data first
    let (node_text, system_instructions, history_data) = APP_STATE.with(|state| {
        let state = state.borrow();
        
        if let Some(node) = state.nodes.get(node_id) {
            (
                node.text.clone(),
                node.system_instructions.clone().unwrap_or_default(),
                node.history.clone().unwrap_or_default()
            )
        } else {
            (String::new(), String::new(), Vec::new())
        }
    });
    
    // Now update modal without holding the borrow
    // Set modal title
    if let Some(modal_title) = document.get_element_by_id("modal-title") {
        modal_title.set_inner_html(&format!("Agent: {}", node_text));
    }
    
    // Set agent name in the input field
    if let Some(name_elem) = document.get_element_by_id("agent-name") {
        if let Some(name_input) = name_elem.dyn_ref::<HtmlInputElement>() {
            name_input.set_value(&node_text);
        }
    }
    
    // Load system instructions
    if let Some(system_elem) = document.get_element_by_id("system-instructions") {
        if let Some(system_textarea) = system_elem.dyn_ref::<HtmlTextAreaElement>() {
            system_textarea.set_value(&system_instructions);
        }
    }
    
    // Load conversation history
    if let Some(history_container) = document.get_element_by_id("history-container") {
        if history_data.is_empty() {
            history_container.set_inner_html("<p>No history available.</p>");
        } else {
            // Clear existing history
            history_container.set_inner_html("");
            
            // Add each message to the history container
            for message in history_data {
                if let Ok(message_elem) = document.create_element("div") {
                    message_elem.set_class_name(&format!("history-item {}", message.role));
                    
                    if let Ok(content) = document.create_element("p") {
                        content.set_inner_html(&message.content);
                        
                        let _ = message_elem.append_child(&content);
                        let _ = history_container.append_child(&message_elem);
                    }
                }
            }
        }
    }
    
    // Show the modal
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        let _ = modal.set_attribute("style", "display: block;");
    }
    
    Ok(())
} 