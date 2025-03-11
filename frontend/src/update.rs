// frontend/src/update.rs
//
use crate::messages::Message;
use crate::state::AppState;
use crate::models::NodeType;
use crate::storage::ActiveView;

pub fn update(state: &mut AppState, msg: Message) {
    match msg {
        Message::ToggleView(view) => {
            state.active_view = view;
            state.state_modified = true;
        },
        
        Message::CreateAgent(name) => {
            // Create a new agent node at a default position
            let _ = state.add_node(name, 100.0, 100.0, NodeType::AgentIdentity);
            state.state_modified = true;
        },
        
        Message::CreateAgentWithDetails { name, agent_id, system_instructions, task_instructions } => {
            // Calculate center position for the new agent
            let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
            let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
            
            let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - 75.0; // Center - half node width
            let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - 50.0; // Center - half node height
            
            // Create the node with the proper ID format
            let node_id = format!("agent-{}", agent_id);
            
            // Create and add the node directly to state
            let node = crate::models::Node {
                id: node_id.clone(),
                x,
                y,
                text: name,
                width: 200.0,
                height: 80.0,
                color: "#ffecb3".to_string(), // Light amber color
                parent_id: None,
                node_type: NodeType::AgentIdentity,
                system_instructions: Some(system_instructions),
                task_instructions: Some(task_instructions),
                history: Some(Vec::new()),
                status: Some("idle".to_string()),
            };
            
            // Add the node to our state
            state.nodes.insert(node_id.clone(), node);
            
            // Log the new node ID
            web_sys::console::log_1(&format!("Created new agent with ID: {}", node_id).into());
            
            // Draw the nodes on canvas
            state.draw_nodes();
            
            state.state_modified = true;
        },
        
        Message::EditAgent(agent_id) => {
            state.selected_node_id = Some(agent_id);
            state.active_view = ActiveView::Canvas;
            state.state_modified = true;
            
            // Show the agent modal for editing
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            if let Err(e) = crate::views::show_agent_modal(state, &document) {
                web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
            }
        },
        
        Message::DeleteAgent(agent_id) => {
            state.nodes.remove(&agent_id);
            state.state_modified = true;
        },
        
        Message::UpdateNodePosition { node_id, x, y } => {
            state.update_node_position(&node_id, x, y);
            // Only mark state as modified if we're not actively dragging
            // This prevents triggering state saves for every tiny move
            if !state.is_dragging_agent {
                state.state_modified = true;
            }
        },
        
        Message::AddNode { text, x, y, node_type } => {
            // Special case for agents created with default coordinates (0, 0)
            // This indicates we should position at viewport center
            if node_type == NodeType::AgentIdentity && x == 0.0 && y == 0.0 {
                // Calculate center position for the new agent
                let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
                let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
                
                let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - 75.0; // Center - half node width
                let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - 50.0; // Center - half node height
                
                let node_id = state.add_node(text, x, y, node_type);
                web_sys::console::log_1(&format!("Created new agent node: {}", node_id).into());
            } else {
                // Normal case with specific coordinates
                let _ = state.add_node(text, x, y, node_type);
            }
            state.state_modified = true;
        },
        
        Message::AddResponseNode { parent_id, response_text } => {
            let _ = state.add_response_node(&parent_id, response_text);
            state.state_modified = true;
        },
        
        Message::ToggleAutoFit => {
            state.toggle_auto_fit();
            state.state_modified = true;
        },
        
        Message::CenterView => {
            state.center_view();
            state.state_modified = true;
        },
        
        Message::ClearCanvas => {
            state.nodes.clear();
            state.latest_user_input_id = None;
            state.message_id_to_node_id.clear();
            state.viewport_x = 0.0;
            state.viewport_y = 0.0;
            state.zoom_level = 1.0;
            state.auto_fit = true;
            state.state_modified = true;
        },
        
        Message::UpdateInputText(text) => {
            state.input_text = text;
        },
        
        Message::StartDragging { node_id, offset_x, offset_y } => {
            state.dragging = Some(node_id);
            state.drag_offset_x = offset_x;
            state.drag_offset_y = offset_y;
            state.is_dragging_agent = true;
        },
        
        Message::StopDragging => {
            state.dragging = None;
            state.is_dragging_agent = false;
            state.state_modified = true;
            
            // Explicitly save state to API when dragging is complete
            // This ensures we only save the final position once
            crate::storage::save_state_to_api(state);
        },
        
        Message::StartCanvasDrag { start_x, start_y } => {
            state.canvas_dragging = true;
            state.drag_start_x = start_x;
            state.drag_start_y = start_y;
            state.drag_last_x = start_x;
            state.drag_last_y = start_y;
        },
        
        Message::UpdateCanvasDrag { current_x, current_y } => {
            if state.canvas_dragging {
                let dx = current_x - state.drag_last_x;
                let dy = current_y - state.drag_last_y;
                
                state.viewport_x -= dx;
                state.viewport_y -= dy;
                
                state.drag_last_x = current_x;
                state.drag_last_y = current_y;
                
                state.state_modified = true;
            }
        },
        
        Message::StopCanvasDrag => {
            state.canvas_dragging = false;
            state.state_modified = true;
        },
        
        Message::ZoomCanvas { new_zoom, viewport_x, viewport_y } => {
            state.zoom_level = new_zoom;
            state.viewport_x = viewport_x;
            state.viewport_y = viewport_y;
            state.state_modified = true;
            
            // Redraw with the new zoom level
            state.draw_nodes();
        },
        
        Message::SaveAgentDetails { name, system_instructions, task_instructions } => {
            if let Some(id) = &state.selected_node_id {
                let id_clone = id.clone();
                if let Some(node) = state.nodes.get_mut(&id_clone) {
                    // Update the node name if provided
                    if !name.trim().is_empty() {
                        node.text = name.clone();
                    }
                    
                    // Update system instructions
                    node.system_instructions = Some(system_instructions.clone());
                    
                    // Update task instructions
                    node.task_instructions = Some(task_instructions.clone());
                    
                    state.state_modified = true;
                }
            }
            
            // The modal UI update is handled in the view render
        },
        
        Message::CloseAgentModal => {
            // The actual UI update to hide the modal is handled in the view render
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            if let Err(e) = crate::views::hide_agent_modal(&document) {
                web_sys::console::error_1(&format!("Failed to hide modal: {:?}", e).into());
            }
        },
        
        Message::SendTaskToAgent => {
            if let Some(agent_id) = &state.selected_node_id {
                let agent_id_clone = agent_id.clone();
                
                // Get the task instructions using our helper method
                let task_text = state.get_task_instructions_with_fallback(&agent_id_clone);
                
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
                
                // Adjust viewport to fit all nodes if auto-fit is enabled
                if state.auto_fit {
                    state.fit_nodes_to_view();
                }
                
                // Add a response node (this creates a visual node for the response)
                let response_node_id = state.add_response_node(&agent_id_clone, "...".to_string());
                
                // Track the message ID to node ID mapping
                state.track_message(message_id.clone(), response_node_id);
                
                // Store the necessary values for the network call
                // This way, we don't need to borrow APP_STATE again in the send_text_to_backend function
                let task_text_clone = task_text.clone();
                let message_id_clone = message_id.clone();
                
                // Mark state as modified
                state.state_modified = true;
                
                // Close the modal after sending - this doesn't borrow APP_STATE
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                if let Err(e) = crate::views::hide_agent_modal(&document) {
                    web_sys::console::error_1(&format!("Failed to hide modal: {:?}", e).into());
                }
                
                // After the update function returns and the AppState borrow is dropped,
                // the dispatch function caller will need to call this function:
                // crate::network::send_text_to_backend(&task_text_clone, message_id_clone);
                
                // For now, we'll use a simple approach: stash the data in state
                state.pending_network_call = Some((task_text_clone, message_id_clone));
            }
        },
        
        Message::SwitchToMainTab => {
            // The actual UI update is handled in the view render
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            
            // Show main content, hide history content
            if let Some(main_content) = document.get_element_by_id("main-content") {
                if let Err(e) = main_content.set_attribute("style", "display: block;") {
                    web_sys::console::error_1(&format!("Failed to show main content: {:?}", e).into());
                }
            }
            
            if let Some(history_content) = document.get_element_by_id("history-content") {
                if let Err(e) = history_content.set_attribute("style", "display: none;") {
                    web_sys::console::error_1(&format!("Failed to hide history content: {:?}", e).into());
                }
            }
            
            // Update active tab
            if let Some(main_tab) = document.get_element_by_id("main-tab") {
                main_tab.set_class_name("tab-button active");
            }
            
            if let Some(history_tab) = document.get_element_by_id("history-tab") {
                history_tab.set_class_name("tab-button");
            }
        },
        
        Message::SwitchToHistoryTab => {
            // The actual UI update is handled in the view render
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("should have a document");
            
            // Hide main content, show history content
            if let Some(main_content) = document.get_element_by_id("main-content") {
                if let Err(e) = main_content.set_attribute("style", "display: none;") {
                    web_sys::console::error_1(&format!("Failed to hide main content: {:?}", e).into());
                }
            }
            
            if let Some(history_content) = document.get_element_by_id("history-content") {
                if let Err(e) = history_content.set_attribute("style", "display: block;") {
                    web_sys::console::error_1(&format!("Failed to show history content: {:?}", e).into());
                }
            }
            
            // Update active tab
            if let Some(main_tab) = document.get_element_by_id("main-tab") {
                main_tab.set_class_name("tab-button");
            }
            
            if let Some(history_tab) = document.get_element_by_id("history-tab") {
                history_tab.set_class_name("tab-button active");
            }
        },
        
        Message::UpdateSystemInstructions(instructions) => {
            if let Some(id) = &state.selected_node_id {
                let id_clone = id.clone();
                if let Some(node) = state.nodes.get_mut(&id_clone) {
                    node.system_instructions = Some(instructions);
                    state.state_modified = true;
                }
            }
        },
        
        Message::UpdateAgentName(name) => {
            if let Some(id) = &state.selected_node_id {
                let id_clone = id.clone();
                if let Some(node) = state.nodes.get_mut(&id_clone) {
                    // Only update if name is not empty
                    if !name.trim().is_empty() {
                        node.text = name;
                        state.state_modified = true;
                        
                        // Update modal title for consistency
                        let window = web_sys::window().expect("no global window exists");
                        let document = window.document().expect("should have a document");
                        if let Some(modal_title) = document.get_element_by_id("modal-title") {
                            modal_title.set_inner_html(&format!("Agent: {}", node.text));
                        }
                    }
                }
            }
        },
    }
} 