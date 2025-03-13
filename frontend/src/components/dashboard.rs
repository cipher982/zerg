use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};
use crate::state::APP_STATE;
use crate::models::NodeType;
use crate::constants::DEFAULT_SYSTEM_INSTRUCTIONS;

// Agent status for displaying in the dashboard
#[derive(Clone, Debug, PartialEq)]
pub enum AgentStatus {
    // Agent is currently busy with a task
    Running,
    // Agent exists but isn't processing anything right now
    Idle,
    // Agent is in an error state
    Error,
    // Agent's next run is set for a future time
    Scheduled,
    // Agent is created but intentionally blocked from running
    Paused,
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
    pub last_run_success: Option<bool>,
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
            last_run_success: None,
        }
    }
}

// Main function to render the dashboard
pub fn render_dashboard(document: &Document) -> Result<(), JsValue> {
    let dashboard = document.get_element_by_id("dashboard")
        .ok_or(JsValue::from_str("Could not find dashboard element"))?;
    
    // Clear existing content
    dashboard.set_inner_html("");
    
    // Check loading state
    let is_loading = APP_STATE.with(|state| {
        let state = state.borrow();
        state.is_loading
    });
    
    if is_loading {
        // Show loading indicator
        let loading_div = document.create_element("div")?;
        loading_div.set_class_name("loading-indicator");
        loading_div.set_inner_html("Loading agents...");
        dashboard.append_child(&loading_div)?;
        return Ok(());
    }
    
    // Get agents and render them
    let _agents = get_agents_from_app_state();
    
    // Create header area with search and create button
    let header = create_dashboard_header(document)?;
    dashboard.append_child(&header)?;
    
    // Create the table
    let table = create_agents_table(document)?;
    dashboard.append_child(&table)?;
    
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
        
        // Generate a random agent name
        let agent_name = format!("New Agent {}", (js_sys::Math::random() * 100.0).round());
        
        // Create the agent data payload for the API
        let agent_data = format!(
            r#"{{
                "name": "{}",
                "system_instructions": "You are a helpful AI assistant.",
                "task_instructions": "Respond to user questions accurately and concisely.",
                "model": "gpt-3.5-turbo"
            }}"#,
            agent_name
        );
        
        // Use async block to call the API
        wasm_bindgen_futures::spawn_local(async move {
            web_sys::console::log_1(&"Creating agent in API first".into());
            
            // Make the API call to create the agent
            match crate::network::ApiClient::create_agent(&agent_data).await {
                Ok(response) => {
                    // Parse the response to get the agent ID
                    if let Ok(json) = js_sys::JSON::parse(&response) {
                        if let Some(id) = js_sys::Reflect::get(&json, &"id".into()).ok()
                            .and_then(|v| v.as_f64()) 
                        {
                            let agent_id = id as u32;
                            web_sys::console::log_1(&format!("Successfully created agent with ID: {}", agent_id).into());
                            
                            // Now that we have the ID from API, add to state with the proper node ID format
                            crate::state::dispatch_global_message(crate::messages::Message::CreateAgentWithDetails {
                                name: agent_name,
                                agent_id,
                                system_instructions: "You are a helpful AI assistant.".to_string(),
                                task_instructions: "Respond to user questions accurately and concisely.".to_string(),
                            });
                        }
                    }
                },
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to create agent in API: {:?}", e).into());
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    create_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", create_callback.as_ref().unchecked_ref())?;
    create_callback.forget();
    
    header.append_child(&create_btn)?;
    
    // Create reset database button (development only)
    let reset_btn = document.create_element("button")?;
    reset_btn.set_id("reset-db-btn");
    reset_btn.set_class_name("reset-db-btn");
    reset_btn.set_inner_html("🗑️ Reset DB");
    
    let reset_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        web_sys::console::log_1(&"Reset database requested".into());
        
        if let Some(window) = web_sys::window() {
            // Confirm dialog before proceeding
            if !window.confirm_with_message("WARNING: This will delete ALL agents and data. This action cannot be undone. Proceed?").unwrap_or(false) {
                return;
            }
            
            // Use wasm_bindgen_futures to handle the async API call
            use wasm_bindgen_futures::spawn_local;
            use crate::network::ApiClient;
            
            spawn_local(async move {
                match ApiClient::reset_database().await {
                    Ok(response) => {
                        web_sys::console::log_1(&format!("Database reset successful: {}", response).into());
                        
                        // Force a hard refresh without showing another popup
                        let window = web_sys::window().unwrap();
                        if let Some(document) = window.document() {
                            // First try to immediately refresh the dashboard UI
                            let _ = render_dashboard(&document);
                            
                            // Then force a page reload to ensure everything is fresh
                            // Use a timeout to allow the UI to update first
                            let reload_callback = Closure::wrap(Box::new(move || {
                                // Use the raw JS API to force a hard refresh (no cache)
                                js_sys::eval("window.location.reload(true)").unwrap();
                            }) as Box<dyn FnMut()>);
                            
                            let _ = window.set_timeout_with_callback_and_timeout_and_arguments(
                                reload_callback.as_ref().unchecked_ref(),
                                100, // Short delay to ensure message is seen but not requiring interaction
                                &js_sys::Array::new()
                            );
                            reload_callback.forget();
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error resetting database: {:?}", e).into());
                        window.alert_with_message(&format!("Error resetting database: {:?}", e)).unwrap();
                    }
                }
            });
        }
    }) as Box<dyn FnMut(_)>);
    
    reset_btn.dyn_ref::<HtmlElement>()
        .unwrap()
        .add_event_listener_with_callback("click", reset_callback.as_ref().unchecked_ref())?;
    reset_callback.forget();
    
    header.append_child(&reset_btn)?;
    
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
        
        // Add actions-header class to Actions column
        if column == "Actions" {
            th.set_class_name("actions-header");
        }
        
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
        AgentStatus::Running => "● Running",
        AgentStatus::Idle => "○ Idle",
        AgentStatus::Error => "⚠ Error",
        AgentStatus::Scheduled => "⏱ Scheduled",
        AgentStatus::Paused => "⏸ Paused",
    };
    
    status_indicator.set_inner_html(status_text);
    status_cell.append_child(&status_indicator)?;
    
    // Add last run success/failure indicator if available
    if let Some(success) = agent.last_run_success {
        let last_run_indicator = document.create_element("span")?;
        last_run_indicator.set_class_name(
            if success {
                "last-run-indicator last-run-success"
            } else {
                "last-run-indicator last-run-failure"
            }
        );
        
        last_run_indicator.set_inner_html(
            if success {
                " (Last: ✓)"
            } else {
                " (Last: ✗)"
            }
        );
        
        status_cell.append_child(&last_run_indicator)?;
    }
    
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
        
        // Check if this is an API agent (with agent-{id} format)
        if let Some(api_agent_id_str) = agent_id.strip_prefix("agent-") {
            if let Ok(api_agent_id) = api_agent_id_str.parse::<u32>() {
                // Call the API to run the agent
                wasm_bindgen_futures::spawn_local(async move {
                    match crate::network::ApiClient::run_agent(api_agent_id).await {
                        Ok(_) => {
                            web_sys::console::log_1(&format!("Agent {} running via API", api_agent_id).into());
                        }
                        Err(e) => {
                            web_sys::console::error_1(&format!("Error running agent {}: {:?}", api_agent_id, e).into());
                        }
                    }
                });
            } else {
                web_sys::console::error_1(&format!("Invalid agent ID format: {}", agent_id).into());
            }
        } else {
            // For legacy nodes without the agent- prefix, use the old approach
            // Dispatch SendTaskToAgent message and capture both the need_refresh flag and any pending network call
            let (need_refresh, pending_call) = {
                // Scope the mutable borrow so it's dropped before refresh_ui_after_state_change
                APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    
                    // Set the selected agent first so the message handler knows which agent to run
                    state.selected_node_id = Some(agent_id.clone());
                    
                    // Dispatch the message to run the agent
                    state.dispatch(crate::messages::Message::SendTaskToAgent)
                })
            };
            
            // After borrowing mutably, we can refresh UI if needed in a separate borrow
            if need_refresh {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::warn_1(&format!("Failed to refresh UI: {:?}", e).into());
                }
            }
            
            // Now that we've completely dropped the borrow, we can execute the network call
            if let Some((task_text, message_id)) = pending_call {
                crate::network::send_text_to_backend(&task_text, message_id);
            }
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
            
            // Set system instructions with explicit defaults if empty
            let system_instructions_value = if system_instructions.trim().is_empty() {
                DEFAULT_SYSTEM_INSTRUCTIONS.to_string()
            } else {
                system_instructions
            };
            
            // Load system instructions
            if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                if let Some(system_textarea) = system_elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                    system_textarea.set_value(&system_instructions_value);
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
        
        // Return early if still loading
        if state.is_loading {
            return;
        }
        
        // Get agents from node data
        for (id, node) in &state.nodes {
            if node.node_type == NodeType::AgentIdentity {
                // Get agent status
                let status_str = node.status.clone();
                
                // Get message history from node's history field
                let history = node.history.clone();
                
                agents.push((
                    id.clone(), 
                    node.text.clone(), 
                    status_str, 
                    history
                ));
            }
        }
    });
    
    // Sort by name
    agents.sort_by(|a, b| a.1.cmp(&b.1));
    
    // Convert to Agent objects
    let mut agent_objects = Vec::new();
    
    for (id, name, status_str, history) in agents {
        // Convert node status to AgentStatus
        let status = match status_str.as_deref() {
            Some("processing") => AgentStatus::Running,
            Some("error") => AgentStatus::Error,
            Some("scheduled") => AgentStatus::Scheduled,
            Some("paused") => AgentStatus::Paused,
            Some("idle") => AgentStatus::Idle,
            None => AgentStatus::Idle,
            _ => AgentStatus::Idle, // Default to idle
        };
        
        // Get last run from history if available
        let (last_run, last_run_success) = if let Some(history_vec) = &history {
            if !history_vec.is_empty() {
                // Format timestamp to human-readable format
                let last_message = history_vec.last().unwrap();
                let timestamp = last_message.timestamp;
                let date = js_sys::Date::new(&JsValue::from_f64(timestamp as f64));
                
                let formatted_date = format!(
                    "{:02}/{:02} {:02}:{:02}",
                    date.get_month() + 1, // JS months are 0-indexed
                    date.get_date(),
                    date.get_hours(),
                    date.get_minutes()
                );
                
                // For last run success, check if the message has an error content
                // For now, we'll assume errors contain the word "error" in the content
                let success = !last_message.content.to_lowercase().contains("error");
                
                (Some(formatted_date), Some(success))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };
        
        // Calculate success rate
        let (success_rate, run_count) = if let Some(history_vec) = &history {
            let total = history_vec.len();
            
            if total > 0 {
                // Count successful messages (those without errors)
                // For now we'll use a simple heuristic - messages without "error" in content
                let successes = history_vec.iter()
                    .filter(|msg| !msg.content.to_lowercase().contains("error"))
                    .count();
                
                ((successes as f64 / total as f64) * 100.0, total as u32)
            } else {
                (0.0, 0)
            }
        } else {
            (0.0, 0)
        };
        
        // Create agent object
        let mut agent = Agent::new(id, name);
        
        // Update additional fields
        agent.status = status;
        agent.last_run = last_run;
        agent.next_run = None; // This would need to be populated from actual scheduling data
        agent.success_rate = success_rate;
        agent.run_count = run_count as u32;
        agent.last_run_success = last_run_success;
        
        agent_objects.push(agent);
    }
    
    agent_objects
}

// Function to refresh the dashboard based on the latest state
pub fn refresh_dashboard(document: &Document) -> Result<(), JsValue> {
    // Ensure both container and dashboard elements exist
    if document.get_element_by_id("dashboard-container").is_none() {
        // If container doesn't exist, set up the dashboard from scratch
        setup_dashboard(document)?;
    } else if document.get_element_by_id("dashboard").is_none() {
        // If container exists but dashboard doesn't, create dashboard
        let container = document.get_element_by_id("dashboard-container")
            .ok_or(JsValue::from_str("Dashboard container not found"))?;
        
        let dashboard = document.create_element("div")?;
        dashboard.set_id("dashboard");
        dashboard.set_class_name("dashboard");
        container.append_child(&dashboard)?;
    }
    
    // Now render the dashboard content
    render_dashboard(document)?;
    Ok(())
}

// Function to set up the dashboard in the application
pub fn setup_dashboard(document: &Document) -> Result<(), JsValue> {
    // Check if there's already a dashboard container
    if document.get_element_by_id("dashboard-container").is_none() {
        // Create a container for the dashboard
        let dashboard_container = document.create_element("div")?;
        dashboard_container.set_id("dashboard-container");
        dashboard_container.set_class_name("dashboard-container");
        
        // Create the dashboard element itself
        let dashboard = document.create_element("div")?;
        dashboard.set_id("dashboard");
        dashboard.set_class_name("dashboard");
        dashboard_container.append_child(&dashboard)?;
        
        // Get the body element or another appropriate parent
        let app_container = document
            .get_element_by_id("app-container")
            .ok_or(JsValue::from_str("Could not find app-container"))?;
        
        // Append the dashboard container
        app_container.append_child(&dashboard_container)?;
    } else {
        // If container exists but dashboard doesn't, create dashboard
        let container = document.get_element_by_id("dashboard-container")
            .ok_or(JsValue::from_str("Dashboard container not found"))?;
        
        if document.get_element_by_id("dashboard").is_none() {
            let dashboard = document.create_element("div")?;
            dashboard.set_id("dashboard");
            dashboard.set_class_name("dashboard");
            container.append_child(&dashboard)?;
        }
    }
    
    // Now render the dashboard content
    render_dashboard(document)?;
    
    Ok(())
} 