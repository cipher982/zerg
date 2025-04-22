use wasm_bindgen::prelude::*;
use web_sys::Document;

pub fn create_base_ui(document: &Document) -> Result<(), JsValue> {
    // Create header
    let header = document.create_element("div")?;
    header.set_class_name("header");
    header.set_attribute("id", "header")?;
    
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
    
    // Create tabs - now just "Main" and "History"
    let tab_container = document.create_element("div")?;
    tab_container.set_class_name("tab-container");
    
    let main_tab = document.create_element("button")?;
    main_tab.set_class_name("tab-button active");
    main_tab.set_id("main-tab");
    main_tab.set_inner_html("Main");
    
    let history_tab = document.create_element("button")?;
    history_tab.set_class_name("tab-button");
    history_tab.set_id("history-tab");
    history_tab.set_inner_html("History");
    
    tab_container.append_child(&main_tab)?;
    tab_container.append_child(&history_tab)?;
    
    // Create main content (combining name, system instructions, and task)
    let main_content = document.create_element("div")?;
    main_content.set_class_name("tab-content");
    main_content.set_id("main-content");
    
    // Add agent name field
    let name_label = document.create_element("label")?;
    name_label.set_inner_html("Agent Name:");
    name_label.set_attribute("for", "agent-name")?;
    
    let name_input = document.create_element("input")?;
    name_input.set_id("agent-name");
    name_input.set_attribute("type", "text")?;
    name_input.set_attribute("placeholder", "Enter agent name...")?;
    
    // Add system instructions field
    let system_label = document.create_element("label")?;
    system_label.set_inner_html("System Instructions:");
    system_label.set_attribute("for", "system-instructions")?;
    
    let system_textarea = document.create_element("textarea")?;
    system_textarea.set_id("system-instructions");
    system_textarea.set_attribute("rows", "6")?;
    system_textarea.set_attribute("placeholder", "Enter system-level instructions for this agent...")?;
    
    // Add default task instructions field
    let default_task_label = document.create_element("label")?;
    default_task_label.set_inner_html("Task Instructions:");
    default_task_label.set_attribute("for", "default-task-instructions")?;
    
    let default_task_textarea = document.create_element("textarea")?;
    default_task_textarea.set_id("default-task-instructions");
    default_task_textarea.set_attribute("rows", "4")?;
    default_task_textarea.set_attribute("placeholder", "Optional task instructions that will be used when running this agent. If empty, a default 'begin' prompt will be used.")?;
    
    // Add all fields to main content
    main_content.append_child(&name_label)?;
    main_content.append_child(&name_input)?;
    main_content.append_child(&system_label)?;
    main_content.append_child(&system_textarea)?;
    main_content.append_child(&default_task_label)?;
    main_content.append_child(&default_task_textarea)?;

    // ------------------------------------------------------------------
    // Scheduling controls (Cron + toggle)
    // ------------------------------------------------------------------

    // Schedule (Cron) label & input
    let schedule_label = document.create_element("label")?;
    schedule_label.set_inner_html("Schedule (Cron expression):");
    schedule_label.set_attribute("for", "agent-schedule")?;

    let schedule_input = document.create_element("input")?;
    schedule_input.set_id("agent-schedule");
    schedule_input.set_attribute("type", "text")?;
    schedule_input.set_attribute("placeholder", "*/15 * * * * (min hour day month weekday)")?;
    
    // Add help text for cron format
    let schedule_help = document.create_element("div")?;
    schedule_help.set_class_name("help-text");
    schedule_help.set_inner_html("Format: minute(0-59) hour(0-23) day(1-31) month(1-12) weekday(0-6). Example: */15 * * * * runs every 15 minutes.");
    schedule_help.set_attribute("style", "font-size: 0.6em; color: #666; margin-bottom: 10px;")?;

    // Enable schedule toggle
    let enable_label = document.create_element("label")?;
    enable_label.set_inner_html("Enable schedule:");

    let enable_checkbox = document.create_element("input")?;
    enable_checkbox.set_id("agent-run-on-schedule");
    enable_checkbox.set_attribute("type", "checkbox")?;

    // Append scheduling controls
    main_content.append_child(&schedule_label)?;
    main_content.append_child(&schedule_input)?;
    main_content.append_child(&schedule_help)?;
    main_content.append_child(&enable_label)?;
    main_content.append_child(&enable_checkbox)?;
    
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
    
    button_container.append_child(&save_button)?;
    
    // Add all elements to the document
    modal_content.append_child(&modal_header)?;
    modal_content.append_child(&tab_container)?;
    modal_content.append_child(&main_content)?;
    modal_content.append_child(&history_content)?;
    modal_content.append_child(&button_container)?;
    
    modal.append_child(&modal_content)?;
    
    // Add modal to the document body
    let body = document.body().ok_or(JsValue::from_str("No body found"))?;
    body.append_child(&modal)?;
    
    // Event handlers will be set up separately
    
    Ok(())
}
