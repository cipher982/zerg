use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlInputElement, HtmlTextAreaElement};
use wasm_bindgen::JsCast;
use crate::state::APP_STATE;
use crate::constants::{DEFAULT_SYSTEM_INSTRUCTIONS, DEFAULT_TASK_INSTRUCTIONS};

/// Opens the agent modal with the data from the specified node
pub fn open_agent_modal(document: &Document, node_id: &str) -> Result<(), JsValue> {
    // Get agent data first
    let (node_text, system_instructions, task_instructions, history_data) = APP_STATE.with(|state| {
        let state = state.borrow();
        
        if let Some(node) = state.nodes.get(node_id) {
            (
                node.text.clone(),
                node.system_instructions().unwrap_or_default(),
                node.task_instructions().unwrap_or_default(),
                node.history().unwrap_or_default()
            )
        } else {
            (String::new(), String::new(), String::new(), Vec::new())
        }
    });

    // Attempt to fetch schedule metadata from agent (if node linked to agent)
    let (schedule_value, run_on_schedule_flag) = APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(node) = state.nodes.get(node_id) {
            // Try get agent_id either from node or derived from node_id string
            if let Some(agent_id) = node.agent_id
                .or_else(|| node_id.strip_prefix("agent-").and_then(|s| s.parse::<u32>().ok()))
            {
                if let Some(agent) = state.agents.get(&agent_id) {
                    return (
                        agent.schedule.clone().unwrap_or_default(),
                        agent.run_on_schedule.unwrap_or(false),
                    );
                }
            }
        }
        (String::new(), false)
    });
    
    // Store current node ID in a data attribute on the modal
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        modal.set_attribute("data-node-id", node_id)?;
    }
    
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
    
    // Set explicit default values for system instructions if empty
    let system_instructions_value = if system_instructions.trim().is_empty() {
        DEFAULT_SYSTEM_INSTRUCTIONS.to_string()
    } else {
        system_instructions
    };
    
    // Load system instructions with defaults if empty
    if let Some(system_elem) = document.get_element_by_id("system-instructions") {
        if let Some(system_textarea) = system_elem.dyn_ref::<HtmlTextAreaElement>() {
            system_textarea.set_value(&system_instructions_value);
        }
    }
    
    // Set explicit default values for task instructions if empty
    let task_instructions_value = if task_instructions.trim().is_empty() {
        DEFAULT_TASK_INSTRUCTIONS.to_string()
    } else {
        task_instructions
    };
    
    // Load default task instructions with defaults if empty
    if let Some(task_elem) = document.get_element_by_id("default-task-instructions") {
        if let Some(task_textarea) = task_elem.dyn_ref::<HtmlTextAreaElement>() {
            task_textarea.set_value(&task_instructions_value);
        }
    }

    // Set schedule cron input if control exists
    if let Some(schedule_elem) = document.get_element_by_id("agent-schedule") {
        if let Some(schedule_input) = schedule_elem.dyn_ref::<HtmlInputElement>() {
            schedule_input.set_value(&schedule_value);
        }
    }

    // Set run_on_schedule checkbox if control exists
    if let Some(enable_elem) = document.get_element_by_id("agent-run-on-schedule") {
        if let Some(enable_checkbox) = enable_elem.dyn_ref::<HtmlInputElement>() {
            enable_checkbox.set_checked(run_on_schedule_flag);
        }
    }
    
    // Load thread history
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

/// Close the agent modal
pub fn close_agent_modal(document: &Document) -> Result<(), JsValue> {
    // Hide the modal
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        modal.set_attribute("style", "display: none;")?;
    }
    
    Ok(())
} 