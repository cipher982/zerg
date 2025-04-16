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

// Render the appropriate view based on the active view in the state
pub fn render_active_view(state: &mut AppState, document: &Document) -> Result<(), JsValue> {
    match state.active_view {
        ActiveView::Dashboard => render_dashboard_view(state, document)?,
        ActiveView::Canvas => render_canvas_view(state, document)?,
        ActiveView::ChatView => {
            // Hide other views
            if let Some(dashboard) = document.get_element_by_id("dashboard-container") {
                dashboard.set_attribute("style", "display: none;")?;
            }
            
            if let Some(canvas) = document.get_element_by_id("canvas-container") {
                canvas.set_attribute("style", "display: none;")?;
            }
            
            // Setup the chat view if not already done
            crate::components::chat_view::setup_chat_view(document)?;
            
            // Get the agent ID from the current thread
            let agent_id = if let Some(thread_id) = state.current_thread_id {
                if let Some(thread) = state.threads.get(&thread_id) {
                    Some(thread.agent_id)
                } else {
                    None
                }
            } else {
                None
            };
            
            // If we have an agent ID, show the chat view for that agent
            if let Some(agent_id) = agent_id {
                crate::components::chat_view::show_chat_view(document, agent_id)?;
            } else {
                // If no agent ID, go back to dashboard
                state.active_view = ActiveView::Dashboard;
                return render_dashboard_view(state, document);
            }
        }
    }
    
    Ok(())
}

// Render the dashboard view
fn render_dashboard_view(_state: &AppState, document: &Document) -> Result<(), JsValue> {
    // Show dashboard container, hide canvas container
    let dashboard_container = match document.get_element_by_id("dashboard-container") {
        Some(container) => {
            // Show the container
            container.set_attribute("style", "display: block;")?;
            container
        },
        None => {
            // Create the container if it doesn't exist
            let container = document.create_element("div")?;
            container.set_id("dashboard-container");
            container.set_class_name("dashboard-container");
            container.set_attribute("style", "display: block;")?;
            
            // Get the app container and append the dashboard container
            let app_container = document
                .get_element_by_id("app-container")
                .ok_or(JsValue::from_str("Could not find app-container"))?;
            
            app_container.append_child(&container)?;
            container
        }
    };
    
    // Ensure the dashboard element exists within the container
    if document.get_element_by_id("dashboard").is_none() {
        let dashboard = document.create_element("div")?;
        dashboard.set_id("dashboard");
        dashboard.set_class_name("dashboard");
        dashboard_container.append_child(&dashboard)?;
    }
    
    if let Some(canvas) = document.get_element_by_id("canvas-container") {
        canvas.set_attribute("style", "display: none;")?;
    }
    
    if let Some(input_panel) = document.get_element_by_id("input-panel") {
        input_panel.set_attribute("style", "display: none;")?;
    }
    
    // Update tab styling
    if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
        dashboard_tab.set_class_name("tab-button active");
    }
    
    if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
        canvas_tab.set_class_name("tab-button");
    }
    
    // Initialize or refresh the dashboard content
    crate::components::dashboard::refresh_dashboard(document)?;
    
    Ok(())
}

// Render the canvas view
fn render_canvas_view(state: &AppState, document: &Document) -> Result<(), JsValue> {
    // Show canvas container, hide dashboard container
    if let Some(dashboard) = document.get_element_by_id("dashboard-container") {
        dashboard.set_attribute("style", "display: none;")?;
    }
    
    if let Some(canvas) = document.get_element_by_id("canvas-container") {
        canvas.set_attribute("style", "display: block;")?;
    }
    
    if let Some(input_panel) = document.get_element_by_id("input-panel") {
        input_panel.set_attribute("style", "display: block;")?;
    }
    
    // Update tab styling
    if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
        dashboard_tab.set_class_name("tab-button");
    }
    
    if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
        canvas_tab.set_class_name("tab-button active");
    }
    
    // Trigger canvas resize to ensure it renders correctly
    if let Some(canvas_elem) = document.get_element_by_id("node-canvas") {
        if let Ok(canvas) = canvas_elem.dyn_into::<web_sys::HtmlCanvasElement>() {
            let _ = crate::components::canvas_editor::resize_canvas(&canvas, 
                crate::components::canvas_editor::AppStateRef::Immutable(state));
        }
    }
    
    // Draw the nodes on the canvas
    if let (Some(canvas_elem), Some(context)) = (&state.canvas, &state.context) {
        // Clear the canvas
        let canvas_width = canvas_elem.width() as f64;
        let canvas_height = canvas_elem.height() as f64;
        context.clear_rect(0.0, 0.0, canvas_width, canvas_height);
        
        // Apply viewport transformations
        context.save();
        context.translate(state.viewport_x, state.viewport_y)?;
        context.scale(state.zoom_level, state.zoom_level)?;
        
        // Draw all nodes
        for (_, node) in &state.nodes {
            crate::canvas::renderer::draw_node(context, node, &state.agents);
        }
        
        // Restore the context
        context.restore();
    }
    
    Ok(())
}

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
    web_sys::console::log_1(&format!("render_active_view_by_type called for view: {:?}", view_type).into());
    // First hide all view containers
    let containers = ["dashboard-container", "canvas-container", "chat-view-container"];
    for container_id in containers.iter() {
        if let Some(container) = document.get_element_by_id(container_id) {
            container.set_attribute("style", "display: none;")?;
            web_sys::console::log_1(&format!("Hidden container: {}", container_id).into());
        } else {
            web_sys::console::warn_1(&format!("Container not found: {}", container_id).into());
        }
    }

    match view_type {
        ActiveView::Dashboard => {
            web_sys::console::log_1(&"Rendering Dashboard view".into());
            // Create dashboard container if needed
            if document.get_element_by_id("dashboard-container").is_none() {
                web_sys::console::log_1(&"Creating new dashboard container".into());
                let dashboard_container = document.create_element("div")?;
                dashboard_container.set_id("dashboard-container");
                dashboard_container.set_class_name("dashboard-container");
                dashboard_container.set_attribute("style", "display: block;")?;
                
                let app_container = document
                    .get_element_by_id("app-container")
                    .ok_or(JsValue::from_str("Could not find app-container"))?;
                
                app_container.append_child(&dashboard_container)?;
                
                // Create dashboard element
                let dashboard = document.create_element("div")?;
                dashboard.set_id("dashboard");
                dashboard.set_class_name("dashboard");
                dashboard_container.append_child(&dashboard)?;
            }
            render_dashboard_view(&AppState::new(), document)?
        },
        ActiveView::Canvas => {
            web_sys::console::log_1(&"Rendering Canvas view".into());
            // For canvas, we'll just toggle the visibility without relying on state
            // Create canvas container if needed
            if document.get_element_by_id("canvas-container").is_none() {
                web_sys::console::log_1(&"Creating new canvas container".into());
                let canvas_container = document.create_element("div")?;
                canvas_container.set_id("canvas-container");
                canvas_container.set_class_name("canvas-container");
                canvas_container.set_attribute("style", "display: block;")?;
                
                let app_container = document
                    .get_element_by_id("app-container")
                    .ok_or(JsValue::from_str("Could not find app-container"))?;
                
                app_container.append_child(&canvas_container)?;
                
                // Create canvas
                let canvas = document.create_element("canvas")?;
                canvas.set_id("node-canvas");
                canvas_container.append_child(&canvas)?;
                
                // Set up the canvas once it's created
                crate::components::canvas_editor::setup_canvas(document)?;
            } else {
                web_sys::console::log_1(&"Showing existing canvas container".into());
                // Just show the existing canvas container
                if let Some(canvas) = document.get_element_by_id("canvas-container") {
                    canvas.set_attribute("style", "display: block;")?;
                }
            }
            
            // Show input panel for canvas view
            if let Some(input_panel) = document.get_element_by_id("input-panel") {
                input_panel.set_attribute("style", "display: block;")?;
            }
            
            // Update tab styling
            if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
                dashboard_tab.set_class_name("tab-button");
            }
            
            if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
                canvas_tab.set_class_name("tab-button active");
            }
            web_sys::console::log_1(&"Canvas view setup complete".into());
        },
        ActiveView::ChatView => {
            // Setup the chat view if needed
            crate::components::chat_view::setup_chat_view(document)?;
            
            // Show chat view container
            if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
                chat_container.set_attribute("style", "display: flex;")?;
            }
        }
    }
    
    Ok(())
}