// frontend/src/update.rs
//
use crate::messages::{Message, Command};
use crate::state::{AppState, APP_STATE, dispatch_global_message};
use crate::models::{NodeType, ApiThread, ApiThreadMessage, ApiAgent};
use crate::constants::{
    DEFAULT_NODE_WIDTH,
    DEFAULT_NODE_HEIGHT,
    DEFAULT_AGENT_NODE_COLOR,
    DEFAULT_THREAD_TITLE,
};
use web_sys::Document;
use wasm_bindgen::JsValue;
use std::collections::HashMap;
use crate::components::chat_view::{update_thread_list_ui, update_conversation_ui};
use serde_json;
use rand;
use chrono;
use std::collections::HashSet;


pub fn update(state: &mut AppState, msg: Message) -> Vec<Command> {
    let mut needs_refresh = true; // We'll still track this internally for now
    let mut commands = Vec::new(); // Collect commands to return

    match msg {
        Message::ToggleView(view) => {
            state.active_view = view;
            state.state_modified = true;
            
            // Add a command to refresh the UI after view change
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Err(e) = AppState::refresh_ui_after_state_change() {
                    web_sys::console::error_1(&format!("Failed to refresh UI after view change: {:?}", e).into());
                }
            })));
        },
       
        Message::CreateAgent(name) => {
            // Create a new agent node at a default position
            let _ = state.add_node(name, 100.0, 100.0, NodeType::AgentIdentity);
            state.state_modified = true;
        },
       
        Message::CreateAgentWithDetails { name, agent_id, system_instructions: _system_instructions, task_instructions: _task_instructions } => {
            // Calculate center position for the new agent
            let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
            let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
           
            let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - DEFAULT_NODE_WIDTH / 2.0;
            let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - DEFAULT_NODE_HEIGHT / 2.0;
           
            // Create the node with the proper ID format
            let node_id = format!("agent-{}", agent_id);
           
            // Create and add the node directly to state
            let node = crate::models::Node {
                node_id: node_id.clone(),
                agent_id: Some(agent_id),
                x,
                y,
                text: name,
                width: DEFAULT_NODE_WIDTH,
                height: DEFAULT_NODE_HEIGHT,
                color: DEFAULT_AGENT_NODE_COLOR.to_string(),
                node_type: NodeType::AgentIdentity,
                parent_id: None,
                is_selected: false,
                is_dragging: false,
            };
           
            // Add the node to our state
            state.nodes.insert(node_id.clone(), node);
           
            // Log the new node ID - make it clear this is a visual node representing an agent
            web_sys::console::log_1(&format!("Created visual node with ID: {} for agent {}", node_id, agent_id).into());
           
            // Draw the nodes on canvas
            state.draw_nodes();
           
            // After creating the agent, immediately create a default thread
            commands.push(Command::SendMessage(Message::CreateThread(agent_id, DEFAULT_THREAD_TITLE.to_string())));
           
            // IMPORTANT: We intentionally don't set state_modified = true here
            // This prevents the auto-save mechanism from making redundant API calls
            
            // Note: A default thread will be created by our Message::AgentsRefreshed handler
            // when the agent list is refreshed from the API.
        },
       
        Message::EditAgent(agent_id) => {
            web_sys::console::log_1(&format!("Update: Handling EditAgent for agent_id: {}", agent_id).into());
            // Find the node associated with this agent ID
            let node_id_to_select = state.nodes.iter()
                .find(|(_, node)| node.agent_id == Some(agent_id))
                .map(|(id, _)| id.clone());

            if let Some(node_id) = node_id_to_select {
                // Happy‑path: we already have a visual node for this agent
                web_sys::console::log_1(&format!("Found node_id {} for agent_id {}, selecting it.", node_id, agent_id).into());
                state.selected_node_id = Some(node_id);
                state.state_modified = true;

                // Open the modal
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                if let Err(e) = crate::views::show_agent_modal(state, &document) {
                    web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
                }
            } else {
                // Fallback: no canvas node yet (e.g., user came from Dashboard before generating canvas)
                let synthetic_node_id = format!("agent-{}", agent_id);
                web_sys::console::log_1(&format!("No visual node for agent_id {}. Using synthetic id {}.", agent_id, synthetic_node_id).into());

                state.selected_node_id = Some(synthetic_node_id.clone());
                // Not marking state_modified because we haven't changed any persistent state

                // Directly open the modal with this synthetic id – show_agent_modal already has logic
                // to fall back to agent data when it receives an id of the form "agent-{id}".
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                if let Err(e) = crate::views::show_agent_modal(state, &document) {
                    web_sys::console::error_1(&format!("Failed to show modal for synthetic id: {:?}", e).into());
                }

                // We purposely skip any canvas refresh here
                needs_refresh = false;
            }
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
                web_sys::console::log_1(&format!("Created visual node for agent: {}", node_id).into());
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
       
        Message::SaveAgentDetails { name, system_instructions, task_instructions: _task_instructions, model, schedule, run_on_schedule } => {
            // Get the current node ID from the modal
            let node_id = if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(modal) = document.get_element_by_id("agent-modal") {
                        modal.get_attribute("data-node-id")
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            };
            
            // Process the save operation
            if let Some(node_id) = node_id {
                // Extract agent_id from node_id if in format "agent-{id}"
                let agent_id = if let Some(node) = state.nodes.get(&node_id) {
                    // If we have a node, get its agent_id
                    node.agent_id
                } else if let Some(id_str) = node_id.strip_prefix("agent-") {
                    // If no node but ID is in "agent-{id}" format, extract numeric ID
                    id_str.parse::<u32>().ok()
                } else {
                    None
                };
                
                // Update node if it exists
                if let Some(node) = state.nodes.get_mut(&node_id) {
                    // Update node's visual representation
                    node.text = name.clone();
                }
                
                // Update agent data if we have an agent ID
                if let Some(id) = agent_id {
                    if let Some(agent) = state.agents.get_mut(&id) {
                        // Update agent properties
                        agent.name = name;
                        agent.system_instructions = Some(system_instructions.clone());
                        agent.model = Some(model.clone());
                        agent.schedule = schedule.clone();
                        agent.run_on_schedule = Some(run_on_schedule);
                        
                        // Build update struct
                        use crate::models::ApiAgentUpdate;
                        let api_update = ApiAgentUpdate {
                            name: Some(agent.name.clone()),
                            status: None,
                            system_instructions: Some(agent.system_instructions.clone().unwrap_or_default()),
                            task_instructions: None,
                            model: Some(model.clone()),
                            schedule: schedule.clone(),
                            run_on_schedule: Some(run_on_schedule),
                            config: None,
                            last_error: None,
                        };

                        // Serialize to JSON
                        let update_payload = serde_json::to_string(&api_update).unwrap_or_else(|_| "{}".to_string());

                        // Return a command to update the agent via API
                        commands.push(Command::UpdateAgent {
                            agent_id: id,
                            payload: update_payload,
                            on_success: Box::new(Message::RefreshAgentsFromAPI),
                            on_error: Box::new(Message::RefreshAgentsFromAPI),
                        });
                    }
                }
                
                // Mark state as modified
                state.state_modified = true;
                
                // Save state to API
                let _ = state.save_if_modified();
                
                // Close the modal after saving
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Err(e) = crate::ui::modals::close_agent_modal(&document) {
                            web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
                        }
                    }
                }
            }
        },
       
        Message::CloseAgentModal => {
            // Get the document to close the modal
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    // Close the modal using the modal helper function
                    if let Err(e) = crate::ui::modals::close_agent_modal(&document) {
                        web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
                    }
                }
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
                   
                    // Instead of directly modifying history, use API calls
                    // Save the message to agent history via API
                    crate::storage::save_agent_messages_to_api(&agent_id_clone, &[user_message]);
                   
                    // Update agent status via API if it has an agent_id
                    if let Some(_agent_id) = agent_node.agent_id {
                        agent_node.set_status(Some("processing".to_string()));
                    }
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
                let task_text_clone = task_text.clone();
                let message_id_clone = message_id.clone();

                // Generate a u32 client ID for the send command
                let client_id_u32 = u32::MAX - rand::random::<u32>() % 1000;

                // Mark state as modified
                state.state_modified = true;

                // Close the modal after sending - this doesn't borrow APP_STATE
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                if let Err(e) = crate::views::hide_agent_modal(&document) {
                    web_sys::console::error_1(&format!("Failed to hide modal: {:?}", e).into());
                }

                // After the update function returns and the AppState borrow is dropped,
                // we'll send a command to handle the thread message
                commands.push(Command::SendThreadMessage {
                    thread_id: state.current_thread_id.unwrap_or(1),
                    client_id: Some(client_id_u32),
                    content: task_text_clone.clone(),
                });

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
                    node.set_system_instructions(Some(instructions));
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
       
        Message::UpdateNodeText { node_id, text, is_first_chunk } => {
            if let Some(node) = state.nodes.get_mut(&node_id) {
                // If this is the first chunk, replace the text, otherwise append
                if is_first_chunk {
                    node.text = text;
                } else {
                    node.text.push_str(&text);
                }
               
                // Update node size based on new content
                state.resize_node_for_content(&node_id);
                state.state_modified = true;
            }
        },
       
        Message::CompleteNodeResponse { node_id, final_text } => {
            if let Some(node) = state.nodes.get_mut(&node_id) {
                // Update the final text if needed
                if !final_text.is_empty() && node.text != final_text {
                    node.text = final_text;
                }
               
                // Update node status to completed
                node.set_status(Some("complete".to_string()));
               
                // Store parent_id before ending the borrow or making other mutable borrows
                let parent_id = node.parent_id.clone();
               
                // Mark state as modified
                state.state_modified = true;
               
                // Update node size for final content
                state.resize_node_for_content(&node_id);
               
                // If this node has a parent, update parent status too
                if let Some(parent_id) = parent_id {
                    if let Some(parent) = state.nodes.get_mut(&parent_id) {
                        parent.set_status(Some("idle".to_string()));
                    }
                }
            }
        },
       
        Message::UpdateNodeStatus { node_id, status } => {
            if let Some(node) = state.nodes.get_mut(&node_id) {
                // Update the node's visual properties based on status
                match status.as_str() {
                    "idle" => node.color = "#ffecb3".to_string(),      // Light amber
                    "processing" => node.color = "#b3e5fc".to_string(), // Light blue
                    "complete" => node.color = "#c8e6c9".to_string(),   // Light green
                    "error" => node.color = "#ffcdd2".to_string(),      // Light red
                    _ => node.color = "#ffecb3".to_string(),           // Default light amber
                }
               
                // If this node is associated with an agent, update the agent's status
                // but do it through an explicit sync mechanism
                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        // Update agent status directly in our local model
                        agent.status = Some(status.clone());
                       
                        // Create an update for the API
                        let update = crate::models::ApiAgentUpdate {
                            name: None,
                            status: Some(status.clone()),
                            system_instructions: None,
                            task_instructions: None,
                            model: None,
                            schedule: None,
                            run_on_schedule: None,
                            config: None,
                            last_error: None,
                        };
                       
                        // Clone data for async use
                        let agent_id_clone = agent_id;
                        let update_clone = update.clone();
                       
                        // Update the agent via API
                        wasm_bindgen_futures::spawn_local(async move {
                            // Serialize the update struct to a JSON string
                            match serde_json::to_string(&update_clone) {
                                Ok(json_str) => {
                                    if let Err(e) = crate::network::ApiClient::update_agent(agent_id_clone, &json_str).await {
                                        web_sys::console::error_1(&format!("Failed to update agent: {:?}", e).into());
                                    }
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to serialize agent update: {}", e).into());
                                }
                            }
                        });
                    }
                }
               
                state.state_modified = true;
            }
        },
       
        Message::AnimationTick => {
            // Process animation updates like pulsing effect for nodes
            for (_id, node) in state.nodes.iter_mut() {
                // Use the new method that doesn't access APP_STATE
                let status = node.get_status_from_agents(&state.agents);
               
                if let Some(status_str) = status {
                    if status_str == "processing" {
                        // We'd update visual properties here if needed
                        // This is called on each animation frame
                    }
                }
            }
        },
       
        // New Canvas Node message handlers
        Message::AddAgentNode { agent_id, x, y, node_type, text } => {
            // This method creates a Node
            let node_id = state.add_node_with_agent(agent_id, x, y, node_type, text);
            web_sys::console::log_1(&format!("Created new node: {}", node_id).into());
            state.state_modified = true;
        },
       
        Message::DeleteNode { node_id } => {
            // This method deletes a Node
            state.nodes.remove(&node_id);
           
            // Also remove it from the current workflow if it exists
            if let Some(workflow_id) = state.current_workflow_id {
                if let Some(workflow) = state.workflows.get_mut(&workflow_id) {
                    workflow.nodes.retain(|n| n.node_id != node_id);
                   
                    // Also remove any edges connected to this node
                    workflow.edges.retain(|edge|
                        edge.from_node_id != node_id && edge.to_node_id != node_id
                    );
                }
            }
           
            state.state_modified = true;
        },
       
        // Workflow management messages
        Message::CreateWorkflow { name } => {
            let workflow_id = state.create_workflow(name.clone());
            web_sys::console::log_1(&format!("Created new workflow '{}' with ID: {}", name, workflow_id).into());
           
            state.state_modified = true;
        },
       
        Message::SelectWorkflow { workflow_id } => {
            state.current_workflow_id = Some(workflow_id);
           
            // Clear nodes and repopulate from the selected workflow
            state.nodes.clear();
           
            if let Some(workflow) = state.workflows.get(&workflow_id) {
                for node in &workflow.nodes {
                    state.nodes.insert(node.node_id.clone(), node.clone());
                }
            }
           
            // Draw the nodes on canvas
            state.draw_nodes();
           
            state.state_modified = true;
        },
       
        Message::AddEdge { from_node_id, to_node_id, label } => {
            let edge_id = state.add_edge(from_node_id, to_node_id, label);
            web_sys::console::log_1(&format!("Created new edge with ID: {}", edge_id).into());
           
            // Draw the nodes and edges on canvas
            state.draw_nodes();
           
            state.state_modified = true;
        },
       
        Message::SyncNodeToAgent { node_id, agent_id } => {
            // Explicitly sync node data to agent (e.g., when node text changes and should update agent name)
            if let Some(node) = state.nodes.get(&node_id) {
                if let Some(agent) = state.agents.get_mut(&agent_id) {
                    // Update agent data based on node
                    // For now, we're just syncing the name from node.text
                    agent.name = node.text.clone();
                   
                    // Create an update object for the API
                    let update = crate::models::ApiAgentUpdate {
                        name: Some(agent.name.clone()),
                        status: None,
                        system_instructions: None,
                        task_instructions: None,
                        model: None,
                        schedule: None,
                        run_on_schedule: None,
                        config: None,
                        last_error: None,
                    };
                   
                    // Clone data for async use
                    let agent_id_clone = agent_id;
                    let update_clone = update.clone();
                   
                    // Update the agent via API
                    wasm_bindgen_futures::spawn_local(async move {
                        // Serialize the update struct to a JSON string
                        match serde_json::to_string(&update_clone) {
                            Ok(json_str) => {
                                if let Err(e) = crate::network::ApiClient::update_agent(agent_id_clone, &json_str).await {
                                    web_sys::console::error_1(&format!("Failed to update agent: {:?}", e).into());
                                }
                            },
                            Err(e) => {
                                web_sys::console::error_1(&format!("Failed to serialize agent update: {}", e).into());
                            }
                        }
                    });
                   
                    state.state_modified = true;
                }
            }
        },
       
        Message::SyncAgentToNode { agent_id, node_id } => {
            // Explicitly sync agent data to node (e.g., when agent data changes and should update node display)
            if let Some(agent) = state.agents.get(&agent_id) {
                if let Some(node) = state.nodes.get_mut(&node_id) {
                    // Update node data based on agent
                    node.text = agent.name.clone();
                   
                    // Update node color based on agent status if desired
                    if let Some(status) = &agent.status {
                        // Example of how you might set node color based on agent status
                        match status.as_str() {
                            "idle" => node.color = "#ffecb3".to_string(),      // Light amber
                            "processing" => node.color = "#b3e5fc".to_string(), // Light blue
                            "complete" => node.color = "#c8e6c9".to_string(),   // Light green
                            "error" => node.color = "#ffcdd2".to_string(),      // Light red
                            _ => node.color = "#ffecb3".to_string(),           // Default light amber
                        }
                    }
                   
                    state.state_modified = true;
                }
            }
        },
       
        Message::GenerateCanvasFromAgents => {
            // Loop through all agents in state.agents and create nodes for any that don't have one
            let mut nodes_created = 0;
           
            // First, find all agents that need nodes
            let agents_needing_nodes = state.agents.iter()
                .filter(|(id, _)| {
                    // Check if this agent already has a node
                    let node_id = format!("agent-{}", id);
                    !state.nodes.contains_key(&node_id)
                })
                .map(|(id, agent)| (*id, agent.name.clone()))
                .collect::<Vec<_>>();
           
            // Now create nodes for each agent
            for (i, (agent_id, name)) in agents_needing_nodes.iter().enumerate() {
                // Calculate grid position
                let row = i / 3; // 3 nodes per row
                let col = i % 3;
               
                let x = 100.0 + (col as f64 * 250.0); // 250px horizontal spacing
                let y = 100.0 + (row as f64 * 150.0); // 150px vertical spacing
               
                // Create the node with the proper ID format
                let node_id = format!("agent-{}", agent_id);
               
                // Create and add the node directly to state
                let node = crate::models::Node {
                    node_id: node_id.clone(),
                    agent_id: Some(*agent_id),
                    x,
                    y,
                    text: name.clone(),
                    width: 200.0,
                    height: 80.0,
                    color: "#ffecb3".to_string(), // Light amber color
                    node_type: NodeType::AgentIdentity,
                    parent_id: None,
                    is_selected: false,
                    is_dragging: false,
                };
               
                // Add the node to our state
                state.nodes.insert(node_id.clone(), node);
                nodes_created += 1;
            }
           
            web_sys::console::log_1(&format!("Created {} nodes for agents without visual representation", nodes_created).into());
           
            // Only mark as modified if we actually created nodes
            if nodes_created > 0 {
                state.state_modified = true;
                // Draw the updated nodes on canvas
                state.draw_nodes();
            }
        },
       
        Message::ResetDatabase => {
            // The actual database reset happens via API call (already done in dashboard.rs)
            // We don't need to do anything here because:
            // 1. The page will be refreshed immediately after this (in dashboard.rs)
            // 2. On refresh, it will automatically load the fresh state from the backend
            web_sys::console::log_1(&"Reset database message received - state will refresh".into());
        },
       
        Message::RefreshAgentsFromAPI => {
            // Trigger an async operation to fetch agents from the API
            web_sys::console::log_1(&"Requesting agent refresh from API".into());
            // Return a command to fetch agents instead of doing it directly
            commands.push(Command::FetchAgents);
            needs_refresh = false; // The command executor will handle subsequent messages/updates
        },
       
        Message::AgentsRefreshed(agents) => {
            web_sys::console::log_1(&format!("Update: Handling AgentsRefreshed with {} agents", agents.len()).into());

            // Get the current set of agent IDs BEFORE updating
            let old_agent_ids: HashSet<u32> = state.agents.keys().cloned().collect();

            // Update state.agents with the new list
            state.agents.clear();
            let mut new_agent_ids = HashSet::new();
            for agent in &agents {
                if let Some(id) = agent.id {
                    state.agents.insert(id, agent.clone());
                    new_agent_ids.insert(id);
                }
            }

            // Check for newly created agent
            let just_created_agent_ids: Vec<u32> = new_agent_ids.difference(&old_agent_ids).cloned().collect();
            
            if just_created_agent_ids.len() == 1 {
                let new_agent_id = just_created_agent_ids[0];
                web_sys::console::log_1(&format!("Detected newly created agent ID: {}. Creating default thread.", new_agent_id).into());
                commands.push(Command::SendMessage(Message::CreateThread(new_agent_id, DEFAULT_THREAD_TITLE.to_string())));
            } else if just_created_agent_ids.len() > 1 {
                web_sys::console::warn_1(&"Detected multiple new agents after refresh, cannot auto-create default thread.".into());
            }
            
            // Schedule a UI refresh after state is updated
            state.pending_ui_updates = Some(Box::new(|| {
                if let Err(e) = AppState::refresh_ui_after_state_change() {
                    web_sys::console::error_1(&format!("Failed to refresh UI after AgentsRefreshed: {:?}", e).into());
                }
            }));
            needs_refresh = false; // Refresh handled by pending_ui_updates
        },
       
        // Thread-related messages
        Message::LoadThreads(agent_id) => {
            state.is_chat_loading = true;
           
            // Instead of spawning an async task directly, return a command
            commands.push(Command::FetchThreads(agent_id));
        },
       
        Message::ThreadsLoaded(threads) => {
            // Only process if we're in ChatView
            if state.active_view == crate::storage::ActiveView::ChatView {
                web_sys::console::log_1(&format!("Update: Handling ThreadsLoaded with {} threads", threads.len()).into());
                
                // Update state
                state.threads = threads.iter()
                    .filter_map(|t| t.id.map(|id| (id, t.clone())))
                    .collect();
                state.is_chat_loading = false;
                
                // If no thread selected, select first thread
                let selected_thread_id = if state.current_thread_id.is_none() {
                    threads.first().and_then(|t| t.id)
                } else {
                    None
                };
                
                // Instead of scheduling side effects directly, return commands
                if let Some(thread_id) = selected_thread_id {
                    commands.push(Command::SendMessage(Message::SelectThread(thread_id)));
                }
            } else {
                web_sys::console::warn_1(&"Received ThreadsLoaded outside of ChatView".into());
            }
        },
       
        Message::CreateThread(agent_id, title) => {
            // Log for debugging
            web_sys::console::log_1(&format!("Creating thread for agent {} with title: {}", agent_id, title).into());
            
            // Instead of spawning directly, return a command that will be executed after state update
            commands.push(Command::CreateThread { 
                agent_id, 
                title 
            });
        },
       
        Message::ThreadCreated(thread) => {
            // Data is already ApiThread
            web_sys::console::log_1(&format!("Update: Handling ThreadCreated: {:?}", thread).into()); // Use {:?}
            if let Some(thread_id) = thread.id {
                state.threads.insert(thread_id, thread.clone());
                
                // Return SelectThread as a command instead of dispatching directly
                commands.push(Command::SendMessage(Message::SelectThread(thread_id)));
            } else {
                 web_sys::console::error_1(&format!("ThreadCreated message missing thread ID: {:?}", thread).into()); // Use {:?}
            }
        },
       
        Message::SelectThread(thread_id) => {
            web_sys::console::log_1(&format!("State: Selecting thread {}", thread_id).into());
            
            // Only proceed if the thread is actually changing or wasn't set
            if state.current_thread_id != Some(thread_id) {
                // Update state
                state.current_thread_id = Some(thread_id);
                state.is_chat_loading = true;
                state.thread_messages.remove(&thread_id); // Clear stale messages
                state.active_streams.remove(&thread_id); // Clear stale streams

                // Get the title of the selected thread to update the header
                let selected_thread_title = state.threads.get(&thread_id)
                    .expect("Selected thread not found in state")
                    .title
                    .clone();

                // Return commands for side effects
                commands.push(Command::SendMessage(Message::LoadThreadMessages(thread_id)));
                
                // Collect data needed for UI updates
                let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
                let current_thread_id = state.current_thread_id;
                let thread_messages = state.thread_messages.clone();
                
                // Update thread list UI
                commands.push(Command::SendMessage(Message::UpdateThreadList(
                    threads,
                    current_thread_id,
                    thread_messages
                )));

                // Update the main thread title UI
                commands.push(Command::SendMessage(Message::UpdateThreadTitleUI(selected_thread_title)));

                // Critical fix: Initialize WebSocket subscription for the thread topic
                // This ensures that the frontend will receive stream messages from the backend
                let topic_manager = state.topic_manager.clone();
                commands.push(Command::UpdateUI(Box::new(move || {
                    if let Err(e) = crate::components::chat::init_chat_view_ws(thread_id, topic_manager) {
                        web_sys::console::error_1(&format!("Failed to initialize WebSocket for thread {}: {:?}", thread_id, e).into());
                    } else {
                        web_sys::console::log_1(&format!("Initialized WebSocket subscription for thread {}", thread_id).into());
                    }
                })));
            }
        },
       
        Message::LoadThreadMessages(thread_id) => {
            state.is_chat_loading = true;
           
            // Instead of spawning an async task directly, return a command
            // that will be executed after the state update is complete
            commands.push(Command::FetchThreadMessages(thread_id));
        },
       
        Message::ThreadMessagesLoaded(thread_id, messages) => {
            // Data is already Vec<ApiThreadMessage>
            web_sys::console::log_1(&format!("Update: Handling ThreadMessagesLoaded for {}: {} messages", thread_id, messages.len()).into());
            state.thread_messages.insert(thread_id, messages);
            state.is_chat_loading = false;
            // Trigger UI update for conversation area implicitly via state change
        },
       
        Message::SendThreadMessage(thread_id, content) => {
            web_sys::console::log_1(&format!("Update: Handling SendThreadMessage for thread {}: '{}'", thread_id, content).into());

            // Generate a client ID for tracking this message
            let client_id = u32::MAX - rand::random::<u32>() % 1000;
            
            // Create an optimistic message for immediate UI feedback
            let now = chrono::Utc::now().to_rfc3339();
            let user_message = ApiThreadMessage {
                id: Some(client_id),
                thread_id,
                role: "user".to_string(),
                content: content.clone(),
                created_at: Some(now),
            };
            
            // Add optimistic message to state
            if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                messages.push(user_message.clone());
            } else {
                state.thread_messages.insert(thread_id, vec![user_message.clone()]);
            }
            
            // Prepare UI update commands
            let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            
            // Add UI update command
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                    dispatch_global_message(Message::UpdateConversation(
                        thread_messages.get(&thread_id).cloned().unwrap_or_default()
                    ));
                    dispatch_global_message(Message::UpdateThreadList(
                        threads,
                        current_thread_id,
                        thread_messages
                    ));
                }
            })));
            
            web_sys::console::log_1(&format!("Update: Pushing Command::SendThreadMessage for thread {} with client_id {}", thread_id, client_id).into());
            // Add network operation command
            commands.push(Command::SendThreadMessage {
                thread_id,
                client_id: Some(client_id),
                content: content.clone(),
            });
        },
       
        Message::ThreadMessageSent(response, client_id) => {
            // Parse the response from the server
            if let Ok(thread_message) = serde_json::from_str::<ApiThreadMessage>(&response) {
                let thread_id = thread_message.thread_id;
                
                // Try to parse the client_id string back to u32
                if let Ok(client_id_num) = client_id.parse::<u32>() {
                    // Find and replace the optimistic message with the confirmed message
                    if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                        // Find the index of the optimistic message
                        if let Some(index) = messages.iter().position(|msg| 
                            msg.id.as_ref().map_or(false, |id| *id == client_id_num)
                        ) {
                            // Replace the optimistic message with the confirmed message
                            messages[index] = thread_message.clone();
                            
                            // Return a UI update command
                            let messages_clone = messages.clone();
                            commands.push(Command::UpdateUI(Box::new(move || {
                                if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                                    // Update the conversation UI
                                    dispatch_global_message(Message::UpdateConversation(messages_clone));
                                }
                            })));
                        }
                    }
                }
                
                // Trigger the thread to run and process the message
                commands.push(Command::RunThread(thread_id));
            }
        },
       
        Message::ThreadMessageFailed(thread_id, client_id) => {
            // Try to parse the client_id string back to u32
            if let Ok(client_id_num) = client_id.parse::<u32>() {
                // Find and update the status of the optimistic message
                if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                    // Find the optimistic message by its client ID
                    if let Some(message) = messages.iter_mut().find(|msg| 
                        msg.id.as_ref().map_or(false, |id| *id == client_id_num)
                    ) {
                        // Mark as failed by adding a special tag to the content
                        message.content = format!("[Failed to send] {}", message.content);
                        
                        // Return a UI update command
                        let messages_clone = messages.clone();
                        commands.push(Command::UpdateUI(Box::new(move || {
                            if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                                // Update the conversation UI with the failed message status
                                dispatch_global_message(Message::UpdateConversation(messages_clone));
                            }
                        })));
                    }
                }
            }
        },
       
        Message::UpdateThreadTitle(thread_id, title) => {
            // Update the thread title optimistically
            if let Some(thread) = state.threads.get_mut(&thread_id) {
                thread.title = title.clone();
            }
           
            // Collect data for UI updates
            let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            let title_clone = title.clone();
            let thread_id_clone = thread_id;
            
            // Add UI update command
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(_document) = web_sys::window().expect("no global window exists").document() {
                    // Update thread list UI
                    dispatch_global_message(Message::UpdateThreadList(
                        threads.clone(), 
                        current_thread_id, 
                        thread_messages.clone()
                    ));
                    
                    // Also update the thread title in the header if this is the current thread
                    if current_thread_id == Some(thread_id_clone) {
                        dispatch_global_message(Message::UpdateThreadTitleUI(title_clone.clone()));
                    }
                }
            })));
           
            // Add network update command
            commands.push(Command::UpdateThreadTitle {
                thread_id,
                title,
            });
        },
       
        Message::DeleteThread(thread_id) => {
            // Delete the thread optimistically
            state.threads.remove(&thread_id);
            state.thread_messages.remove(&thread_id);
           
            // If this was the current thread, clear the current thread
            if state.current_thread_id == Some(thread_id) {
                state.current_thread_id = None;
            }
           
            // Collect thread data for UI updates
            let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            
            // Create a new closure that follows the message-passing pattern
            let threads_clone = threads.clone();
            let thread_messages_clone = thread_messages.clone();
            
            // Store updates to be executed after the borrow is released
            state.pending_ui_updates = Some(Box::new(move || {
                // Update the UI using message passing instead of direct function calls
                if let Some(_document) = web_sys::window().expect("no global window exists").document() {
                    // Use message dispatch for UI updates
                    dispatch_global_message(Message::UpdateThreadList(threads_clone, current_thread_id, thread_messages_clone));
                    
                    // For the conversation, we'll clear it if this thread was selected
                    if current_thread_id.is_none() {
                        dispatch_global_message(Message::UpdateConversation(Vec::new()));
                    }
                }
            }));
            
            // Send the delete request to the backend
            let thread_id_clone = thread_id;
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::delete_thread(thread_id_clone).await {
                    Ok(_) => {
                        // No need for a message here, we already updated the UI optimistically
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to delete thread: {:?}", e).into());
                    }
                }
            });
        },
       
        // Navigation messages
        Message::NavigateToChatView(agent_id) => {
            // 1. Pure state updates first
            state.active_view = crate::storage::ActiveView::ChatView;
            state.is_chat_loading = true;
            state.current_agent_id = Some(agent_id);
            
            // 2. Collect data needed for side effects
            let agent_id_for_effects = agent_id;
            
            // 3. Schedule side effects to run after state mutation is complete
            state.pending_ui_updates = Some(Box::new(move || {
                // UI updates
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::chat_view::setup_chat_view(&document);
                    let _ = crate::components::chat_view::show_chat_view(&document, agent_id_for_effects);
                }
                
                // Data fetching - use API directly instead of dispatching
                wasm_bindgen_futures::spawn_local(async move {
                    // Pass agent_id as Some(agent_id) to match the API expectation
                    match crate::network::api_client::ApiClient::get_threads(Some(agent_id_for_effects)).await {
                        Ok(response) => {
                            match serde_json::from_str::<Vec<ApiThread>>(&response) {
                                Ok(threads) => {
                                    // Single dispatch after data is ready
                                    dispatch_global_message(Message::ThreadsLoaded(threads));
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to parse threads: {:?}", e).into());
                                    dispatch_global_message(Message::UpdateLoadingState(false));
                                }
                            }
                        },
                        Err(e) => {
                            web_sys::console::error_1(&format!("Failed to load threads: {:?}", e).into());
                            dispatch_global_message(Message::UpdateLoadingState(false));
                        }
                    }
                });
            }));
        },
       
        Message::NavigateToThreadView(thread_id) => {
            // Set the current thread and navigate to chat view
            state.current_thread_id = Some(thread_id);
            
            // Get the agent ID from the thread
            if let Some(thread) = state.threads.get(&thread_id) {
                let agent_id = thread.agent_id;
                crate::state::dispatch_global_message(Message::NavigateToChatView(agent_id));
            }
        },
       
        Message::NavigateToDashboard => {
            // Set the active view to Dashboard
            state.active_view = crate::storage::ActiveView::Dashboard;
           
            // Instead of directly rendering the view,
            // which would cause a nested borrow, use pending_ui_updates
            state.pending_ui_updates = Some(Box::new(move || {
                if let Some(document) = web_sys::window().unwrap().document() {
                    // First hide the chat view container
                    if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
                        let _ = chat_container.set_attribute("style", "display: none;");
                    }
                    
                    if let Err(e) = crate::views::render_active_view_by_type(&crate::storage::ActiveView::Dashboard, &document) {
                        web_sys::console::error_1(&format!("Failed to render dashboard: {:?}", e).into());
                    }
                }
            }));
        },

        Message::LoadAgentInfo(agent_id) => {
            // Start an async task to load agent info
            wasm_bindgen_futures::spawn_local(async move {
                let result = crate::network::api_client::ApiClient::get_agent(agent_id)
                    .await
                    .and_then(|response| {
                        serde_json::from_str::<ApiAgent>(&response)
                            .map(Box::new)
                            .map_err(|e| e.to_string().into())
                    });

                match result {
                    Ok(boxed_agent) => {
                        crate::state::dispatch_global_message(Message::AgentInfoLoaded(boxed_agent));
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to load/parse agent info: {:?}", e).into());
                    }
                }
            });
        },

        Message::AgentInfoLoaded(agent_box) => {
            // Data is already Box<ApiAgent>
            let agent = *agent_box; // Deref the box
            web_sys::console::log_1(&format!("Update: Handling AgentInfoLoaded: {:?}", agent).into()); // Use {:?}
            if state.active_view == crate::storage::ActiveView::ChatView {
                if let Some(agent_id) = agent.id {
                    state.agents.insert(agent_id, agent.clone());
                    // Trigger UI update for agent info display implicitly via state change
                } else {
                    web_sys::console::error_1(&"AgentInfoLoaded message missing agent ID".into());
                }
            } else {
                web_sys::console::warn_1(&"Received AgentInfoLoaded outside of ChatView".into());
            }
        },

        Message::RequestNewThread => {
            // Try to get agent ID from current thread, else from current_agent_id
            let agent_id_opt = state.current_thread_id
                .and_then(|thread_id| state.threads.get(&thread_id))
                .map(|thread| thread.agent_id)
                .or(state.current_agent_id);
            
            // Log for debugging
            web_sys::console::log_1(&format!("RequestNewThread - agent_id: {:?}", agent_id_opt).into());
           
            // If we have an agent ID, return a command to create a thread
            if let Some(agent_id) = agent_id_opt {
                let title = DEFAULT_THREAD_TITLE.to_string();
                web_sys::console::log_1(&format!("Creating new thread for agent: {}", agent_id).into());
                commands.push(Command::SendMessage(Message::CreateThread(agent_id, title)));
            } else {
                web_sys::console::error_1(&"Cannot create thread: No agent selected".into());
            }
        },

        Message::RequestSendMessage(content) => {
            // Store thread_id locally
            let thread_id_opt = state.current_thread_id;
            
            // Return a command instead of using pending_ui_updates
            if let Some(thread_id) = thread_id_opt {
                let content_clone = content.clone();
                commands.push(Command::SendMessage(Message::SendThreadMessage(thread_id, content_clone)));
            }
        },

        Message::RequestUpdateThreadTitle(title) => {
            // Store thread_id locally before ending the borrow
            let thread_id_opt = state.current_thread_id;
            
            // Instead of using pending_ui_updates, return a command
            if let Some(thread_id) = thread_id_opt {
                let title_clone = title.clone();
                commands.push(Command::SendMessage(Message::UpdateThreadTitle(thread_id, title_clone)));
            }
        },

        Message::UpdateThreadList(threads, current_thread_id, thread_messages) => {
            // Update the UI with the provided thread data
            if let Some(document) = web_sys::window().expect("no global window exists").document() {
                // Use update_thread_list_ui directly since we already have the data
                let _ = update_thread_list_ui(&document, &threads, current_thread_id, &thread_messages);
            }
        },

        Message::UpdateConversation(messages) => {
            // Update the UI with the provided conversation messages
            if let Some(document) = web_sys::window().unwrap().document() {
                let _ = update_conversation_ui(&document, &messages);
            }
        },
        
        Message::UpdateThreadTitleUI(title) => {
            // Update the UI with the provided thread title using a command
            let title_clone = title.clone();
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window().expect("no global window exists").document() {
                    let _ = crate::components::chat_view::update_thread_title_with_data(&document, &title_clone);
                }
            })));
        },

        Message::RequestThreadTitleUpdate => {
            // Get the current thread title from state
            let current_title = state.current_thread_id
                .and_then(|thread_id| state.threads.get(&thread_id))
                .map(|thread| thread.title.clone())
                .unwrap_or_default();
            
            // Store the title for later use
            let title_to_update = current_title.clone();
            
            // Schedule UI update for after this function completes
            state.pending_ui_updates = Some(Box::new(move || {
                // Dispatch a message to update the thread title UI
                dispatch_global_message(Message::UpdateThreadTitleUI(title_to_update));
            }));
        },

        Message::UpdateLoadingState(is_loading) => {
            state.is_chat_loading = is_loading;
            
            // Update the UI
            if let Some(document) = web_sys::window().expect("no global window exists").document() {
                let _ = crate::components::chat_view::update_loading_state(&document, is_loading);
            }
        },

        // --- WebSocket Event Handlers ---
        Message::ReceiveNewMessage(message) => {
            // Get thread_id directly (it's guaranteed to be u32 based on model)
            let thread_id = message.thread_id;

            // Get existing messages or create new vec
            let messages = state.thread_messages.entry(thread_id).or_default();
            messages.push(message);

            // If this is the current thread, update the conversation UI
            if state.current_thread_id == Some(thread_id) {
                let messages_clone = messages.clone();
                state.pending_ui_updates = Some(Box::new(move || {
                    dispatch_global_message(Message::UpdateConversation(messages_clone));
                }));
            }
        },

        Message::ReceiveThreadUpdate { thread_id, title } => {
            // Update thread title if we have this thread
            if let Some(thread) = state.threads.get_mut(&thread_id) {
                if let Some(new_title) = title {
                    thread.title = new_title;
                }

                // If this is the current thread, update the UI
                if state.current_thread_id == Some(thread_id) {
                    let title_clone = thread.title.clone();
                    state.pending_ui_updates = Some(Box::new(move || {
                        dispatch_global_message(Message::UpdateThreadTitleUI(title_clone));
                    }));
                }
            }
        },

        Message::ReceiveStreamStart(thread_id) => {
            // Mark thread as streaming in local state
            state.streaming_threads.insert(thread_id);

            // Create a new message entry for the assistant's response
            let now = chrono::Utc::now().to_rfc3339();
            let assistant_message = ApiThreadMessage {
                id: None, // ID might be set later or not needed for streaming display
                thread_id,
                role: "assistant".to_string(),
                content: "".to_string(), // Start with empty content
                created_at: Some(now),
            };

            // Add the new message to the state
            let messages = state.thread_messages.entry(thread_id).or_default();
            messages.push(assistant_message);

            // If this is the current thread, trigger an immediate UI update 
            // to show the new (empty) assistant message bubble.
            if state.current_thread_id == Some(thread_id) {
                let messages_clone = messages.clone();
                state.pending_ui_updates = Some(Box::new(move || {
                    dispatch_global_message(Message::UpdateConversation(messages_clone));
                }));
            }

            web_sys::console::log_1(&format!("Stream started for thread {}: Created empty assistant message.", thread_id).into());
        },

        Message::ReceiveStreamChunk { thread_id, content } => {
            // Append chunk to the last message if it's for the current thread
            if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                if let Some(last_message) = messages.last_mut() {
                    last_message.content.push_str(&content);

                    // If this is the current thread, update the conversation UI
                    if state.current_thread_id == Some(thread_id) {
                        let messages_clone = messages.clone();
                        state.pending_ui_updates = Some(Box::new(move || {
                            dispatch_global_message(Message::UpdateConversation(messages_clone));
                        }));
                    }
                }
            }
        },

        Message::ReceiveStreamEnd(thread_id) => {
            // Mark thread as no longer streaming in local state
            state.streaming_threads.remove(&thread_id);

            // Find the last user message and update its status (e.g., mark as completed)
            // We assume the stream ending means the corresponding user message is processed.
            if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                if let Some(last_user_message) = messages.iter_mut().filter(|msg| msg.role == "user").last() {
                    // Mark completion by setting the temporary ID back to None
                    // This will stop the UI from showing "Sending..." or "pending" styles
                    last_user_message.id = None;
                    web_sys::console::log_1(&format!("Stream ended: Set last user message ID to None for thread {}.", thread_id).into());
                }

                // Trigger UI update for the conversation to reflect the change
                if state.current_thread_id == Some(thread_id) {
                    let messages_clone = messages.clone();
                    state.pending_ui_updates = Some(Box::new(move || {
                        dispatch_global_message(Message::UpdateConversation(messages_clone));
                    }));
                }
            }

            web_sys::console::log_1(&format!("Stream ended for thread {}.", thread_id).into());
        },

        // --- NEW WebSocket Event Handlers ---
        Message::ReceiveAgentUpdate(agent_data) => {
            web_sys::console::log_1(&format!("Update handler: Received agent update: {:?}", agent_data).into());
            // TODO: Update agent list/details in AppState if needed
            // state.agents.insert(agent_data.id as u32, agent_data.into()); // Example update
            needs_refresh = true; // Assume agent list UI might need refresh
        },
        Message::ReceiveAgentDelete(agent_id) => {
            web_sys::console::log_1(&format!("Update handler: Received agent delete: {}", agent_id).into());
            // TODO: Remove agent from AppState if needed
            // state.agents.remove(&(agent_id as u32)); // Example removal
            needs_refresh = true; // Assume agent list UI might need refresh
        },
        Message::ReceiveThreadHistory(messages) => {
            let system_messages_count = messages.iter().filter(|msg| msg.role == "system").count();
            if system_messages_count > 0 {
                web_sys::console::log_1(&format!("Thread history contains {} system messages which won't be displayed in the chat UI", system_messages_count).into());
            }
            web_sys::console::log_1(&format!("Update handler: Received thread history ({} messages, {} displayable)", 
                messages.len(), messages.len() - system_messages_count).into());
            
            // Use the correct field name: current_thread_id
            if let Some(active_thread_id) = state.current_thread_id {
                // Store the received history messages in the correct cache: thread_messages
                // Clone messages here before the insert
                let messages_clone_for_dispatch = messages.clone(); 
                state.thread_messages.insert(active_thread_id, messages);
                
                // Dispatch a message to update the UI instead of calling render directly
                // This keeps the update flow consistent
                state.pending_ui_updates = Some(Box::new(move || {
                    dispatch_global_message(Message::UpdateConversation(messages_clone_for_dispatch));
                }));

                needs_refresh = false; // UI update handled by UpdateConversation
            } else {
                web_sys::console::warn_1(&"Received thread history but no active thread selected in state.".into());
                needs_refresh = false;
            }
        },
       
        // --- New Agent Deletion Flow ---
        Message::RequestAgentDeletion { agent_id } => {
            commands.push(Command::DeleteAgentApi { agent_id });
        },
        
        Message::DeleteAgentApi { agent_id } => {
            // Just delegate to the Command that handles the API call
            commands.push(Command::DeleteAgentApi { agent_id });
        },
        
        Message::AgentDeletionSuccess { agent_id } => {
            // Remove agent from agents map
            state.agents.remove(&agent_id);
            
            // Remove any nodes associated with this agent
            state.nodes.retain(|_, node| {
                if let Some(node_agent_id) = node.agent_id {
                    node_agent_id != agent_id
                } else {
                    true
                }
            });
            
            state.state_modified = true;
            
            // Add a command to refresh the agents list after state is updated
            commands.push(Command::SendMessage(Message::RefreshAgentsFromAPI));
        },
        
        Message::AgentDeletionFailure { agent_id, error } => {
            web_sys::console::error_1(&format!("Update: Received AgentDeletionFailure for {}: {}", agent_id, error).into());
            // Optionally, update UI to show error message
            // For now, just log the error
            needs_refresh = false; // No state change, no refresh needed
        },
        // --- End New Agent Deletion Flow ---

        // Model management
        Message::SetAvailableModels { models, default_model_id } => {
            state.available_models = models;
            state.selected_model = default_model_id;
            state.state_modified = true;
        },

        Message::RequestCreateAgent { name, system_instructions, task_instructions } => {
            // Use the selected model from state
            let model = state.selected_model.clone();
            let agent_payload = serde_json::json!({
                "name": name,
                "system_instructions": system_instructions,
                "task_instructions": task_instructions,
                "model": model
            });
            let agent_data = agent_payload.to_string();
            commands.push(Command::NetworkCall {
                endpoint: "/api/agents".to_string(),
                method: "POST".to_string(),
                body: Some(agent_data),
                on_success: Box::new(Message::RefreshAgentsFromAPI),
                on_error: Box::new(Message::RefreshAgentsFromAPI), // Could add error handling message
            });
            needs_refresh = false;
        },

        Message::RequestThreadListUpdate(agent_id) => {
            // Filter threads for the given agent_id
            let threads: Vec<ApiThread> = state.threads.values()
                .filter(|t| t.agent_id == agent_id)
                .cloned()
                .collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            commands.push(Command::UpdateUI(Box::new(move || {
                dispatch_global_message(Message::UpdateThreadList(
                    threads,
                    current_thread_id,
                    thread_messages,
                ));
            })));
        },

        // Force re-render of Dashboard when active
        Message::RefreshDashboard => {
            // Schedule UI update outside the current mutable borrow scope
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        // Step 1: read active view with an immutable borrow and drop it.
                        let is_dashboard = crate::state::APP_STATE.with(|state_cell| {
                            state_cell.borrow().active_view == crate::storage::ActiveView::Dashboard
                        });

                        if is_dashboard {
                            // Refresh only the dashboard to avoid borrowing state mutably while inside refresh.
                            if let Err(e) = crate::components::dashboard::refresh_dashboard(&document) {
                                web_sys::console::error_1(&format!("Failed to refresh dashboard: {:?}", e).into());
                            }
                        }
                    }
                }
            })));

            needs_refresh = false;
        },

        Message::RetryAgentTask { agent_id } => {
            // Optimistically update the agent status in the state
            if let Some(agent) = state.agents.get_mut(&agent_id) {
                agent.status = Some("running".to_string());
            }
            
            // Create command to call the API
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/task", agent_id),
                method: "POST".to_string(),
                body: None,
                on_success: Box::new(Message::RefreshAgentsFromAPI),
                on_error: Box::new(Message::RefreshAgentsFromAPI),
            });
            
            needs_refresh = true;
        },
        
        Message::DismissAgentError { agent_id } => {
            // Optimistically clear the error in the state
            if let Some(agent) = state.agents.get_mut(&agent_id) {
                agent.last_error = None;
            }
            
            // Create payload for API update
            let payload = serde_json::json!({"last_error": null}).to_string();
            
            // Create command to update the agent via API
            commands.push(Command::UpdateAgent {
                agent_id,
                payload,
                on_success: Box::new(Message::RefreshAgentsFromAPI),
                on_error: Box::new(Message::RefreshAgentsFromAPI),
            });
            
            needs_refresh = true;
        },
    }

    // For now, if needs_refresh is true, add a NoOp command
    // We'll replace this with proper UI refresh commands later
    if needs_refresh {
        commands.push(Command::NoOp);
    }

    commands
}

// Update thread list UI
pub fn update_thread_list(document: &Document) -> Result<(), JsValue> {
    APP_STATE.with(|state| {
        let state = state.borrow();
        let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
        let current_thread_id = state.current_thread_id;
        let thread_messages = state.thread_messages.clone();
        update_thread_list_ui(document, &threads, current_thread_id, &thread_messages)
    })
}

// A version of update_thread_list that accepts data directly instead of accessing APP_STATE
pub fn update_thread_list_with_data(
    document: &Document,
    threads: &[ApiThread],
    current_thread_id: Option<u32>,
    thread_messages: &HashMap<u32, Vec<ApiThreadMessage>>
) -> Result<(), JsValue> {
    update_thread_list_ui(document, threads, current_thread_id, thread_messages)
}

// Update conversation UI
pub fn update_conversation(document: &Document) -> Result<(), JsValue> {
    APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(thread_id) = state.current_thread_id {
            if let Some(messages) = state.thread_messages.get(&thread_id) {
                return update_conversation_ui(document, messages);
            }
        }
        Ok(())
    })
}