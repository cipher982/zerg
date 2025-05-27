//! Agent domain reducer: handles all agent CRUD, config, modal, error, run history, debug modal, and MCP integration logic.

use crate::messages::{Message, Command};
use crate::state::AppState;

/// Handles agent-related messages. Returns true if the message was handled.
pub fn update(state: &mut AppState, msg: &Message, commands: &mut Vec<Command>) -> bool {
    match msg {
        Message::CreateAgent(name) => {
            // Create a new agent node at a default position
            let _ = state.add_node(name, 100.0, 100.0, crate::models::NodeType::AgentIdentity);
            state.state_modified = true;
            true
        }
        Message::CreateAgentWithDetails { name, agent_id, system_instructions: _system_instructions, task_instructions: _task_instructions } => {
            // Calculate center position for the new agent
            let viewport_width = if state.canvas_width > 0.0 { state.canvas_width } else { 800.0 };
            let viewport_height = if state.canvas_height > 0.0 { state.canvas_height } else { 600.0 };

            let x = state.viewport_x + (viewport_width / state.zoom_level) / 2.0 - crate::constants::DEFAULT_NODE_WIDTH / 2.0;
            let y = state.viewport_y + (viewport_height / state.zoom_level) / 2.0 - crate::constants::DEFAULT_NODE_HEIGHT / 2.0;

            // Create and add the node via the new helper that keeps the
            // agent<->node mapping up-to-date.
            let node_id = state.add_agent_node(*agent_id, name, x, y);

            // Log the new node ID - make it clear this is a visual node representing an agent
            web_sys::console::log_1(&format!("Created visual node with ID: {} for agent {}", node_id, agent_id).into());

            // Draw the nodes on canvas
            state.mark_dirty();

            // After creating the agent, immediately create a default thread
            commands.push(Command::SendMessage(crate::messages::Message::CreateThread(*agent_id, crate::constants::DEFAULT_THREAD_TITLE.to_string())));

            // IMPORTANT: We intentionally don't set state_modified = true here
            // This prevents the auto-save mechanism from making redundant API calls

            // Note: A default thread will be created by our Message::AgentsRefreshed handler
            // when the agent list is refreshed from the API.
            true
        }
        Message::EditAgent(agent_id) => {
            web_sys::console::log_1(&format!("Update: Handling EditAgent for agent_id: {}", agent_id).into());
            // Quick O(1) lookup via the explicit map
            let node_id_to_select = state.agent_id_to_node_id.get(agent_id).cloned();

            if let Some(node_id) = node_id_to_select {
                // Happy‑path: we already have a visual node for this agent
                web_sys::console::log_1(&format!("Found node_id {} for agent_id {}, selecting it.", node_id, agent_id).into());
                let node_id_cloned = node_id.clone();
                state.selected_node_id = Some(node_id);
                state.state_modified = true;

                // Defer opening of the modal until *after* we return from
                // `update()`.  Doing this via a `Command::UpdateUI` ensures we
                // don't attempt to borrow `APP_STATE` while we are still
                // holding an active mutable borrow (which would panic at
                // runtime).  The closure is executed by the dispatcher once
                // the state borrow has been released, following the same
                // pattern we use for `ToggleView` above.

                commands.push(Command::UpdateUI(Box::new({
                    let _node_id = node_id_cloned;
                    let agent_id = *agent_id;
                    move || {
                        if let (_, Some(document)) = (web_sys::window(), web_sys::window().and_then(|w| w.document())) {
                            if let Err(e) = crate::views::show_agent_modal(agent_id, &document) {
                                web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
                            }
                        }
                    }
                })));
            } else {
                // Fallback: no canvas node yet (e.g., user came from Dashboard)
                // We simply open the modal without selecting a node.

                let agent_id = *agent_id;
                commands.push(Command::UpdateUI(Box::new(move || {
                    if let (_, Some(document)) = (web_sys::window(), web_sys::window().and_then(|w| w.document())) {
                        if let Err(e) = crate::views::show_agent_modal(agent_id, &document) {
                            web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
                        }
                    }
                })));

                // No canvas refresh necessary
            }
            true
        }
        Message::SaveAgentDetails { name, system_instructions, task_instructions, model, schedule } => {
            // Get the current AGENT ID from the modal's data attribute
            let agent_id = if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(modal) = document.get_element_by_id("agent-modal") {
                        // Read "data-agent-id" instead of "data-node-id"
                        modal.get_attribute("data-agent-id")
                             .and_then(|id_str| id_str.parse::<u32>().ok()) // Parse directly to u32
                    } else {
                        web_sys::console::warn_1(&"Modal element not found!".into());
                        None
                    }
                } else { None }
            } else { None };

            // Process the save operation (NO node lookup needed)
            if let Some(id) = agent_id { // Use the agent_id directly
                if let Some(agent) = state.agents.get_mut(&id) {
                    // Update agent properties
                    agent.name = name;
                    agent.system_instructions = Some(system_instructions.clone());
                    agent.task_instructions = Some(task_instructions.clone());
                    agent.model = Some(model.clone());
                    agent.schedule = schedule.clone();

                    // Build update struct
                    use crate::models::ApiAgentUpdate;
                    let api_update = ApiAgentUpdate {
                        name: Some(agent.name.clone()),
                        status: None,
                        system_instructions: Some(agent.system_instructions.clone().unwrap_or_default()),
                        task_instructions: Some(task_instructions.clone()),
                        model: Some(model.clone()),
                        schedule: schedule.clone(),
                        config: None,
                        last_error: None,
                    };

                    // Serialize to JSON
                    let update_payload = serde_json::to_string(&api_update).unwrap_or_else(|_| "{}".to_string());

                    // Return a command to update the agent via API
                    commands.push(Command::UpdateAgent {
                        agent_id: id,
                        payload: update_payload,
                        on_success: Box::new(crate::messages::Message::RefreshAgentsFromAPI),
                        on_error: Box::new(crate::messages::Message::RefreshAgentsFromAPI),
                    });
                } else {
                    web_sys::console::warn_1(&format!("Agent {} not found in state!", id).into());
                }
            } else {
                web_sys::console::warn_1(&"No agent_id found in modal data attribute!".into());
            }

            // Mark state as modified
            state.state_modified = true;

            // Save state to API
            let _ = state.save_if_modified();

            // Close the modal after saving
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Err(e) = crate::components::agent_config_modal::AgentConfigModal::close(&document) {
                        web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
                    }
                }
            }
            true
        }
        Message::CloseAgentModal => {
            // Get the document to close the modal
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    // Close the modal using the modal helper function
                    if let Err(e) = crate::components::agent_config_modal::AgentConfigModal::close(&document) {
                        web_sys::console::error_1(&format!("Failed to close modal: {:?}", e).into());
                    }
                }
            }
            true
        }
        Message::SetAgentTab(tab) => {
            // Use the same helper as in update.rs
            crate::update::handle_agent_tab_switch(state, commands, tab.clone());
            true
        }
        Message::UpdateSystemInstructions(instructions) => {
            // Update the currently-selected agent’s system instructions directly.
            if let Some(node_id) = &state.selected_node_id {
                // Try to resolve the underlying agent_id.
                let agent_id_opt = state.nodes.get(node_id).and_then(|n| n.agent_id);

                if let Some(agent_id) = agent_id_opt {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.system_instructions = Some(instructions.clone());
                        state.state_modified = true;
                    }
                }
            }
            true
        }
        Message::UpdateAgentName(name) => {
            if let Some(id) = &state.selected_node_id {
                let id_clone = id.clone();
                if let Some(node) = state.nodes.get_mut(&id_clone) {
                    // Only update if name is not empty
                    if !name.trim().is_empty() {
                        node.text = name.clone();
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
            true
        }
        Message::LoadAgentInfo(agent_id) => {
            // Start an async task to load agent info
            let agent_id = *agent_id;
            wasm_bindgen_futures::spawn_local(async move {
                let result = crate::network::api_client::ApiClient::get_agent(agent_id)
                    .await
                    .and_then(|response| {
                        serde_json::from_str::<crate::models::ApiAgent>(&response)
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
            true
        }
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
            true
        }
        Message::RetryAgentTask { agent_id } => {
            // Optimistically update the agent status in the state
            if let Some(agent) = state.agents.get_mut(agent_id) {
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

            true
        }
        Message::DismissAgentError { agent_id } => {
            // Optimistically clear the error in the state
            if let Some(agent) = state.agents.get_mut(agent_id) {
                agent.last_error = None;
            }

            // Create payload for API update
            let payload = serde_json::json!({"last_error": null}).to_string();

            // Create command to update the agent via API
            commands.push(Command::UpdateAgent {
                agent_id: *agent_id,
                payload,
                on_success: Box::new(Message::RefreshAgentsFromAPI),
                on_error: Box::new(Message::RefreshAgentsFromAPI),
            });

            true
        }
        Message::RefreshAgentsFromAPI => {
            // Trigger an async operation to fetch agents from the API
            web_sys::console::log_1(&"Requesting agent refresh from API".into());
            // Return a command to fetch agents instead of doing it directly
            commands.push(Command::FetchAgents);
            true
        }
        Message::AgentsRefreshed(agents) => {
            web_sys::console::log_1(&format!("Update: Handling AgentsRefreshed with {} agents", agents.len()).into());

            // Get the current set of agent IDs BEFORE updating
            let old_agent_ids: std::collections::HashSet<u32> = state.agents.keys().cloned().collect();

            // Update state.agents with the new list
            state.agents.clear();
            let mut new_agent_ids = std::collections::HashSet::new();
            for agent in agents {
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
                commands.push(Command::SendMessage(crate::messages::Message::CreateThread(new_agent_id, crate::constants::DEFAULT_THREAD_TITLE.to_string())));
            } else if just_created_agent_ids.len() > 1 {
                web_sys::console::warn_1(&"Detected multiple new agents after refresh, cannot auto-create default thread.".into());
            }

            // Schedule a UI refresh after state is updated
            state.pending_ui_updates = Some(Box::new(|| {
                if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                    web_sys::console::error_1(&format!("Failed to refresh UI after AgentsRefreshed: {:?}", e).into());
                }

                // Ensure Dashboard WS manager is subscribed to all current agents.
                if let Err(e) = crate::components::dashboard::ws_manager::init_dashboard_ws() {
                    web_sys::console::error_1(&format!("Failed to re-init dashboard WS subscriptions: {:?}", e).into());
                }
            }));

            // After updating the agent list trigger a label refresh so all
            // visual nodes show the latest agent names.
            commands.push(Command::SendMessage(crate::messages::Message::RefreshCanvasLabels));

            // Reconcile placeholder canvas nodes that were inserted while the
            // agent list was still loading.  This upgrades them to proper
            // AgentIdentity nodes with correct labels.
            crate::storage::fix_stub_nodes();
            true
        }
        Message::RefreshCanvasLabels => {
            use std::collections::HashSet;

            let mut updated_nodes: HashSet<String> = HashSet::new();

            for (agent_id, node_id) in state.agent_id_to_node_id.iter() {
                if let (Some(agent), Some(node)) = (
                    state.agents.get(agent_id),
                    state.nodes.get_mut(node_id),
                ) {
                    if node.text != agent.name {
                        node.text = agent.name.clone();
                        updated_nodes.insert(node_id.clone());
                    }
                }
            }

            if !updated_nodes.is_empty() {
                state.mark_dirty();
            }
            true
        }
        Message::RequestAgentDeletion { agent_id } => {
            commands.push(Command::DeleteAgentApi { agent_id: *agent_id });
            true
        }
        Message::DeleteAgentApi { agent_id } => {
            // Just delegate to the Command that handles the API call
            commands.push(Command::DeleteAgentApi { agent_id: *agent_id });
            true
        }
        Message::AgentDeletionSuccess { agent_id } => {
            // Remove agent from agents map
            state.agents.remove(agent_id);

            // Remove any nodes associated with this agent
            state.nodes.retain(|_, node| {
                if let Some(node_agent_id) = node.agent_id {
                    node_agent_id != *agent_id
                } else {
                    true
                }
            });

            state.state_modified = true;

            // Add a command to refresh the agents list after state is updated
            commands.push(Command::SendMessage(crate::messages::Message::RefreshAgentsFromAPI));
            true
        }
        Message::AgentDeletionFailure { agent_id, error } => {
            web_sys::console::error_1(&format!("Update: Received AgentDeletionFailure for {}: {}", agent_id, error).into());
            // Optionally, update UI to show error message
            // For now, just log the error
            true
        }
        Message::LoadAgentRuns(agent_id) => {
            if !state.agent_runs.contains_key(agent_id) {
                commands.push(Command::FetchAgentRuns(*agent_id));
            }
            true
        }
        Message::ReceiveAgentRuns { agent_id, runs } => {
            state.agent_runs.insert(*agent_id, runs.clone());

            // Update running_runs set to include any runs that are still in
            // "running" status after the fetch.  Also remove stale IDs not
            // present anymore.
            let mut still_running_ids = std::collections::HashSet::new();
            if let Some(list) = state.agent_runs.get(agent_id) {
                for r in list.iter() {
                    if r.status == "running" {
                        still_running_ids.insert(r.id);
                    }
                }
            }

            // Remove runs for this agent that are no longer running
            state
                .running_runs
                .retain(|rid| still_running_ids.contains(rid) || !state.agent_runs.values().any(|v| v.iter().any(|r| r.id == *rid && r.status != "running")) );

            // Add new running ones
            state.running_runs.extend(still_running_ids);

            // Schedule dashboard refresh so the run history table replaces the spinner.
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        let _ = crate::components::dashboard::refresh_dashboard(&document);
                    }
                }
            })));
            true
        }
        Message::ReceiveRunUpdate { agent_id, run } => {
            let runs_list = state.agent_runs.entry(*agent_id).or_default();
            if let Some(pos) = runs_list.iter().position(|r| r.id == run.id) {
                runs_list.remove(pos);
            }
            let run_clone = run.clone();
            runs_list.insert(0, run_clone);
            if runs_list.len() > 20 {
                runs_list.truncate(20);
            }

            // Manage running_runs set
            match run.status.as_str() {
                "running" => {
                    state.running_runs.insert(run.id);
                }
                "success" | "failed" | "queued" => {
                    state.running_runs.remove(&run.id);
                }
                _ => {}
            }

            // If the dashboard is visible and row expanded, refresh UI to show new row.
            if state.active_view == crate::storage::ActiveView::Dashboard {
                commands.push(Command::UpdateUI(Box::new(|| {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let _ = crate::components::dashboard::refresh_dashboard(&document);
                        }
                    }
                })));
            }
            true
        }
        _ => false,
    }
}
