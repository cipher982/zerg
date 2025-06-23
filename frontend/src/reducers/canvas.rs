//! Canvas/workflow reducer split out from `update.rs`.
//!
//! Handles all canvas, node, edge, drag, zoom, and workflow logic.

use crate::state::AppState;
use crate::messages::{Message, Command};
use crate::models::NodeType;

pub fn update(state: &mut AppState, msg: &Message, cmds: &mut Vec<Command>) -> bool {
    match msg {
        Message::SaveToolConfig { node_id, config } => {
            if let Some(node) = state.nodes.get_mut(node_id) {
                if let crate::models::NodeType::Tool { config: node_config, .. } = &mut node.node_type {
                    *node_config = config.clone();
                    state.state_modified = true;
                    state.mark_dirty();
                }
            }
            true
        }
        Message::UpdateTriggerNodeConfig { node_id, params } => {
            if let Some(node) = state.nodes.get_mut(node_id) {
                if let crate::models::NodeType::Trigger { config, .. } = &mut node.node_type {
                    // Update the params in the trigger config
                    if let serde_json::Value::Object(param_map) = params {
                        config.params.clear();
                        for (key, value) in param_map {
                            config.params.insert(key.clone(), value.clone());
                        }
                    }
                    state.state_modified = true;
                    state.mark_dirty();
                }
            }
            true
        }
        Message::UpdateNodePosition { node_id, x, y } => {
            state.update_node_position(node_id, *x, *y);
            // Always mark dirty for visual updates during dragging
            state.mark_dirty();
            // Only mark state as modified (triggering API saves) if not currently dragging
            // This prevents spam of PATCH requests during drag operations for all node types
            if state.dragging.is_none() {
                state.state_modified = true;
            }
            true
        }
        Message::AddNode { text, x, y, node_type } => {
            if *node_type == NodeType::AgentIdentity && *x == 0.0 && *y == 0.0 {
                let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
                let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };
                let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - 75.0;
                let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - 50.0;
                let node_id = state.add_node(text.clone(), x, y, node_type.clone());
                web_sys::console::log_1(&format!("Created visual node for agent: {}", node_id).into());
            } else {
                let _ = state.add_node(text.clone(), *x, *y, node_type.clone());
            }
            state.state_modified = true;
            true
        }
        Message::AddResponseNode { parent_id, response_text } => {
            let _ = state.add_response_node(parent_id, response_text.clone());
            state.state_modified = true;
            true
        }
        Message::ToggleAutoFit => {
            state.toggle_auto_fit();
            state.state_modified = true;
            true
        }
        Message::CenterView => {
            state.center_view();
            state.state_modified = true;
            true
        }
        Message::ResetView => {
            state.reset_view();
            state.state_modified = true;
            true
        }
        Message::ClearCanvas => {
            state.nodes.clear();
            state.latest_user_input_id = None;
            state.message_id_to_node_id.clear();
            state.viewport_x = 0.0;
            state.viewport_y = 0.0;
            state.zoom_level = 1.0;
            state.auto_fit = true;
            state.state_modified = true;
            true
        }
        Message::ShowToolConfigModal { node_id } => {
            if let Some(node) = state.nodes.get(node_id) {
                if let NodeType::Tool { tool_name, server_name, config, .. } = &node.node_type {
                    let description = state.available_mcp_tools.values()
                        .flat_map(|tools| tools.iter())
                        .find(|t| &t.name == tool_name && &t.server_name == server_name)
                        .and_then(|t| t.description.clone())
                        .unwrap_or_else(|| "No description available.".to_string());

                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let _ = crate::components::tool_config_modal::ToolConfigModal::open(
                                &document,
                                node,
                            );
                        }
                    }
                }
            }
            true
        }
        Message::ShowTriggerConfigModal { node_id } => {
            if let Some(node) = state.nodes.get(node_id) {
                if let NodeType::Trigger { .. } = &node.node_type {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let _ = crate::components::trigger_config_modal::TriggerConfigModal::open(
                                &document,
                                node,
                            );
                        }
                    }
                }
            }
            true
        }
        Message::CanvasNodeClicked { node_id } => {
            if state.connection_mode {
                // Handle connection mode clicks
                if let Some(source_node_id) = &state.connection_source_node {
                    if source_node_id != node_id {
                        // Create connection from source to clicked node
                        cmds.push(Command::SendMessage(Message::CreateConnectionFromSelected {
                            target_node_id: node_id.clone()
                        }));
                    }
                } else {
                    // Select this node as connection source
                    cmds.push(Command::SendMessage(Message::SelectNodeForConnection {
                        node_id: node_id.clone()
                    }));
                }
            } else {
                // Normal click behavior - open agent config
                if let Some(agent_id) = state.nodes.get(node_id).and_then(|n| n.agent_id) {
                    cmds.push(Command::SendMessage(Message::EditAgent(agent_id)));
                }
            }
            true
        }
        Message::MarkCanvasDirty => {
            state.mark_dirty();
            true
        }
        Message::StartDragging { node_id, offset_x, offset_y, start_x, start_y, is_agent: _ } => {
            state.dragging = Some(node_id.clone());
            state.drag_offset_x = *offset_x;
            state.drag_offset_y = *offset_y;
            state.drag_start_x = *start_x;
            state.drag_start_y = *start_y;
            true
        }
        Message::StopDragging => {
            state.dragging = None;
            state.state_modified = true;
            let _ = state.save_if_modified();
            true
        }
        Message::StartCanvasDrag { start_x, start_y } => {
            state.canvas_dragging = true;
            state.drag_start_x = *start_x;
            state.drag_start_y = *start_y;
            state.drag_last_x = *start_x;
            state.drag_last_y = *start_y;
            true
        }
        Message::UpdateCanvasDrag { current_x, current_y } => {
            if state.canvas_dragging {
                let dx = *current_x - state.drag_last_x;
                let dy = *current_y - state.drag_last_y;
                let zoom = state.zoom_level;
                state.viewport_x -= dx / zoom;
                state.viewport_y -= dy / zoom;
                state.drag_last_x = *current_x;
                state.drag_last_y = *current_y;
                state.state_modified = true;
                state.mark_dirty();
            }
            true
        }
        Message::StopCanvasDrag => {
            state.canvas_dragging = false;
            state.state_modified = true;
            let _ = state.save_if_modified();
            true
        }
        Message::ZoomCanvas { new_zoom, viewport_x, viewport_y } => {
            state.zoom_level = *new_zoom;
            state.clamp_zoom();
            state.viewport_x = *viewport_x;
            state.viewport_y = *viewport_y;
            state.state_modified = true;
            state.mark_dirty();
            true
        }
        Message::AddCanvasNode { agent_id, x, y, node_type, text } => {
            let node_id = state.add_node_with_agent(*agent_id, *x, *y, node_type.clone(), text.clone());
            web_sys::console::log_1(&format!("Created new node: {}", node_id).into());
            state.state_modified = true;
            true
        }
        Message::DeleteNode { node_id } => {
            state.nodes.remove(node_id);
            if let Some(workflow_id) = state.current_workflow_id {
                if let Some(workflow) = state.workflows.get_mut(&workflow_id) {
                    workflow.nodes.retain(|n| n.node_id != *node_id);
                    workflow.edges.retain(|edge| edge.from_node_id != *node_id && edge.to_node_id != *node_id);
                }
            }
            state.state_modified = true;
            true
        }
        Message::UpdateNodeText { node_id, text, is_first_chunk } => {
            if let Some(node) = state.nodes.get_mut(node_id) {
                if *is_first_chunk {
                    node.text = text.clone();
                } else {
                    node.text.push_str(text);
                }
                state.resize_node_for_content(node_id);
                state.state_modified = true;
            }
            true
        }
        Message::CompleteNodeResponse { node_id, final_text } => {
            if let Some(node) = state.nodes.get_mut(node_id) {
                if !final_text.is_empty() && node.text != *final_text {
                    node.text = final_text.clone();
                }
                node.color = "#c8e6c9".to_string();
                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.status = Some("idle".to_string());
                    }
                }
                let parent_id = node.parent_id.clone();
                state.state_modified = true;
                state.resize_node_for_content(node_id);
                if let Some(parent_id) = parent_id {
                    if let Some(parent) = state.nodes.get_mut(&parent_id) {
                        parent.color = "#ffecb3".to_string();
                        if let Some(agent_id) = parent.agent_id {
                            if let Some(agent) = state.agents.get_mut(&agent_id) {
                                agent.status = Some("idle".to_string());
                            }
                        }
                    }
                }
            }
            true
        }
        Message::UpdateNodeStatus { node_id, status } => {
            if let Some(node) = state.nodes.get_mut(node_id) {
                use crate::models::NodeExecStatus;

                let (color, exec_status) = match status.as_str() {
                    "running" | "processing" => ("#fcd34d", NodeExecStatus::Running),   // amber-300
                    "success" | "complete" => ("#86efac", NodeExecStatus::Success),     // green-300
                    "failed" | "error" => ("#fca5a5", NodeExecStatus::Failed),        // red-300
                    _ => ("#e0e7ff", NodeExecStatus::Idle),                               // indigo-100
                };

                node.color = color.to_string();
                node.exec_status = Some(exec_status);
                
                // Only update agent status for agent-backed nodes with valid workflow execution statuses
                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        // Only update agent status for legitimate workflow execution status changes
                        // that come from the backend workflow engine, not UI-only updates
                        let should_update_agent_status = match status.as_str() {
                            "running" | "success" | "failed" => true,  // Valid workflow execution statuses
                            _ => false,  // Skip UI-only statuses like "processing", "complete", etc.
                        };
                        
                        if should_update_agent_status {
                            // Map node execution status to valid agent status
                            let agent_status = match status.as_str() {
                                "running" => "running",
                                "success" => "idle",  // Node completed, agent goes back to idle
                                "failed" => "error",
                                _ => return true,  // Should not reach here due to filter above
                            };
                            
                            web_sys::console::log_1(&format!("Updating agent {} status from '{}' to '{}' due to node {} workflow execution", agent_id, agent.status.as_ref().unwrap_or(&"unknown".to_string()), agent_status, node_id).into());
                            
                            agent.status = Some(agent_status.to_string());
                            let update = crate::models::ApiAgentUpdate {
                                name: None,
                                status: Some(agent_status.to_string()),
                                system_instructions: None,
                                task_instructions: None,
                                model: None,
                                schedule: None,
                                config: None,
                                last_error: None,
                            };
                            let agent_id_clone = agent_id;
                            let update_clone = update.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                match serde_json::to_string(&update_clone) {
                                    Ok(json_str) => {
                                        if let Err(e) = crate::network::ApiClient::update_agent(agent_id_clone, &json_str).await {
                                            web_sys::console::error_1(&format!("Failed to update agent: {:?}", e).into());
                                        }
                                    }
                                    Err(e) => {
                                        web_sys::console::error_1(&format!("Failed to serialize agent update: {}", e).into());
                                    }
                                }
                            });
                        } else {
                            web_sys::console::log_1(&format!("Skipping agent status update for node {} with status '{}' (not a workflow execution status)", node_id, status).into());
                        }
                    }
                }
                state.state_modified = true;
                
                // Refresh results panel for node status updates via command
                cmds.push(Command::UpdateUI(Box::new(|| {
                    // Removed: let _ = crate::components::execution_results_panel::refresh_results_panel();
                })));
            }
            true
        }
        Message::AnimationTick => {
            // Only mark dirty if there are actual animations or updates that need rendering
            let needs_animation = !state.running_runs.is_empty() || 
                                  state.connection_drag_active || 
                                  state.dragging.is_some() ||
                                  state.canvas_dragging;
            
            if needs_animation {
                state.mark_dirty();
            }

            if state.dirty {
                state.dirty = false;
                cmds.push(Command::UpdateUI(Box::new(|| {
                    crate::state::APP_STATE.with(|state_rc| {
                        let mut st = state_rc.borrow_mut();
                        crate::canvas::renderer::draw_nodes(&mut st);
                        #[cfg(debug_assertions)] {
                            if let Some(ctx) = st.context.as_ref() {
                                crate::utils::debug::draw_overlay(ctx, &st.debug_ring);
                            }
                        }
                    });
                })));
            }
            let mut duration_changed = false;
            let now_ms = crate::utils::now_ms();
            for run_id in state.running_runs.iter() {
                if let Some((_agent_id, run_list)) = state
                    .agent_runs
                    .iter_mut()
                    .find(|(_aid, list)| list.iter().any(|r| r.id == *run_id))
                {
                    if let Some(run) = run_list.iter_mut().find(|r| r.id == *run_id) {
                        if let Some(start_iso) = &run.started_at {
                            if let Some(start_ms) = crate::utils::parse_iso_ms(start_iso) {
                                let new_duration = now_ms.saturating_sub(start_ms as u64);
                                if run
                                    .duration_ms
                                    .map(|old| new_duration / 1000 != old / 1000)
                                    .unwrap_or(true)
                                {
                                    run.duration_ms = Some(new_duration);
                                    duration_changed = true;
                                }
                            }
                        }
                    }
                }
            }
            for (_id, node) in state.nodes.iter_mut() {
                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get(&agent_id) {
                        if let Some(status_str) = &agent.status {
                            if status_str == "processing" {
                                // Placeholder for future animation effect
                            }
                        }
                    }
                }
            }
            if duration_changed && state.active_view == crate::storage::ActiveView::Dashboard {
                cmds.push(Command::UpdateUI(Box::new(|| {
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            let _ = crate::components::dashboard::refresh_dashboard(&doc);
                        }
                    }
                })));
            }
            if state.state_modified {
                let now = crate::utils::now_ms();
                if now.saturating_sub(state.last_modified_ms) > 400 {
                    cmds.push(Command::SaveState);
                    state.state_modified = false;
                }
            }
            true
        }
        Message::CreateWorkflow { name } => {
            // Don't create local workflow here - let update.rs handle the API call
            // and WorkflowCreated message will update the state properly
            state.creating_workflow = true;
            cmds.push(Command::CreateWorkflowApi { name: name.clone() });

            // Refresh tab bar UI
            cmds.push(Command::UpdateUI(Box::new(|| {
                if let (Some(win),) = (web_sys::window(),) {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::workflow_switcher::refresh(&doc);
                    }
                }
            })));

            true
        }
        Message::SelectWorkflow { workflow_id } => {
            state.current_workflow_id = Some(*workflow_id);
            web_sys::console::log_1(&format!("DEBUG: SelectWorkflow {} - clearing {} nodes", workflow_id, state.nodes.len()).into());
            for (id, node) in &state.nodes {
                web_sys::console::log_1(&format!("DEBUG: Clearing node {} (type: {:?})", id, node.node_type).into());
            }
            state.nodes.clear();
            if let Some(workflow) = state.workflows.get(workflow_id) {
                web_sys::console::log_1(&format!("DEBUG: Repopulating with {} workflow nodes", workflow.nodes.len()).into());
                for node in &workflow.nodes {
                    web_sys::console::log_1(&format!("DEBUG: Restoring node {} (type: {:?})", node.node_id, node.node_type).into());
                    state.nodes.insert(node.node_id.clone(), node.clone());
                }
            }
            state.mark_dirty();
            state.state_modified = true;

            cmds.push(Command::UpdateUI(Box::new(|| {
                if let (Some(win),) = (web_sys::window(),) {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::workflow_switcher::refresh(&doc);
                    }
                }
            })));

            true
        }
        Message::AddEdge { from_node_id, to_node_id, label } => {
            let edge_id = state.add_edge(from_node_id.clone(), to_node_id.clone(), label.clone());
            web_sys::console::log_1(&format!("Created new edge with ID: {}", edge_id).into());
            state.mark_dirty();
            state.state_modified = true;
            true
        }
        Message::GenerateCanvasFromAgents => {
            let mut nodes_created = 0;
            let agents_needing_nodes = state.agents.iter()
                .filter(|(id, _)| !state.agent_id_to_node_id.contains_key(id))
                .map(|(id, agent)| (*id, agent.name.clone()))
                .collect::<Vec<_>>();
            for (i, (agent_id, name)) in agents_needing_nodes.iter().enumerate() {
                let row = i / 3;
                let col = i % 3;
                let x = 100.0 + (col as f64 * 250.0);
                let y = 100.0 + (row as f64 * 150.0);
                state.add_agent_node(*agent_id, name.clone(), x, y);
                nodes_created += 1;
            }
            web_sys::console::log_1(&format!("Created {} nodes for agents without visual representation", nodes_created).into());
            if nodes_created > 0 {
                state.state_modified = true;
                state.mark_dirty();
            }
            true
        }

        // Connection mode handlers
        Message::ToggleConnectionMode => {
            state.connection_mode = !state.connection_mode;
            if !state.connection_mode {
                // Clear selection when exiting connection mode
                state.connection_source_node = None;
                state.selected_node_id = None;
            }
            web_sys::console::log_1(&format!("Connection mode: {}", state.connection_mode).into());
            state.mark_dirty();
            
            // Update button appearance
            cmds.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::update_connection_button(&doc);
                }
            })));
            
            true
        }

        Message::SelectNodeForConnection { node_id } => {
            if state.connection_mode {
                state.connection_source_node = Some(node_id.clone());
                state.selected_node_id = Some(node_id.clone());
                web_sys::console::log_1(&format!("Selected node {} as connection source", node_id).into());
                state.mark_dirty();
            }
            true
        }

        Message::CreateConnectionFromSelected { target_node_id } => {
            if state.connection_mode {
                if let Some(source_node_id) = state.connection_source_node.clone() {
                    if source_node_id != *target_node_id {
                        let edge_id = state.add_edge(source_node_id.clone(), target_node_id.clone(), None);
                        web_sys::console::log_1(&format!("Created connection from {} to {} (edge ID: {})", source_node_id, target_node_id, edge_id).into());
                        state.mark_dirty();
                        state.state_modified = true;
                        
                        // Clear selection after creating connection
                        state.connection_source_node = None;
                        state.selected_node_id = None;
                    } else {
                        web_sys::console::log_1(&"Cannot connect node to itself".into());
                    }
                } else {
                    web_sys::console::log_1(&"No source node selected for connection".into());
                }
            }
            true
        }

        Message::ClearNodeSelection => {
            state.connection_source_node = None;
            state.selected_node_id = None;
            web_sys::console::log_1(&"Cleared node selection".into());
            state.mark_dirty();
            true
        }

        // Connection handle dragging handlers
        Message::StartConnectionDrag { node_id, handle_position, start_x, start_y } => {
            state.connection_drag_active = true;
            state.connection_drag_start = Some((node_id.clone(), handle_position.clone()));
            state.connection_drag_current = Some((*start_x, *start_y));
            web_sys::console::log_1(&format!("Started connection drag from {} handle of node {}", handle_position, node_id).into());
            state.mark_dirty();
            true
        }

        Message::UpdateConnectionDrag { current_x, current_y } => {
            if state.connection_drag_active {
                state.connection_drag_current = Some((*current_x, *current_y));
                state.mark_dirty(); // Repaint to show preview line
            }
            true
        }

        Message::EndConnectionDrag { end_x, end_y } => {
            if state.connection_drag_active {
                // Check if we're dropping on another node handle with validation
                if let Some((source_node_id, source_handle)) = state.connection_drag_start.clone() {
                    // Check if dropped on a specific handle (precise targeting)
                    let mut connection_created = false;
                    for (target_node_id, _) in &state.nodes.clone() {
                        if target_node_id != &source_node_id {
                            if let Some(target_handle) = state.get_handle_at_point(target_node_id, *end_x, *end_y) {
                                // Validate the connection before creating
                                if state.is_valid_connection(&source_handle, &target_handle, &source_node_id, target_node_id) {
                                    let edge_id = state.add_edge(source_node_id.clone(), target_node_id.clone(), None);
                                    web_sys::console::log_1(&format!("Created valid connection from {} ({}) to {} ({}) (edge ID: {})", source_node_id, source_handle, target_node_id, target_handle, edge_id).into());
                                    state.state_modified = true;
                                    connection_created = true;
                                } else {
                                    web_sys::console::log_1(&format!("Invalid connection: {} ({}) to {} ({})", source_node_id, source_handle, target_node_id, target_handle).into());
                                }
                                break;
                            }
                        }
                    }
                    
                    // Fallback: if not dropped on a handle, check if dropped on a node (auto-route to input)
                    if !connection_created {
                        if let Some((target_node_id, _, _)) = state.find_node_at_position(*end_x, *end_y) {
                            if target_node_id != source_node_id {
                                // Auto-route: output -> input only
                                if state.is_valid_connection(&source_handle, "input", &source_node_id, &target_node_id) {
                                    let edge_id = state.add_edge(source_node_id.clone(), target_node_id.clone(), None);
                                    web_sys::console::log_1(&format!("Created auto-routed connection from {} ({}) to {} (input) (edge ID: {})", source_node_id, source_handle, target_node_id, edge_id).into());
                                    state.state_modified = true;
                                } else {
                                    web_sys::console::log_1(&format!("Invalid auto-route connection: {} ({}) to {} (input)", source_node_id, source_handle, target_node_id).into());
                                }
                            } else {
                                web_sys::console::log_1(&"Cannot connect node to itself".into());
                            }
                        }
                    }
                }
                
                // Clear drag state
                state.connection_drag_active = false;
                state.connection_drag_start = None;
                state.connection_drag_current = None;
            }
            state.mark_dirty();
            true
        }

        _ => false,
    }
}
