use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};
use crate::state::APP_STATE;
use crate::models::NodeType;

// Agent status for displaying in the dashboard
#[derive(Clone, Debug, PartialEq)]
pub enum AgentStatus {
    Active,
    Idle,
    Error,
    Scheduled,
}

// Agent data structure for the dashboard
#[derive(Clone, Debug)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub success_rate: f64,
    pub run_count: u32,
}

impl Agent {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            status: AgentStatus::Idle,
            last_run: None,
            next_run: None,
            success_rate: 0.0,
            run_count: 0,
        }
    }
}

// Main function to render the dashboard
pub fn render_dashboard(document: &Document, container: &Element) -> Result<(), JsValue> {
    // Clear the container
    container.set_inner_html("");
    
    // Create header area with search and create button
    let header = create_dashboard_header(document)?;
    container.append_child(&header)?;
    
    // Create the table
    let table = create_agents_table(document)?;
    container.append_child(&table)?;
    
    // Populate the table with data
    populate_agents_table(document)?;
    
    Ok(())
}

// Create the dashboard header with search and create button
fn create_dashboard_header(document: &Document) -> Result<Element, JsValue> {
    let header = document.create_element("div")?;
    header.set_class_name("dashboard-header");
    
    // Search box
    let search_container = document.create_element("div")?;
    search_container.set_class_name("search-container");
    
    let search_icon = document.create_element("span")?;
    search_icon.set_class_name("search-icon");
    search_icon.set_inner_html("🔍");
    search_container.append_child(&search_icon)?;
    
    let search_input = document.create_element("input")?;
    search_input.set_id("agent-search");
    search_input.dyn_ref::<web_sys::HtmlInputElement>().unwrap().set_placeholder("Search agents...");
    
    // Add event listener for searching
    let search_callback = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let target = event.target().unwrap();
        let input = target.dyn_ref::<web_sys::HtmlInputElement>().unwrap();
        let search_term = input.value();
        web_sys::console::log_1(&format!("Search term: {}", search_term).into());
        // Implement search filtering
    }) as Box<dyn FnMut(_)>);
    
    search_input.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("input", search_callback.as_ref().unchecked_ref())?;
    search_callback.forget();
    
    search_container.append_child(&search_input)?;
    header.append_child(&search_container)?;
    
    // Create new agent button
    let create_btn = document.create_element("button")?;
    create_btn.set_id("create-agent-btn");
    create_btn.set_class_name("create-agent-btn");
    create_btn.set_inner_html("+ Create New Agent");
    
    let create_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&"Create new agent from dashboard".into());
        
        // Create a new agent in the app state
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Get viewport center coordinates for node placement
            let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
            let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
            
            let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - 75.0;
            let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - 50.0;
            
            // Create a new agent node
            let node_id = state.add_node(
                "New Agent".to_string(),
                x,
                y,
                NodeType::AgentIdentity
            );
            
            web_sys::console::log_1(&format!("Created new agent with ID: {}", node_id).into());
            
            // Draw the nodes on canvas too
            state.draw_nodes();
            
            // Save state
            state.state_modified = true;
            if let Err(e) = state.save_if_modified() {
                web_sys::console::warn_1(&format!("Failed to save new agent: {:?}", e).into());
            }
        });
        
        // Refresh the dashboard immediately
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        if let Some(container) = document.get_element_by_id("dashboard-container") {
            match render_dashboard(&document, &container) {
                Ok(_) => web_sys::console::log_1(&"Dashboard refreshed".into()),
                Err(e) => web_sys::console::error_1(&format!("Failed to refresh dashboard: {:?}", e).into()),
            }
        } else {
            web_sys::console::warn_1(&"Could not find dashboard container to refresh".into());
        }
    }) as Box<dyn FnMut(_)>);
    
    create_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", create_callback.as_ref().unchecked_ref())?;
    create_callback.forget();
    
    header.append_child(&create_btn)?;
    
    Ok(header)
}

// Create the agents table structure
fn create_agents_table(document: &Document) -> Result<Element, JsValue> {
    let table = document.create_element("table")?;
    table.set_id("agents-table");
    table.set_class_name("agents-table");
    
    // Create table header
    let thead = document.create_element("thead")?;
    let header_row = document.create_element("tr")?;
    
    // Define column headers
    let columns = vec![
        "Name", "Status", "Last Run", "Next Run", "Success Rate", "Actions"
    ];
    
    for &column in columns.iter() {
        let th = document.create_element("th")?;
        th.set_inner_html(column);
        
        // Add sort functionality
        let column_id = column.to_lowercase().replace(" ", "_");
        th.set_attribute("data-column", &column_id)?;
        
        // Add click handler for sorting
        let sort_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            web_sys::console::log_1(&format!("Sort by: {}", column_id).into());
            // Implement sorting logic
        }) as Box<dyn FnMut(_)>);
        
        th.dyn_ref::<HtmlElement>()
            .unwrap()
            .add_event_listener_with_callback("click", sort_callback.as_ref().unchecked_ref())?;
        
        // We need to forget the closure to avoid it being cleaned up when leaving scope
        sort_callback.forget();
        
        header_row.append_child(&th)?;
    }
    
    thead.append_child(&header_row)?;
    table.append_child(&thead)?;
    
    // Create table body
    let tbody = document.create_element("tbody")?;
    tbody.set_id("agents-table-body");
    table.append_child(&tbody)?;
    
    Ok(table)
}

// Populate the table with agent data
fn populate_agents_table(document: &Document) -> Result<(), JsValue> {
    let tbody = document.get_element_by_id("agents-table-body")
        .ok_or_else(|| JsValue::from_str("Could not find agents-table-body"))?;
    
    // Clear existing rows
    tbody.set_inner_html("");
    
    // Get agents from the app state
    let agents = get_agents_from_app_state();
    
    if agents.is_empty() {
        // Display a "No agents found" message
        let empty_row = document.create_element("tr")?;
        let empty_cell = document.create_element("td")?;
        empty_cell.set_attribute("colspan", "6")?;
        empty_cell.set_inner_html("No agents found. Click '+ Create New Agent' to get started.");
        empty_cell.set_attribute("style", "text-align: center; padding: 30px; color: #888;")?;
        
        empty_row.append_child(&empty_cell)?;
        tbody.append_child(&empty_row)?;
    } else {
        // Create a row for each agent
        for agent in agents {
            let row = create_agent_row(document, &agent)?;
            tbody.append_child(&row)?;
        }
    }
    
    Ok(())
}

// Create a table row for an agent
fn create_agent_row(document: &Document, agent: &Agent) -> Result<Element, JsValue> {
    let row = document.create_element("tr")?;
    row.set_attribute("data-agent-id", &agent.id)?;
    
    // Name cell
    let name_cell = document.create_element("td")?;
    name_cell.set_inner_html(&agent.name);
    row.append_child(&name_cell)?;
    
    // Status cell
    let status_cell = document.create_element("td")?;
    let status_indicator = document.create_element("span")?;
    status_indicator.set_class_name(&format!("status-indicator status-{:?}", agent.status).to_lowercase());
    
    let status_text = match agent.status {
        AgentStatus::Active => "● Active",
        AgentStatus::Idle => "○ Idle",
        AgentStatus::Error => "⚠ Error",
        AgentStatus::Scheduled => "⏱ Scheduled",
    };
    
    status_indicator.set_inner_html(status_text);
    status_cell.append_child(&status_indicator)?;
    row.append_child(&status_cell)?;
    
    // Last Run cell
    let last_run_cell = document.create_element("td")?;
    last_run_cell.set_inner_html(&agent.last_run.as_deref().unwrap_or("-"));
    row.append_child(&last_run_cell)?;
    
    // Next Run cell
    let next_run_cell = document.create_element("td")?;
    next_run_cell.set_inner_html(&agent.next_run.as_deref().unwrap_or("-"));
    row.append_child(&next_run_cell)?;
    
    // Success Rate cell
    let success_cell = document.create_element("td")?;
    success_cell.set_inner_html(&format!("{:.1}% ({})", agent.success_rate, agent.run_count));
    row.append_child(&success_cell)?;
    
    // Actions cell
    let actions_cell = document.create_element("td")?;
    actions_cell.set_class_name("actions-cell");
    
    // Run button
    let run_btn = document.create_element("button")?;
    run_btn.set_class_name("action-btn run-btn");
    run_btn.set_inner_html("▶");
    run_btn.set_attribute("title", "Run Agent")?;
    
    // Run button click handler
    let agent_id = agent.id.clone();
    let run_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&format!("Run agent: {}", agent_id).into());
        // Update the agent status in app state
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            if let Some(node) = state.nodes.get_mut(&agent_id) {
                if node.node_type == NodeType::AgentIdentity {
                    node.status = Some("processing".to_string());
                    state.state_modified = true;
                    
                    // Save the state
                    if let Err(e) = state.save_if_modified() {
                        web_sys::console::warn_1(&format!("Failed to save agent state: {:?}", e).into());
                    }
                }
            }
        });
        
        // Refresh the dashboard
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        if let Some(container) = document.get_element_by_id("dashboard-container") {
            let _ = render_dashboard(&document, &container);
        }
    }) as Box<dyn FnMut(_)>);
    
    run_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", run_callback.as_ref().unchecked_ref())?;
    run_callback.forget();
    
    actions_cell.append_child(&run_btn)?;
    
    // Edit button
    let edit_btn = document.create_element("button")?;
    edit_btn.set_class_name("action-btn edit-btn");
    edit_btn.set_inner_html("✎");
    edit_btn.set_attribute("title", "Edit Agent")?;
    
    // Edit button click handler
    let agent_id = agent.id.clone();
    let edit_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&format!("Edit agent: {}", agent_id).into());
        
        // Set the selected node ID
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.selected_node_id = Some(agent_id.clone());
        });
        
        // Open the agent modal
        let window = web_sys::window().expect("no global window exists");
        let document = window.document().expect("should have a document");
        
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            // Get agent data
            let (node_text, system_instructions) = APP_STATE.with(|state| {
                let state = state.borrow();
                
                if let Some(node) = state.nodes.get(&agent_id) {
                    (
                        node.text.clone(),
                        node.system_instructions.clone().unwrap_or_default(),
                    )
                } else {
                    (String::new(), String::new())
                }
            });
            
            // Set modal title
            if let Some(modal_title) = document.get_element_by_id("modal-title") {
                modal_title.set_inner_html(&format!("Agent: {}", node_text));
            }
            
            // Set agent name in the input field
            if let Some(name_elem) = document.get_element_by_id("agent-name") {
                if let Some(name_input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
                    name_input.set_value(&node_text);
                }
            }
            
            // Load system instructions
            if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                if let Some(system_textarea) = system_elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                    system_textarea.set_value(&system_instructions);
                }
            }
            
            // Show the modal
            if let Err(e) = modal.set_attribute("style", "display: block;") {
                web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    edit_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", edit_callback.as_ref().unchecked_ref())?;
    edit_callback.forget();
    
    actions_cell.append_child(&edit_btn)?;
    
    // More options button
    let more_btn = document.create_element("button")?;
    more_btn.set_class_name("action-btn more-btn");
    more_btn.set_inner_html("⋮");
    more_btn.set_attribute("title", "More Options")?;
    
    // More button click handler
    let agent_id = agent.id.clone();
    let more_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&format!("More options for agent: {}", agent_id).into());
        // Show dropdown menu
    }) as Box<dyn FnMut(_)>);
    
    more_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", more_callback.as_ref().unchecked_ref())?;
    more_callback.forget();
    
    actions_cell.append_child(&more_btn)?;
    
    row.append_child(&actions_cell)?;
    
    Ok(row)
}

// Get agents from the application state
fn get_agents_from_app_state() -> Vec<Agent> {
    let mut agents = Vec::new();
    
    APP_STATE.with(|state| {
        let state = state.borrow();
        
        // Loop through all nodes in the state
        for (id, node) in &state.nodes {
            // Only include AgentIdentity nodes
            if node.node_type == NodeType::AgentIdentity {
                // Convert node status to AgentStatus
                let status = match node.status.as_deref() {
                    Some("processing") => AgentStatus::Active,
                    Some("error") => AgentStatus::Error,
                    Some("scheduled") => AgentStatus::Scheduled,
                    _ => AgentStatus::Idle, // Default to idle
                };
                
                // Get last run from history if available
                let last_run = if let Some(history) = &node.history {
                    if !history.is_empty() {
                        // Format timestamp to human-readable format
                        let timestamp = history.last().unwrap().timestamp;
                        let date = js_sys::Date::new(&JsValue::from_f64(timestamp as f64));
                        Some(format!(
                            "{:02}/{:02} {:02}:{:02}",
                            date.get_month() + 1, // JS months are 0-indexed
                            date.get_date(),
                            date.get_hours(),
                            date.get_minutes()
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Calculate success rate
                let (success_rate, run_count) = if let Some(history) = &node.history {
                    let total = history.len();
                    // For demonstration, assume most messages are successful
                    let successes = total.saturating_sub(total / 10); // 90% success rate as a demo
                    
                    if total > 0 {
                        ((successes as f64 / total as f64) * 100.0, total as u32)
                    } else {
                        (0.0, 0)
                    }
                } else {
                    (0.0, 0)
                };
                
                // Create agent object
                let agent = Agent {
                    id: id.clone(),
                    name: node.text.clone(),
                    status,
                    last_run,
                    next_run: None, // Not implemented in the current system
                    success_rate,
                    run_count: run_count as u32,
                };
                
                agents.push(agent);
            }
        }
    });
    
    agents
}

// Function to set up the dashboard in the application
pub fn setup_dashboard(document: &Document) -> Result<(), JsValue> {
    // Check if there's already a dashboard container
    if document.get_element_by_id("dashboard-container").is_none() {
        // Create a container for the dashboard
        let dashboard_container = document.create_element("div")?;
        dashboard_container.set_id("dashboard-container");
        dashboard_container.set_class_name("dashboard-container");
        
        // Get the body element or another appropriate parent
        let app_container = document
            .get_element_by_id("app-container")
            .ok_or(JsValue::from_str("Could not find app-container"))?;
        
        // Append the dashboard container
        app_container.append_child(&dashboard_container)?;
    }
    
    // Get the dashboard container and render the dashboard content
    let dashboard_container = document
        .get_element_by_id("dashboard-container")
        .ok_or(JsValue::from_str("Dashboard container not found"))?;
    
    render_dashboard(document, &dashboard_container)?;
    
    Ok(())
} 