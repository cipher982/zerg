use wasm_bindgen::prelude::*;
use web_sys::Document;

pub fn create_base_ui(document: &Document) -> Result<(), JsValue> {
    // Create header
    let header = document.create_element("div")?;
    header.set_class_name("header");
    
    let title = document.create_element("h1")?;
    title.set_inner_html("AI Agent Platform");
    header.append_child(&title)?;
    
    // Create status bar
    let status_bar = document.create_element("div")?;
    status_bar.set_class_name("status-bar");
    
    let status = document.create_element("div")?;
    status.set_id("status");
    status.set_class_name("yellow"); // Initial state
    status.set_inner_html("Status: Connecting");
    
    let api_status = document.create_element("div")?;
    api_status.set_id("api-status");
    api_status.set_inner_html("API: Ready");
    
    status_bar.append_child(&status)?;
    status_bar.append_child(&api_status)?;
    
    // Get body element
    let body = document.body().ok_or(JsValue::from_str("No body found"))?;
    
    // Append header and status bar
    body.append_child(&header)?;
    body.append_child(&status_bar)?;
    
    Ok(())
}

// Create a modal dialog for agent interaction
pub fn create_agent_input_modal(document: &Document) -> Result<(), JsValue> {
    // Check if modal already exists (avoid duplicates)
    if document.get_element_by_id("agent-modal").is_some() {
        return Ok(());
    }
    
    // Create modal container
    let modal = document.create_element("div")?;
    modal.set_id("agent-modal");
    modal.set_class_name("modal");
    modal.set_attribute("style", "display: none;")?;
    
    // Create modal content
    let modal_content = document.create_element("div")?;
    modal_content.set_class_name("modal-content");
    
    // Create modal header
    let modal_header = document.create_element("div")?;
    modal_header.set_class_name("modal-header");
    
    let modal_title = document.create_element("h2")?;
    modal_title.set_id("modal-title");
    modal_title.set_inner_html("Agent Configuration");
    
    let close_button = document.create_element("span")?;
    close_button.set_class_name("close");
    close_button.set_inner_html("&times;");
    close_button.set_id("modal-close");
    
    modal_header.append_child(&modal_title)?;
    modal_header.append_child(&close_button)?;
    
    // Create tabs
    let tab_container = document.create_element("div")?;
    tab_container.set_class_name("tab-container");
    
    let system_tab = document.create_element("button")?;
    system_tab.set_class_name("tab-button active");
    system_tab.set_id("system-tab");
    system_tab.set_inner_html("System");
    
    let task_tab = document.create_element("button")?;
    task_tab.set_class_name("tab-button");
    task_tab.set_id("task-tab");
    task_tab.set_inner_html("Task");
    
    let history_tab = document.create_element("button")?;
    history_tab.set_class_name("tab-button");
    history_tab.set_id("history-tab");
    history_tab.set_inner_html("History");
    
    tab_container.append_child(&system_tab)?;
    tab_container.append_child(&task_tab)?;
    tab_container.append_child(&history_tab)?;
    
    // Create system instructions section
    let system_content = document.create_element("div")?;
    system_content.set_class_name("tab-content");
    system_content.set_id("system-content");
    
    let system_label = document.create_element("label")?;
    system_label.set_inner_html("System Instructions:");
    system_label.set_attribute("for", "system-instructions")?;
    
    let system_textarea = document.create_element("textarea")?;
    system_textarea.set_id("system-instructions");
    system_textarea.set_attribute("rows", "8")?;
    system_textarea.set_attribute("placeholder", "Enter system-level instructions for this agent...")?;
    
    system_content.append_child(&system_label)?;
    system_content.append_child(&system_textarea)?;
    
    // Create task input section (stub for now)
    let task_content = document.create_element("div")?;
    task_content.set_class_name("tab-content");
    task_content.set_id("task-content");
    task_content.set_attribute("style", "display: none;")?;
    
    // Add task input components
    let task_label = document.create_element("label")?;
    task_label.set_inner_html("Task Input:");
    task_label.set_attribute("for", "task-input")?;
    
    let task_input = document.create_element("textarea")?;
    task_input.set_id("task-input");
    task_input.set_attribute("rows", "6")?;
    task_input.set_attribute("placeholder", "Enter specific task or question for this agent...")?;
    
    task_content.append_child(&task_label)?;
    task_content.append_child(&task_input)?;
    
    // Create history section
    let history_content = document.create_element("div")?;
    history_content.set_class_name("tab-content");
    history_content.set_id("history-content");
    history_content.set_attribute("style", "display: none;")?;
    
    let history_container = document.create_element("div")?;
    history_container.set_id("history-container");
    history_container.set_inner_html("<p>No history available.</p>");
    
    history_content.append_child(&history_container)?;
    
    // Create buttons at the bottom of the modal
    let button_container = document.create_element("div")?;
    button_container.set_class_name("modal-buttons");
    
    let save_button = document.create_element("button")?;
    save_button.set_id("save-agent");
    save_button.set_inner_html("Save");
    
    let send_button = document.create_element("button")?;
    send_button.set_id("send-to-agent");
    send_button.set_inner_html("Send");
    
    button_container.append_child(&save_button)?;
    button_container.append_child(&send_button)?;
    
    // Add all elements to the document
    modal_content.append_child(&modal_header)?;
    modal_content.append_child(&tab_container)?;
    modal_content.append_child(&system_content)?;
    modal_content.append_child(&task_content)?;
    modal_content.append_child(&history_content)?;
    modal_content.append_child(&button_container)?;
    
    modal.append_child(&modal_content)?;
    
    // Add modal to the document body
    let body = document.body().ok_or(JsValue::from_str("No body found"))?;
    body.append_child(&modal)?;
    
    // Event handlers will be set up separately
    
    Ok(())
}
