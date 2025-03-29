// frontend/src/update.rs
//
use crate::messages::Message;
use crate::state::{AppState, APP_STATE, dispatch_global_message};
use crate::models::{NodeType, ApiThread, ApiThreadMessage};
use crate::constants::{
    DEFAULT_NODE_WIDTH,
    DEFAULT_NODE_HEIGHT,
    DEFAULT_AGENT_NODE_COLOR,
};
use web_sys::Document;
use wasm_bindgen::JsValue;
use std::collections::HashMap;
use crate::components::chat_view::{update_thread_list_ui, update_conversation_ui};

// Thread title constants
pub const DEFAULT_THREAD_TITLE: &str = "Default Thread";
pub const NEW_THREAD_TITLE: &str = "New Thread";

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
           
            // IMPORTANT: We intentionally don't set state_modified = true here
            // This prevents the auto-save mechanism from making redundant API calls
            
            // NOTE: We intentionally don't create a thread here.
            // The backend automatically creates a thread when an agent is created,
            // and we'll receive a thread_created WebSocket message.
        },
       
        Message::EditAgent(agent_id) => {
            // agent_id will be in the format "agent-{numeric_id}"
            state.selected_node_id = Some(agent_id.clone());
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
       
        Message::SaveAgentDetails { name, system_instructions, task_instructions: _task_instructions, model } => {
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
                       
                        // Create API update payload
                        let update_payload = format!(
                            r#"{{
                                "name": "{}",
                                "system_instructions": "{}",
                                "model": "{}"
                            }}"#,
                            agent.name,
                            agent.system_instructions.clone().unwrap_or_default(),
                            model
                        );
                       
                        // Queue the API call
                        state.pending_network_call = Some((
                            update_payload,
                            format!("agent-update-{}", id)
                        ));
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
                            config: None,
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
                        config: None,
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
            web_sys::console::log_1(&"Refreshing agents from API".into());
           
            // Clone whatever data we need from state before leaving the borrow
            let current_view = state.active_view.clone();
           
            // Spawn an async operation to fetch agents
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::ApiClient::get_agents().await {
                    Ok(agents_json) => {
                        if let Ok(agents) = serde_json::from_str::<Vec<crate::models::ApiAgent>>(&agents_json) {
                            web_sys::console::log_1(&format!("Refreshed {} agents from API", agents.len()).into());
                           
                            // Update the agents in the global APP_STATE using message dispatch
                            crate::state::APP_STATE.with(|state_ref| {
                                let mut state = state_ref.borrow_mut();
                                state.agents.clear();
                               
                                // Add each agent to the HashMap
                                for agent in &agents {
                                    if let Some(id) = agent.id {
                                        state.agents.insert(id, agent.clone());
                                    }
                                }
                               
                                // Set active view back to what it was
                                state.active_view = current_view;
                            });
                           
                            // Refresh the UI
                            if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                                web_sys::console::error_1(&format!("Failed to refresh UI: {:?}", e).into());
                            }
                        }
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Error refreshing agents: {:?}", e).into());
                    }
                }
            });
        },
       
        // Thread-related messages
        Message::LoadThreads(agent_id) => {
            state.is_chat_loading = true;
           
            // Start an async task to load threads
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::get_threads(Some(agent_id)).await {
                    Ok(response) => {
                        // Dispatch a message with the loaded threads
                        crate::state::dispatch_global_message(Message::ThreadsLoaded(response));
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to load threads: {:?}", e).into());
                        // Update loading state
                        crate::state::APP_STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            state.is_chat_loading = false;
                        });
                    }
                }
            });
        },
       
        Message::ThreadsLoaded(response) => {
            state.is_chat_loading = false;
           
            // Parse the threads from the response
            if let Ok(threads_value) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(threads_array) = threads_value.get("threads").and_then(|v| v.as_array()) {
                    // Clear existing threads for this agent
                    let agent_id = threads_array.first()
                        .and_then(|t| t.get("agent_id"))
                        .and_then(|a| a.as_u64())
                        .map(|a| a as u32);
                   
                    if let Some(agent_id) = agent_id {
                        state.threads.retain(|_, thread| thread.agent_id != agent_id);
                    }
                   
                    // Parse and store the threads
                    for thread_value in threads_array {
                        if let Ok(thread) = serde_json::from_value::<crate::models::ApiThread>(thread_value.clone()) {
                            if let Some(thread_id) = thread.id {
                                state.threads.insert(thread_id, thread.clone());
                               
                                // If we don't have a selected thread yet, select this one
                                if state.current_thread_id.is_none() {
                                    state.current_thread_id = Some(thread_id);
                                   
                                    // IMPORTANT: Don't dispatch here, store for later
                                    // We'll dispatch this after releasing the borrow
                                    let load_messages_for_thread_id = thread_id;
                                    state.pending_ui_updates = Some(Box::new(move || {
                                        dispatch_global_message(Message::LoadThreadMessages(load_messages_for_thread_id));
                                    }));
                                }
                            }
                        }
                    }
                   
                    // Get thread title and thread list data before releasing the borrow
                    let current_title = state.current_thread_id
                        .and_then(|thread_id| state.threads.get(&thread_id))
                        .map(|thread| thread.title.clone());
                    
                    let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
                    let current_thread_id = state.current_thread_id;
                    let thread_messages = state.thread_messages.clone();
                   
                    // Store UI update actions to run after the update() function completes
                    // This ensures we don't try to borrow APP_STATE again while it's already borrowed
                    let current_title_clone = current_title.clone();
                    let threads_clone = threads.clone();
                    let thread_messages_clone = thread_messages.clone();
                    
                    // Add UI updates to the pending updates queue
                    let existing_updates = state.pending_ui_updates.take();
                    state.pending_ui_updates = Some(Box::new(move || {
                        // First run any existing updates
                        if let Some(existing_fn) = existing_updates {
                            existing_fn();
                        }
                        
                        // Now run our new updates
                        if let Some(title) = current_title_clone {
                            dispatch_global_message(Message::UpdateThreadTitleUI(title));
                        }
                        
                        dispatch_global_message(Message::UpdateThreadList(
                            threads_clone,
                            current_thread_id,
                            thread_messages_clone
                        ));
                    }));
                }
            }
        },
       
        Message::CreateThread(agent_id, title) => {
            // Log for debugging
            web_sys::console::log_1(&format!("Creating thread for agent {} with title: {}", agent_id, title).into());
            
            // Create a new thread
            wasm_bindgen_futures::spawn_local(async move {
                web_sys::console::log_1(&format!("Making API call to create thread for agent: {}", agent_id).into());
                match crate::network::api_client::ApiClient::create_thread(agent_id, &title).await {
                    Ok(response) => {
                        web_sys::console::log_1(&format!("Thread created successfully: {}", response).into());
                        // Dispatch a message with the created thread
                        crate::state::dispatch_global_message(Message::ThreadCreated(response));
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to create thread: {:?}", e).into());
                    }
                }
            });
        },
       
        Message::ThreadCreated(response) => {
            // Log the raw response for debugging
            web_sys::console::log_1(&format!("Thread created response: {}", response).into());
            
            // Parse the created thread from the response
            if let Ok(thread_value) = serde_json::from_str::<serde_json::Value>(&response) {
                web_sys::console::log_1(&format!("Parsed thread value: {:?}", thread_value).into());
                
                if let Ok(thread) = serde_json::from_value::<crate::models::ApiThread>(thread_value.clone()) {
                    web_sys::console::log_1(&format!("Thread deserialized: id={:?}, title={:?}", thread.id, thread.title).into());
                    
                    if let Some(thread_id) = thread.id {
                        // Store the thread (clone it first)
                        state.threads.insert(thread_id, thread.clone());
                       
                        // Select the new thread and close any previous WebSocket
                        state.current_thread_id = Some(thread_id);
                       
                        // Collect data for UI updates
                        let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
                        let current_thread_id = state.current_thread_id;
                        let thread_messages = state.thread_messages.clone();
                        
                        // Get the thread title to update the header
                        let thread_title = thread.title.clone();
                       
                        // Use pending_ui_updates to avoid nested borrows
                        // Clone the data for the closure
                        let threads_clone = threads.clone();
                        let thread_messages_clone = thread_messages.clone();
                        let thread_id_clone = thread_id;
                        let thread_title_clone = thread_title.clone();
                        
                        // Store for updates to be executed after the borrow is released
                        state.pending_ui_updates = Some(Box::new(move || {
                            web_sys::console::log_1(&format!("Now updating UI with new thread: {}", thread_id).into());
                            
                            // Update thread list UI
                            dispatch_global_message(Message::UpdateThreadList(
                                threads_clone,
                                current_thread_id,
                                thread_messages_clone
                            ));
                            
                            // Update the thread title in the header
                            dispatch_global_message(Message::UpdateThreadTitleUI(thread_title_clone));
                            
                            // Set up WebSocket for the new thread
                            let _ = crate::network::ws_client::setup_thread_websocket(thread_id_clone);
                        }));
                    } else {
                        web_sys::console::error_1(&"Thread created but ID is missing".into());
                    }
                } else {
                    web_sys::console::error_1(&format!("Failed to deserialize thread from JSON: {}", response).into());
                }
            } else {
                web_sys::console::error_1(&format!("Invalid JSON in thread response: {}", response).into());
            }
        },
       
        Message::SelectThread(thread_id) => {
            // Set the current thread
            state.current_thread_id = Some(thread_id);
           
            // Check if we need to load messages, but don't dispatch while borrowed
            let needs_to_load_messages = !state.thread_messages.contains_key(&thread_id);
           
            // Collect the data we need before ending the borrow
            let threads: Vec<ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            
            // Get the thread title to update the header
            let thread_title = state.threads.get(&thread_id)
                .map(|thread| thread.title.clone())
                .unwrap_or_default();
            
            // Get conversation messages if they exist
            let conversation_messages = state.thread_messages.get(&thread_id)
                .cloned()
                .unwrap_or_default();
            
            // Setup WebSocket for the thread after we return
            let thread_id_clone = thread_id;
            
            // These operations will be performed by the caller after this borrow ends
            state.pending_ui_updates = Some(Box::new(move || {
                // After the borrow ends, these actions will be executed
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    dispatch_global_message(Message::UpdateThreadList(threads, current_thread_id, thread_messages));
                    dispatch_global_message(Message::UpdateConversation(conversation_messages));
                    dispatch_global_message(Message::UpdateThreadTitleUI(thread_title));
                }
                
                // Load messages if needed
                if needs_to_load_messages {
                    dispatch_global_message(Message::LoadThreadMessages(thread_id));
                }
                
                // Setup WebSocket for the thread (existing connections will be closed in setup_thread_websocket)
                let _ = crate::network::ws_client::setup_thread_websocket(thread_id_clone);
            }));
        },
       
        Message::LoadThreadMessages(thread_id) => {
            state.is_chat_loading = true;
           
            // Start an async task to load messages
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::get_thread_messages(thread_id, 0, 100).await {
                    Ok(response) => {
                        // Dispatch a message with the loaded messages
                        crate::state::dispatch_global_message(Message::ThreadMessagesLoaded(response));
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to load thread messages: {:?}", e).into());
                        // Update loading state
                        crate::state::APP_STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            state.is_chat_loading = false;
                        });
                    }
                }
            });
        },
       
        Message::ThreadMessagesLoaded(response) => {
            state.is_chat_loading = false;
           
            // Variables to store data after we drop the borrow
            let mut _conversation_messages = Vec::new();
            let mut needs_ui_update = false;
           
            // Parse the messages from the response
            if let Ok(messages_value) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(messages_array) = messages_value.get("messages").and_then(|v| v.as_array()) {
                    // Get the thread ID from the first message
                    let thread_id = messages_array.first()
                        .and_then(|m| m.get("thread_id"))
                        .and_then(|t| t.as_u64())
                        .map(|t| t as u32);
                   
                    if let Some(thread_id) = thread_id {
                        // Parse and store the messages
                        let mut thread_messages = Vec::new();
                       
                        for message_value in messages_array {
                            if let Ok(message) = serde_json::from_value::<crate::models::ApiThreadMessage>(message_value.clone()) {
                                thread_messages.push(message);
                            }
                        }
                       
                        // Sort messages by creation time (oldest first)
                        thread_messages.sort_by(|a, b| {
                            a.created_at.as_ref().unwrap_or(&"".to_string())
                                .cmp(b.created_at.as_ref().unwrap_or(&"".to_string()))
                        });
                       
                        // Store the messages
                        state.thread_messages.insert(thread_id, thread_messages.clone());
                       
                        // Check if we need to update the UI
                        if state.current_thread_id == Some(thread_id) {
                            needs_ui_update = true;
                            _conversation_messages = thread_messages;
                        }
                    }
                }
            }
           
            // Drop the borrow before UI updates
            {
                // End of borrow scope
            }
           
            // Update UI if needed
            if needs_ui_update {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    dispatch_global_message(Message::UpdateConversation(_conversation_messages));
                }
            }
        },
       
        Message::SendThreadMessage(thread_id, content) => {
            // Use the modular thread handler instead of inline logic
            state.pending_ui_updates = Some(
                crate::thread_handlers::handle_send_thread_message(
                    thread_id, 
                    content,
                    &mut state.thread_messages,
                    &state.threads,
                    state.current_thread_id
                )
            );
        },
       
        Message::ThreadMessageSent(response, client_id) => {
            // Use the modular thread handler
            if let Some(update_fn) = crate::thread_handlers::handle_thread_message_sent(
                response,
                client_id,
                &mut state.thread_messages
            ) {
                state.pending_ui_updates = Some(update_fn);
            }
        },
        
        Message::ThreadMessageFailed(thread_id, client_id) => {
            // Use the modular thread handler
            if let Some(update_fn) = crate::thread_handlers::handle_thread_message_failed(
                thread_id,
                client_id,
                &mut state.thread_messages
            ) {
                state.pending_ui_updates = Some(update_fn);
            }
        },
       
        Message::ThreadMessageReceived(message_str) => {
            // Use the modular thread handler
            if let Some(update_fn) = crate::thread_handlers::handle_thread_message_received(
                message_str,
                &mut state.thread_messages,
                &state.threads,
                state.current_thread_id
            ) {
                state.pending_ui_updates = Some(update_fn);
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
           
            // Drop the borrow before any UI updates
            {
                // End of borrow scope
            }
           
            // Update the UI using the UpdateThreadList message
            if let Some(_document) = web_sys::window().expect("no global window exists").document() {
                dispatch_global_message(Message::UpdateThreadList(threads, current_thread_id, thread_messages));
            }
           
            // Send the update to the backend
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::update_thread(thread_id, &title).await {
                    Ok(_) => {
                        // No need for a message here, we already updated the UI optimistically
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to update thread title: {:?}", e).into());
                    }
                }
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
            // Set the active view to ChatView
            state.active_view = crate::storage::ActiveView::ChatView;
            
            // Check if the agent has any threads
            let needs_default_thread = {
                // Check if this agent has any threads
                let has_threads = state.threads.values().any(|thread| thread.agent_id == agent_id);
                !has_threads
            };
           
            // Get reference to document before leaving this scope
            let document_opt = web_sys::window().and_then(|w| w.document());
            
            // Store agent_id for pending updates after we release the borrow
            let agent_id_for_thread = agent_id;
            
            // Use the pending_ui_updates mechanism to queue the thread creation
            // This avoids recursive borrowing of the state
            if needs_default_thread {
                web_sys::console::log_1(&format!("Will auto-create default thread for agent: {}", agent_id).into());
                
                // Queue the thread creation for after this update completes
                state.pending_ui_updates = Some(Box::new(move || {
                    web_sys::console::log_1(&format!("Now creating default thread for agent: {}", agent_id_for_thread).into());
                    dispatch_global_message(Message::CreateThread(agent_id_for_thread, DEFAULT_THREAD_TITLE.to_string()));
                }));
            } else {
                web_sys::console::log_1(&format!("Agent {} already has threads, skipping auto-creation", agent_id).into());
            }
           
            // End of the state borrow
            {
                // End of borrow scope
            }
            
            // Show the chat view
            if let Some(document) = document_opt {
                let _ = crate::components::chat_view::setup_chat_view(&document);
                let _ = crate::components::chat_view::show_chat_view(&document, agent_id);
            }
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
                    if let Err(e) = crate::views::render_active_view_by_type(&crate::storage::ActiveView::Dashboard, &document) {
                        web_sys::console::error_1(&format!("Failed to render dashboard: {:?}", e).into());
                    }
                }
            }));
        },

        Message::LoadAgentInfo(agent_id) => {
            // Start an async task to load agent info
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::get_agent(agent_id).await {
                    Ok(response) => {
                        crate::state::dispatch_global_message(Message::AgentInfoLoaded(response));
                    },
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to load agent info: {:?}", e).into());
                    }
                }
            });
        },

        Message::AgentInfoLoaded(response) => {
            // Parse and update agent info in state
            if let Ok(agent) = serde_json::from_str::<crate::models::ApiAgent>(&response) {
                if let Some(id) = agent.id {
                    state.agents.insert(id, agent);
                }
            }
        },

        Message::RequestNewThread => {
            // Get current agent ID from the current thread
            let agent_id_opt = state.current_thread_id
                .and_then(|thread_id| state.threads.get(&thread_id))
                .map(|thread| thread.agent_id);
            
            // Log for debugging
            web_sys::console::log_1(&format!("RequestNewThread - agent_id: {:?}", agent_id_opt).into());
           
            // Queue the thread creation for after this update completes to avoid nested borrows
            if let Some(agent_id) = agent_id_opt {
                let title = NEW_THREAD_TITLE.to_string();
                let agent_id_clone = agent_id;
                
                // Store the pending update to be executed after the borrow is released
                state.pending_ui_updates = Some(Box::new(move || {
                    web_sys::console::log_1(&format!("Creating new thread for agent: {}", agent_id_clone).into());
                    dispatch_global_message(Message::CreateThread(agent_id_clone, title));
                }));
            } else {
                web_sys::console::error_1(&"Cannot create thread: No agent selected".into());
            }
        },

        Message::RequestSendMessage(content) => {
            // Store thread_id locally
            let thread_id_opt = state.current_thread_id;
            
            // Use pending_ui_updates to avoid nested borrows
            if let Some(thread_id) = thread_id_opt {
                let content_clone = content.clone();
                
                // Store updates to be executed after the borrow is released
                state.pending_ui_updates = Some(Box::new(move || {
                    dispatch_global_message(Message::SendThreadMessage(thread_id, content_clone));
                }));
            }
        },

        Message::RequestUpdateThreadTitle(title) => {
            // Store thread_id locally before ending the borrow
            let thread_id_opt = state.current_thread_id;
            
            // Use pending_ui_updates to avoid nested borrows
            if let Some(thread_id) = thread_id_opt {
                let title_clone = title.clone();
                
                // Store updates to be executed after the borrow is released
                state.pending_ui_updates = Some(Box::new(move || {
                    dispatch_global_message(Message::UpdateThreadTitle(thread_id, title_clone));
                }));
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
            // Update the UI with the provided thread title directly
            if let Some(document) = web_sys::window().expect("no global window exists").document() {
                let _ = crate::components::chat_view::update_thread_title_with_data(&document, &title);
            }
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
    }
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