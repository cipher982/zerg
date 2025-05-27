// frontend/src/update.rs
//
use crate::messages::{Message, Command};
use crate::state::{AppState, APP_STATE, dispatch_global_message, ToolUiState};
use crate::models::{NodeType, ApiThread, ApiThreadMessage, ApiAgent};
use crate::{
    constants::{
    DEFAULT_NODE_WIDTH,
    DEFAULT_NODE_HEIGHT,
    DEFAULT_THREAD_TITLE,
    },
};
use web_sys::Document;
use wasm_bindgen::JsValue;
use std::collections::HashMap;
use crate::components::chat_view::update_thread_list_ui;
use std::collections::HashSet;
use crate::state::AgentConfigTab;
use crate::dom_utils::{hide, show, set_active, set_inactive};

// ---------------------------------------------------------------------------
// Internal helper – encapsulates all DOM + side-effects when the user switches
// between tabs inside the Agent Configuration modal.  Used by the unified
// `Message::SetAgentTab` handler.
// ---------------------------------------------------------------------------

fn handle_agent_tab_switch(state: &mut AppState, commands: &mut Vec<Command>, tab: AgentConfigTab) {
    state.agent_modal_tab = tab;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let by_id = |id: &str| document.get_element_by_id(id);

    // Content sections
    let main_c = by_id("agent-main-content");
    let hist_c = by_id("agent-history-content");
    let trg_c  = by_id("agent-triggers-content");

    // Tab buttons
    let main_t = by_id("agent-main-tab");
    let hist_t = by_id("agent-history-tab");
    let trg_t  = by_id("agent-triggers-tab");

    // Reset all visibility / active state first
    if let Some(el) = &main_c { hide(el); }
    if let Some(el) = &hist_c { hide(el); }
    if let Some(el) = &trg_c  { hide(el); }

    if let Some(btn) = &main_t { set_inactive(btn); }
    if let Some(btn) = &hist_t { set_inactive(btn); }
    if let Some(btn) = &trg_t  { set_inactive(btn); }

    // Content sections
    let tools_c = by_id("agent-tools-content");
    
    // Tab buttons
    let tools_t = by_id("agent-tools-tab");
    
    // Reset all visibility / active state first
    if let Some(el) = &tools_c { hide(el); }
    if let Some(btn) = &tools_t { set_inactive(btn); }
    
    // Activate selected tab
    match tab {
        AgentConfigTab::Main => {
            if let Some(el) = &main_c { show(el); }
            if let Some(btn) = &main_t { set_active(btn); }
        }
        AgentConfigTab::History => {
            if let Some(el) = &hist_c { show(el); }
            if let Some(btn) = &hist_t { set_active(btn); }
        }
        AgentConfigTab::Triggers => {
            if let Some(el) = &trg_c { show(el); }
            if let Some(btn) = &trg_t { set_active(btn); }
        }
        AgentConfigTab::ToolsIntegrations => {
            if let Some(el) = &tools_c { show(el); }
            if let Some(btn) = &tools_t { set_active(btn); }
        }
    }

    // When switching to Triggers we may need to (lazy) fetch triggers.
    if tab == AgentConfigTab::Triggers {
        let agent_id_opt = document
            .get_element_by_id("agent-modal")
            .and_then(|m| m.get_attribute("data-agent-id"))
            .and_then(|s| s.parse::<u32>().ok());

        if let Some(agent_id) = agent_id_opt {
            if !state.triggers.contains_key(&agent_id) {
                commands.push(Command::FetchTriggers(agent_id));
            }

            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::agent_config_modal::render_triggers_list(&doc, agent_id);
                    }
                }
            })));
        }
    }

    // When switching to Tools tab, load MCP servers
    if tab == AgentConfigTab::ToolsIntegrations {
        let agent_id_opt = document
            .get_element_by_id("agent-modal")
            .and_then(|m| m.get_attribute("data-agent-id"))
            .and_then(|s| s.parse::<u32>().ok());

        if let Some(agent_id) = agent_id_opt {
            // Load MCP tools and servers for this agent
            commands.push(Command::SendMessage(Message::LoadMcpTools(agent_id)));

            // Render the MCP server manager UI
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        if let Some(container) = doc.get_element_by_id("agent-tools-content") {
                            let mcp_manager = crate::components::mcp_server_manager::MCPServerManager::new(agent_id);
                            let _ = mcp_manager.build_ui(&doc, &container);
                        }
                    }
                }
            })));
        }
    }
}

// Bring legacy helper trait into scope so its methods are usable on CanvasNode
// Legacy helper trait no longer needed after decoupling cleanup.


pub fn update(state: &mut AppState, msg: Message) -> Vec<Command> {
    let mut needs_refresh = true; // We'll still track this internally for now
    let mut commands = Vec::new(); // Collect commands to return

    // ---------------------------------------------------------------
    // Delegate to domain-specific reducers first.  When one of them
    // consumes the message we can bail out early.
    // ---------------------------------------------------------------

    if crate::reducers::chat::update(state, &msg, &mut commands) {
        return commands;
    }

    match msg {
        // ---------------------------------------------------------------
        // Auth / profile handling
        // ---------------------------------------------------------------
        Message::CurrentUserLoaded(user) => {
            // Store a *clone* inside the state so that we can still access the
            // original `user` value below without running into move issues.
            state.current_user = Some(user.clone());
            state.logged_in = true;

            // Persist Gmail connection status so the Triggers tab can enable
            // the e-mail trigger option without requiring another network
            // call.
            state.gmail_connected = user.gmail_connected;

            let user_for_ui = user.clone();
            // Mount / refresh user menu asynchronously after borrow ends.
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::user_menu::mount_user_menu(&doc);

                        // ------- Update header greeting ------------------
                        if let Some(title_el) = doc.get_element_by_id("header-title") {
                            let greeting = if let Some(name) = user_for_ui.display_name.as_ref().filter(|s| !s.is_empty()) {
                                format!("Welcome, {}!", name)
                            } else {
                                format!("Welcome, {}!", user_for_ui.email)
                            };
                            title_el.set_inner_html(&greeting);
                        }
                    }
                }
            })));

            // Subscribe to user:{id} topic for live updates.
            if let Some(user_id) = state.current_user.as_ref().map(|u| u.id) {
                let topic = format!("user:{}", user_id);
                let topic_manager_rc = state.topic_manager.clone();

                commands.push(Command::UpdateUI(Box::new(move || {
                    if let Ok(mut tm) = topic_manager_rc.try_borrow_mut() {
                        // Prepare handler closure
                        use std::rc::Rc;
                        use std::cell::RefCell;
                        let handler: crate::network::topic_manager::TopicHandler = Rc::new(RefCell::new(move |payload: serde_json::Value| {
                        if let Ok(msg) = serde_json::from_value::<crate::network::ws_schema::WsMessage>(payload.clone()) {
                            if let crate::network::ws_schema::WsMessage::UserUpdate { data } = msg {
                                let profile: crate::models::CurrentUser = data.into();
                                crate::state::dispatch_global_message(Message::CurrentUserLoaded(profile));
                                return;
                            }
                        }
                        }));

                        let _ = tm.subscribe(topic, handler);
                    }
                })));
            }
        }

        Message::ToggleView(view) => {
            let view_clone = view.clone();
            state.active_view = view;
            state.state_modified = true;
            
            // Use a command to update the UI after this function returns
            // This avoids borrowing issues by executing after the current state borrow is released
            commands.push(Command::UpdateUI(Box::new(move || {
                // Get window and document without borrowing state
                if let Some(window) = web_sys::window() {
                    // Keep location.hash in sync for basic routing
                    let _ = match view_clone {
                        crate::storage::ActiveView::Profile => window.location().set_hash("#/profile"),
                        crate::storage::ActiveView::Dashboard => window.location().set_hash("#/dashboard"),
                        crate::storage::ActiveView::Canvas => window.location().set_hash("#/canvas"),
                        crate::storage::ActiveView::ChatView => window.location().set_hash("#/chat"),
                    };

                    if let Some(document) = window.document() {
                        // Directly call render_active_view_by_type without using refresh_ui_after_state_change
                        if let Err(e) = crate::views::render_active_view_by_type(&view_clone, &document) {
                            web_sys::console::error_1(&format!("Failed to switch view: {:?}", e).into());
                        }
                    }
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
           
            // Create and add the node via the new helper that keeps the
            // agent<->node mapping up-to-date.
            let node_id = state.add_agent_node(agent_id, name, x, y);

            // Log the new node ID - make it clear this is a visual node representing an agent
            web_sys::console::log_1(&format!("Created visual node with ID: {} for agent {}", node_id, agent_id).into());
           
            // Draw the nodes on canvas
            state.mark_dirty();
           
            // After creating the agent, immediately create a default thread
            commands.push(Command::SendMessage(Message::CreateThread(agent_id, DEFAULT_THREAD_TITLE.to_string())));
           
            // IMPORTANT: We intentionally don't set state_modified = true here
            // This prevents the auto-save mechanism from making redundant API calls
            
            // Note: A default thread will be created by our Message::AgentsRefreshed handler
            // when the agent list is refreshed from the API.
        },
       
        Message::EditAgent(agent_id) => {
            web_sys::console::log_1(&format!("Update: Handling EditAgent for agent_id: {}", agent_id).into());
            // Quick O(1) lookup via the explicit map
            let node_id_to_select = state.agent_id_to_node_id.get(&agent_id).cloned();

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

                commands.push(Command::UpdateUI(Box::new(move || {
                    if let (_, Some(document)) = (web_sys::window(), web_sys::window().and_then(|w| w.document())) {
                        if let Err(e) = crate::views::show_agent_modal(agent_id, &document) {
                            web_sys::console::error_1(&format!("Failed to show modal: {:?}", e).into());
                        }
                    }
                })));

                // No canvas refresh necessary
                needs_refresh = false;
            }
        },
       
        Message::UpdateNodePosition { node_id, x, y } => {
            state.update_node_position(&node_id, x, y);
            // Only mark state as modified if we're not actively dragging
            // This prevents triggering state saves for every tiny move
            if !state.is_dragging_agent {
                state.state_modified = true;

                // Repaint immediately so canvas follows the cursor in real-time.
                // This mirrors the behaviour of node dragging where
                // `update_node_position()` triggers a draw on every mousemove.
                state.mark_dirty();
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

        // User clicked a canvas node (no drag) – if the node is linked to an
        // agent forward to existing EditAgent flow so that the UI opens the
        // configuration modal via the normal command pipeline.
        Message::CanvasNodeClicked { node_id } => {
            if let Some(agent_id) = state.nodes.get(&node_id).and_then(|n| n.agent_id) {
                commands.push(Command::SendMessage(Message::EditAgent(agent_id)));
            }
        },
       
        Message::UpdateInputText(text) => {
            state.input_text = text;
        },

        Message::MarkCanvasDirty => {
            state.mark_dirty();
        },
       
        Message::StartDragging { node_id, offset_x, offset_y, start_x, start_y, is_agent } => {
            state.dragging = Some(node_id);
            state.drag_offset_x = offset_x;
            state.drag_offset_y = offset_y;
            state.is_dragging_agent = is_agent;
            state.drag_start_x = start_x;
            state.drag_start_y = start_y;
        },
       
        Message::StopDragging => {
            state.dragging = None;
            state.is_dragging_agent = false;
            state.state_modified = true;

            // Persist the final position immediately so a quick page refresh
            // right after dropping a node does not lose the change.  We
            // ignore any error here – the periodic *auto-save* timer will
            // retry in case of intermittent failures.
            let _ = state.save_if_modified();
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
               
                // The drag delta is measured in screen-space pixels. Convert the
                // movement into world coordinates by dividing by the current
                // zoom level so panning speed remains constant regardless of
                // zoom factor.
                let zoom = state.zoom_level;

                state.viewport_x -= dx / zoom;
                state.viewport_y -= dy / zoom;
               
                state.drag_last_x = current_x;
                state.drag_last_y = current_y;
               
                state.state_modified = true;

                // Trigger an immediate repaint so the canvas visually follows
                // the cursor in real-time.  This used to happen implicitly via
                // `refresh_ui_after_state_change()` but that heavy DOM
                // remount was removed in the previous cleanup.  Explicitly
                // redrawing here restores the smooth panning behaviour without
                // re-creating the entire canvas element.
                state.mark_dirty();
            }
        },
       
        Message::StopCanvasDrag => {
            state.canvas_dragging = false;
            state.state_modified = true;

            // Persist final viewport immediately so a quick tab close after a
            // pan operation still records the new position.  Mirrors the
            // behaviour of `StopDragging` for node moves.
            let _ = state.save_if_modified();
        },
       
        Message::ZoomCanvas { new_zoom, viewport_x, viewport_y } => {
            state.zoom_level = new_zoom;
            state.viewport_x = viewport_x;
            state.viewport_y = viewport_y;
            state.state_modified = true;
           
            // Redraw with the new zoom level
            state.mark_dirty();
        },
       
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
                        on_success: Box::new(Message::RefreshAgentsFromAPI),
                        on_error: Box::new(Message::RefreshAgentsFromAPI),
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
        },
       
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
        },
       
        Message::SendTaskToAgent => {
            if let Some(agent_id) = &state.selected_node_id {
                let agent_id_clone = agent_id.clone();
               
                // Fetch task instructions – abort if missing.
                let task_text = match state.get_task_instructions(&agent_id_clone) {
                    Ok(txt) => txt,
                    Err(e) => {
                        web_sys::console::error_1(&format!("Cannot send task: {}", e).into());
                        // We bail out early; nothing to do.
                        return commands;
                    }
                };
               
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
                    if let Some(agent_id) = agent_node.agent_id {
                        // Update visual colour of the node to indicate processing
                        agent_node.color = "#b3e5fc".to_string(); // light blue

                        // Also update agent status in our cache
                        if let Some(agent) = state.agents.get_mut(&agent_id) {
                            agent.status = Some("processing".to_string());
                        }
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
       
        // Unified tab switching variant.
        Message::SetAgentTab(tab) => {
            handle_agent_tab_switch(state, &mut commands, tab);
        },
       
        Message::UpdateSystemInstructions(instructions) => {
            // Update the currently-selected agent’s system instructions directly.
            if let Some(node_id) = &state.selected_node_id {
                // Try to resolve the underlying agent_id.
                let agent_id_opt = state.nodes.get(node_id).and_then(|n| n.agent_id);

                if let Some(agent_id) = agent_id_opt {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.system_instructions = Some(instructions);
                        state.state_modified = true;
                    }
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
               
                // Update visual colour of node to completed
                node.color = "#c8e6c9".to_string(); // light green

                if let Some(agent_id) = node.agent_id {
                    if let Some(agent) = state.agents.get_mut(&agent_id) {
                        agent.status = Some("complete".to_string());
                    }
                }
               
                // Store parent_id before ending the borrow or making other mutable borrows
                let parent_id = node.parent_id.clone();
               
                // Mark state as modified
                state.state_modified = true;
               
                // Update node size for final content
                state.resize_node_for_content(&node_id);
               
                // If this node has a parent, update parent status too
                if let Some(parent_id) = parent_id {
                    if let Some(parent) = state.nodes.get_mut(&parent_id) {
                        parent.color = "#ffecb3".to_string(); // back to idle colour

                        if let Some(agent_id) = parent.agent_id {
                            if let Some(agent) = state.agents.get_mut(&agent_id) {
                                agent.status = Some("idle".to_string());
                            }
                        }
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
            // If canvas is dirty, schedule a UI draw and clear the flag
            if state.dirty {
                state.dirty = false;
                commands.push(Command::UpdateUI(Box::new(|| {
                    crate::state::APP_STATE.with(|state_rc| {
                        let st = state_rc.borrow_mut();
                        st.draw_nodes();
                        #[cfg(debug_assertions)] {
                            if let Some(ctx) = st.context.as_ref() {
                                crate::utils::debug::draw_overlay(ctx, &st.debug_ring);
                            }
                        }
                    });
                })));
            }
            // Live duration ticker for running runs
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
            // Node animations (existing behaviour)
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
            // Trigger dashboard repaint when durations changed
            if duration_changed && state.active_view == crate::storage::ActiveView::Dashboard {
                commands.push(Command::UpdateUI(Box::new(|| {
                    if let Some(win) = web_sys::window() {
                        if let Some(doc) = win.document() {
                            let _ = crate::components::dashboard::refresh_dashboard(&doc);
                        }
                    }
                })));
            }
            // Debounced state persistence
            if state.state_modified {
                let now = crate::utils::now_ms();
                if now.saturating_sub(state.last_modified_ms) > 400 {
                    commands.push(Command::SaveState);
                    state.state_modified = false;
                }
            }
        },
       
        // New Canvas Node message handlers
        Message::AddCanvasNode { agent_id, x, y, node_type, text } => {
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
            state.mark_dirty();
           
            state.state_modified = true;
        },
       
        Message::AddEdge { from_node_id, to_node_id, label } => {
            let edge_id = state.add_edge(from_node_id, to_node_id, label);
            web_sys::console::log_1(&format!("Created new edge with ID: {}", edge_id).into());
           
            // Draw the nodes and edges on canvas
            state.mark_dirty();
           
            state.state_modified = true;
        },
       
        // The explicit node↔agent sync messages have been removed as part of
        // the nodes-vs-agents decoupling refactor (see node_agent_task.md).
        // Canvas nodes are now *display only*; any future synchronisation
        // should be implemented via a dedicated command (e.g.
        // `RefreshCanvasLabels`) driven by agent updates – never the other
        // way around.
       
        Message::GenerateCanvasFromAgents => {
            // Loop through all agents in state.agents and create nodes for any that don't have one
            let mut nodes_created = 0;
           
            // First, find all agents that need nodes
            let agents_needing_nodes = state.agents.iter()
                .filter(|(id, _)| {
                    // Use the explicit mapping
                    !state.agent_id_to_node_id.contains_key(id)
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
               
                // Create and add the node via helper
                state.add_agent_node(*agent_id, name.clone(), x, y);
                nodes_created += 1;
            }
           
            web_sys::console::log_1(&format!("Created {} nodes for agents without visual representation", nodes_created).into());
           
            // Only mark as modified if we actually created nodes
            if nodes_created > 0 {
                state.state_modified = true;
                // Draw the updated nodes on canvas
                state.mark_dirty();
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

                // Ensure Dashboard WS manager is subscribed to all current agents.
                if let Err(e) = crate::components::dashboard::ws_manager::init_dashboard_ws() {
                    web_sys::console::error_1(&format!("Failed to re-init dashboard WS subscriptions: {:?}", e).into());
                }
            }));
            needs_refresh = false; // Refresh handled by pending_ui_updates

            // After updating the agent list trigger a label refresh so all
            // visual nodes show the latest agent names.
            commands.push(Command::SendMessage(Message::RefreshCanvasLabels));

            // Reconcile placeholder canvas nodes that were inserted while the
            // agent list was still loading.  This upgrades them to proper
            // AgentIdentity nodes with correct labels.
            crate::storage::fix_stub_nodes();
        },

        // ------------------------------------------------------------------
        // Display-only sync: update node.text from agent.name
        // ------------------------------------------------------------------
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
                

                // ------------------------------------------------------------------
                // 1. Merge thread metadata into state.threads
                // ------------------------------------------------------------------
                state.threads = threads
                    .iter()
                    .filter_map(|t| t.id.map(|id| (id, t.clone())))
                    .collect();

                // ------------------------------------------------------------------
                // 2. Seed `state.thread_messages` with the **preloaded** messages that
                //    came bundled inside each `Thread` payload.  This allows the sidebar
                //    previews to render immediately without waiting for an explicit
                //    LoadThreadMessages call (bug-fix #142).
                // ------------------------------------------------------------------
                for t in &threads {
                    if let Some(tid) = t.id {
                        if !t.messages.is_empty() {
                            // Clone to decouple lifetimes
                            state.thread_messages.insert(tid, t.messages.clone());
                        }
                    }
                }
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

                // After state is fully populated, refresh the sidebar so that
                // message previews become visible even before a thread is
                // actively selected.
                let threads_data: Vec<ApiThread> = state.threads.values().cloned().collect();
                let current_thread_id = state.current_thread_id;
                let thread_messages = state.thread_messages.clone();

                commands.push(Command::UpdateUI(Box::new(move || {
                    dispatch_global_message(Message::UpdateThreadList(
                        threads_data,
                        current_thread_id,
                        thread_messages,
                    ));
                })));
            } else {
                web_sys::console::warn_1(&"Received ThreadsLoaded outside of ChatView".into());
            }
        },
       
       
        // This variant should no longer be dispatched by the UI.  We keep the
        // arm to satisfy the exhaustive match requirement and to surface a
        // helpful warning in case some legacy code still emits it.
        Message::SendThreadMessage(_, _) => {
            web_sys::console::warn_1(&"Received legacy SendThreadMessage; ignoring to avoid duplicate network call".into());
        },
       
        Message::ThreadMessageSent(_response, _client_id) => {
            // This handler is deprecated as we no longer use optimistic UI
            // Just log a warning and do nothing
            web_sys::console::warn_1(&"ThreadMessageSent is deprecated, use ThreadMessagesLoaded instead".into());
        },
       
        Message::ThreadMessageFailed(_thread_id, _client_id) => {
            // This handler is deprecated as we no longer use optimistic UI
            // Just log a warning and do nothing
            web_sys::console::warn_1(&"ThreadMessageFailed is deprecated".into());
            
            // Show error notification
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                    // Show error message in UI
                    web_sys::window()
                        .and_then(|w| w.alert_with_message("Message failed to send. Please try again.").ok());
                }
            })));
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
                        hide(&chat_container);
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
            // We expect to be inside an active thread when the user presses
            // Enter in the chat input.  Guard just in case.
            if let Some(thread_id) = state.current_thread_id {
                // Delegate all optimistic‑UI + network responsibilities to the
                // dedicated helper so we have a single source of truth.
                let ui_callback = crate::thread_handlers::handle_send_thread_message(
                    thread_id,
                    content,
                    &mut state.thread_messages,
                    &state.threads,
                    state.current_thread_id,
                );

                // Schedule the callback to run after this update finishes so we
                // avoid active mutable borrows of `state`.
                commands.push(Command::UpdateUI(ui_callback));
            } else {
                web_sys::console::error_1(&"RequestSendMessage but no current_thread_id".into());
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
            // Schedule UI update for the provided conversation messages after state updates
            let messages_clone = messages.clone();
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window().unwrap().document() {
                    let current_user_opt = crate::state::APP_STATE.with(|s| s.borrow().current_user.clone());
                    let _ = crate::components::chat_view::update_conversation_ui(&document, &messages_clone, current_user_opt.as_ref());
                }
            })));
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

       

        // --- NEW WebSocket Received Messages ---
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

            // Regardless of whether this is the active thread we want the
            // sidebar preview to reflect the new message.
            let threads_data: Vec<ApiThread> = state.threads.values().cloned().collect();
            let thread_messages_map = state.thread_messages.clone();
            let current_thread_id = state.current_thread_id;

            commands.push(Command::UpdateUI(Box::new(move || {
                dispatch_global_message(Message::UpdateThreadList(
                    threads_data,
                    current_thread_id,
                    thread_messages_map,
                ));
            })));
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

            // Reset current assistant-message tracker for this thread so that
            // the first chunk starts a new bubble.
            state.active_streams.insert(thread_id, None);

            web_sys::console::log_1(&format!("Stream started for thread {}.", thread_id).into());
        },

        Message::ReceiveStreamEnd(thread_id) => {
            // Mark thread as no longer streaming in local state
            state.streaming_threads.remove(&thread_id);

            // Reset token-mode flag so a subsequent run can re-detect the mode
            state.token_mode_threads.remove(&thread_id);

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

        // -----------------------------------------------------------------
        // AssistantId – arrives once per *token-mode* stream right after the
        // backend persisted the assistant row.  We store the id so upcoming
        // tool_output chunks can link to this bubble.  Additionally we patch
        // the last assistant message (if id is still None) so future UI
        // operations have the correct PK.
        // -----------------------------------------------------------------

        Message::ReceiveAssistantId { thread_id, message_id } => {
            web_sys::console::log_1(&format!("Received AssistantId {} for thread {}", message_id, thread_id).into());

            state.active_streams.insert(thread_id, Some(message_id));

            // Update the most recent assistant bubble id if it was None.
            if let Some(messages) = state.thread_messages.get_mut(&thread_id) {
                if let Some(last) = messages.iter_mut().rev().find(|m| m.role == "assistant" && m.id.is_none()) {
                    last.id = Some(message_id);
                }
            }
        },
        
        // Toggle collapse/expand state for a tool call indicator
        Message::ToggleToolExpansion { tool_call_id } => {
            let entry = state.tool_ui_states.entry(tool_call_id.clone())
                .or_insert(ToolUiState { expanded: false, show_full: false });
            entry.expanded = !entry.expanded;
            // Trigger UI update for the conversation
            if let Some(thread_id) = state.current_thread_id {
                if let Some(messages) = state.thread_messages.get(&thread_id) {
                    let messages_clone = messages.clone();
                    state.pending_ui_updates = Some(Box::new(move || {
                        dispatch_global_message(Message::UpdateConversation(messages_clone));
                    }));
                }
            }
        },
        
        // Toggle full vs truncated tool output view for a tool call
        Message::ToggleToolShowMore { tool_call_id } => {
            let entry = state.tool_ui_states.entry(tool_call_id.clone())
                .or_insert(ToolUiState { expanded: false, show_full: false });
            entry.show_full = !entry.show_full;
            // Trigger UI update for the conversation
            if let Some(thread_id) = state.current_thread_id {
                if let Some(messages) = state.thread_messages.get(&thread_id) {
                    let messages_clone = messages.clone();
                    state.pending_ui_updates = Some(Box::new(move || {
                        dispatch_global_message(Message::UpdateConversation(messages_clone));
                    }));
                }
            }
        },

        // --- Agent Deletion Flow ---
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

        // Agent Debug Modal messages
        Message::UpdateLoadingState(is_loading) => {
            state.is_chat_loading = is_loading;
            
            // Update the UI
            if let Some(document) = web_sys::window().expect("no global window exists").document() {
                let _ = crate::components::chat_view::update_loading_state(&document, is_loading);
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

        // Agent Debug Modal
        Message::ShowAgentDebugModal { agent_id } => {
            state.agent_debug_pane = Some(crate::state::AgentDebugPane {
                agent_id,
                loading: true,
                details: None,
                active_tab: crate::state::DebugTab::Overview,
            });

            commands.push(Command::FetchAgentDetails(agent_id));

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        crate::state::APP_STATE.with(|s| {
                            let app_state = s.borrow();
                            let _ = crate::components::agent_debug_modal::render_agent_debug_modal(&app_state, &doc);
                        });
                    }
                }
            })));

            needs_refresh = false;
        },

        Message::HideAgentDebugModal => {
            state.agent_debug_pane = None;

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        let _ = crate::components::agent_debug_modal::hide_agent_debug_modal(&doc);
                    }
                }
            })));

            needs_refresh = false;
        },

        Message::ReceiveAgentDetails(details) => {
            if let Some(pane) = state.agent_debug_pane.as_mut() {
                pane.loading = false;
                pane.details = Some(details);
            }

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        crate::state::APP_STATE.with(|s| {
                            let app_state = s.borrow();
                            let _ = crate::components::agent_debug_modal::render_agent_debug_modal(&app_state, &doc);
                        });
                    }
                }
            })));

            needs_refresh = false;
        },

        Message::SetAgentDebugTab(tab) => {
            if let Some(pane) = state.agent_debug_pane.as_mut() {
                // Only trigger UI update if the tab actually changed to avoid
                // unnecessary re-renders.
                if pane.active_tab != tab {
                    pane.active_tab = tab.clone();

                    commands.push(Command::UpdateUI(Box::new(|| {
                        if let Some(window) = web_sys::window() {
                            if let Some(doc) = window.document() {
                                crate::state::APP_STATE.with(|s| {
                                    let app_state = s.borrow();
                                    let _ = crate::components::agent_debug_modal::render_agent_debug_modal(&app_state, &doc);
                                });
                            }
                        }
                    })));
                }
            }

            needs_refresh = false;
        },

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

        // -----------------------------------------------------
        // Run History Messages (AgentRun)
        // -----------------------------------------------------

        Message::LoadAgentRuns(agent_id) => {
            if !state.agent_runs.contains_key(&agent_id) {
                commands.push(Command::FetchAgentRuns(agent_id));
            }
            // UI will refresh once runs are loaded
            needs_refresh = false;
        },

        Message::ReceiveAgentRuns { agent_id, runs } => {
            state.agent_runs.insert(agent_id, runs);

            // Update running_runs set to include any runs that are still in
            // "running" status after the fetch.  Also remove stale IDs not
            // present anymore.
            let mut still_running_ids = HashSet::new();
            if let Some(list) = state.agent_runs.get(&agent_id) {
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
        },

        Message::ReceiveRunUpdate { agent_id, run } => {
            let runs_list = state.agent_runs.entry(agent_id).or_default();
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
        },

        // -----------------------------------------------------------------
        // Trigger management (Phase A minimal state sync)
        // -----------------------------------------------------------------

        Message::LoadTriggers(agent_id) => {
            // Fire network command – actual update comes via TriggersLoaded.
            commands.push(Command::FetchTriggers(agent_id));
        },

        Message::TriggersLoaded { agent_id, triggers } => {
            state.triggers.insert(agent_id, triggers);

            // If modal is open on Triggers tab, refresh the list (TODO – will
            // be implemented in Phase B).  For now we simply mark canvas
            // dirty which is a no-op for modal but keeps behaviour
            // consistent with other update paths.
            state.mark_dirty();

            // If triggers tab for this agent is currently visible, re-render.
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        // Check if triggers-content is visible (style display)
                        if let Some(content) = doc.get_element_by_id("triggers-content") {
                            if content.get_attribute("style").map(|s| s.contains("display: block")).unwrap_or(false) {
                                let _ = crate::components::agent_config_modal::render_triggers_list(&doc, agent_id);
                            }
                        }
                    }
                }
            })));
        },

        Message::TriggerCreated { agent_id, trigger } => {
            state.triggers.entry(agent_id).or_default().push(trigger);
            state.mark_dirty();

            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::agent_config_modal::render_triggers_list(&doc, agent_id);
                    }
                }
            })));
        },

        Message::TriggerDeleted { agent_id, trigger_id } => {
            if let Some(list) = state.triggers.get_mut(&agent_id) {
                list.retain(|t| t.id != trigger_id);
            }
            state.mark_dirty();

            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::agent_config_modal::render_triggers_list(&doc, agent_id);
                    }
                }
            })));
        },

        // -----------------------------------------------------------
        // Gmail OAuth flow – Phase C (frontend-only stub)
        // -----------------------------------------------------------

        Message::GmailConnected => {
            state.gmail_connected = true;

            // Re-render UI pieces that depend on the flag (Triggers tab).
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::agent_config_modal::render_gmail_connect_status(&doc);
                    }
                }
            })));
        },

        // UI requested new trigger creation – translate into network command.
        Message::RequestCreateTrigger { payload_json } => {
            commands.push(Command::CreateTrigger { payload_json });
        },

        Message::RequestDeleteTrigger { trigger_id } => {
            commands.push(Command::DeleteTrigger(trigger_id));
        },

        // Toggle compact/full run history view
        Message::ToggleRunHistory { agent_id } => {
            if state.run_history_expanded.contains(&agent_id) {
                state.run_history_expanded.remove(&agent_id);
            } else {
                state.run_history_expanded.insert(agent_id);
            }

            // Refresh dashboard UI
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        let _ = crate::components::dashboard::refresh_dashboard(&document);
                    }
                }
            })));
        },

        // -------------------------------------------------------------------
        // Dashboard scope toggle (My ⇄ All)
        // -------------------------------------------------------------------
        Message::ToggleDashboardScope(new_scope) => {
            if state.dashboard_scope != new_scope {
                state.dashboard_scope = new_scope;

                // Persist to localStorage
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        let _ = storage.set_item("dashboard_scope", new_scope.as_str());
                    }
                }

                // Trigger agent list reload
                commands.push(Command::FetchAgents);

                // Force dashboard re-render
                commands.push(Command::UpdateUI(Box::new(|| {
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            let _ = crate::components::dashboard::refresh_dashboard(&document);
                        }
                    }
                })));
            }
        },
        // -------------------------------------------------------------------
        // MCP Integration Messages
        // -------------------------------------------------------------------
        Message::LoadMcpTools(agent_id) => {
            web_sys::console::log_1(&format!("LoadMcpTools for agent {}", agent_id).into());
            
            // Fetch available tools from the backend
            commands.push(Command::UpdateUI(Box::new(move || {
                wasm_bindgen_futures::spawn_local(async move {
                    match crate::network::api_client::ApiClient::get_mcp_available_tools(agent_id).await {
                        Ok(response) => {
                            // Parse the response to extract builtin and MCP tools
                            match serde_json::from_str::<serde_json::Value>(&response) {
                                Ok(json) => {
                                    let builtin_tools = json["builtin"]
                                        .as_array()
                                        .unwrap_or(&Vec::new())
                                        .iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect::<Vec<String>>();
                                    
                                    let mut mcp_tools: HashMap<String, Vec<crate::state::McpToolInfo>> = HashMap::new();
                                    
                                    if let Some(mcp_obj) = json["mcp"].as_object() {
                                        for (server_name, tools_array) in mcp_obj {
                                            let tools: Vec<crate::state::McpToolInfo> = tools_array
                                                .as_array()
                                                .unwrap_or(&Vec::new())
                                                .iter()
                                                .filter_map(|tool| {
                                                    tool.as_str().map(|name| crate::state::McpToolInfo {
                                                        name: name.to_string(),
                                                        server_name: server_name.clone(),
                                                        description: None,
                                                    })
                                                })
                                                .collect();
                                            
                                            if !tools.is_empty() {
                                                mcp_tools.insert(server_name.clone(), tools);
                                            }
                                        }
                                    }
                                    
                                    dispatch_global_message(Message::McpToolsLoaded {
                                        agent_id,
                                        builtin_tools,
                                        mcp_tools,
                                    });
                                },
                                Err(e) => {
                                    web_sys::console::error_1(&format!("Failed to parse MCP tools response: {:?}", e).into());
                                    dispatch_global_message(Message::McpError {
                                        agent_id,
                                        error: format!("Failed to parse tools: {}", e),
                                    });
                                }
                            }
                        },
                        Err(e) => {
                            web_sys::console::error_1(&format!("Failed to load MCP tools: {:?}", e).into());
                            dispatch_global_message(Message::McpError {
                                agent_id,
                                error: format!("Failed to load tools: {:?}", e),
                            });
                        }
                    }
                });
            })));
        },
        Message::McpToolsLoaded { agent_id, builtin_tools, mcp_tools } => {
            // Update state with loaded tools
            state.available_mcp_tools.insert(agent_id, mcp_tools.values().flatten().cloned().collect());
            // TODO: Handle builtin_tools separately if needed
            web_sys::console::log_1(&format!("McpToolsLoaded for agent {}: {:?} built-in, {:?} mcp", agent_id, builtin_tools, mcp_tools).into());
            // Trigger UI update for the tools tab
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        // TODO: Render MCP tools UI
                        web_sys::console::log_1(&"Render MCP tools UI after loading".into());
                    }
                }
            })));
        },
        Message::AddMcpServer { agent_id, server_config } => {
            web_sys::console::log_1(&format!("AddMcpServer for agent {}: {:?}", agent_id, server_config).into());
            let payload = serde_json::to_string(&server_config).unwrap_or_else(|_| "{}".to_string());
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers", agent_id),
                method: "POST".to_string(),
                body: Some(payload),
                on_success: Box::new(Message::McpServerAdded {
                    agent_id,
                    server_name: server_config.name.clone(),
                }),
                on_error: Box::new(Message::McpError {
                    agent_id,
                    error: format!("Failed to add server: {}", server_config.name),
                }),
            });
        },
        Message::RemoveMcpServer { agent_id, server_name } => {
            web_sys::console::log_1(&format!("RemoveMcpServer for agent {}: {}", agent_id, server_name).into());
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers/{}", agent_id, server_name),
                method: "DELETE".to_string(),
                body: None,
                on_success: Box::new(Message::McpServerRemoved {
                    agent_id,
                    server_name: server_name.clone(),
                }),
                on_error: Box::new(Message::McpError {
                    agent_id,
                    error: format!("Failed to remove server: {}", server_name),
                }),
            });
        },
        Message::TestMcpConnection { agent_id, server_config } => {
            web_sys::console::log_1(&format!("TestMcpConnection for agent {}: {:?}", agent_id, server_config).into());
            let payload = serde_json::to_string(&server_config).unwrap_or_else(|_| "{}".to_string());
            commands.push(Command::NetworkCall {
                endpoint: format!("/api/agents/{}/mcp-servers/test", agent_id),
                method: "POST".to_string(),
                body: Some(payload),
                on_success: Box::new(Message::McpConnectionTested {
                    agent_id,
                    server_name: server_config.name.clone(),
                    status: crate::state::ConnectionStatus::Healthy, // Placeholder
                }),
                on_error: Box::new(Message::McpConnectionTested {
                    agent_id,
                    server_name: server_config.name.clone(),
                    status: crate::state::ConnectionStatus::Failed("Connection failed".to_string()), // Placeholder
                }),
            });
        },
        Message::McpConnectionTested { agent_id, server_name, status } => {
            let key = format!("{}:{}", agent_id, server_name);
            state.mcp_connection_status.insert(key, status.clone());
            web_sys::console::log_1(&format!("McpConnectionTested for agent {}: {} status {:?}", agent_id, server_name, status).into());
            // Trigger UI update for the tools tab
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        // TODO: Render MCP tools UI to show updated status
                        web_sys::console::log_1(&"Render MCP tools UI after connection test".into());
                    }
                }
            })));
        },
        Message::UpdateAllowedTools { agent_id, allowed_tools } => {
            if let Some(config) = state.agent_mcp_configs.get_mut(&agent_id) {
                config.allowed_tools = allowed_tools;
            } else {
                state.agent_mcp_configs.insert(agent_id, crate::state::AgentMcpConfig {
                    servers: Vec::new(),
                    allowed_tools,
                });
            }
            web_sys::console::log_1(&format!("UpdateAllowedTools for agent {}: {:?}", agent_id, state.agent_mcp_configs.get(&agent_id).map(|c| &c.allowed_tools)).into());
            // TODO: Persist this change to the backend (update agent config)
        },
        Message::McpServerAdded { agent_id, server_name } => {
            // Refresh MCP configs for the agent
            commands.push(Command::SendMessage(Message::LoadMcpTools(agent_id)));
            web_sys::console::log_1(&format!("McpServerAdded for agent {}: {}", agent_id, server_name).into());
        },
        Message::McpServerRemoved { agent_id, server_name } => {
            // Refresh MCP configs for the agent
            commands.push(Command::SendMessage(Message::LoadMcpTools(agent_id)));
            web_sys::console::log_1(&format!("McpServerRemoved for agent {}: {}", agent_id, server_name).into());
        },
        Message::McpError { agent_id, error } => {
            web_sys::console::error_1(&format!("MCP Error for agent {}: {}", agent_id, error).into());
            // TODO: Display error to user in UI
        },

        // --- MCP UI message handlers ---
        Message::SetMCPTab { agent_id, tab } => {
            // Update the active MCP tab in state
            web_sys::console::log_1(&format!("SetMCPTab for agent {}: {:?}", agent_id, tab).into());
            
            // TODO: Store tab state if needed
            // For now, just trigger a UI update to re-render with the new tab
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    if let Some(doc) = win.document() {
                        // The MCP component will handle tab switching based on its internal state
                        web_sys::console::log_1(&"MCP tab switch UI update".into());
                    }
                }
            })));
        },
        
        Message::ConnectMCPPreset { agent_id, preset_id } => {
            web_sys::console::log_1(&format!("ConnectMCPPreset for agent {}: {}", agent_id, preset_id).into());
            
            // TODO: Show auth dialog for preset connection
            // For now, just log and show a placeholder alert
            commands.push(Command::UpdateUI(Box::new(move || {
                if let Some(win) = web_sys::window() {
                    let _ = win.alert_with_message(&format!("Connect to {} preset (auth flow to be implemented)", preset_id));
                }
            })));
        },
        
        Message::AddMCPServer { agent_id, url, name, preset, auth_token } => {
            web_sys::console::log_1(&format!("AddMCPServer for agent {}: {} ({})", agent_id, name, url.as_deref().unwrap_or("preset")).into());
            
            // Create the server config
            let server_config = crate::state::McpServerConfig {
                name: name.clone(),
                url: url.clone(),
                preset: preset.clone(),
                auth_token: Some(auth_token.clone()),
            };
            
            // Convert to the message format expected by the existing handler
            commands.push(Command::SendMessage(Message::AddMcpServer {
                agent_id,
                server_config,
            }));
        },
        
        Message::RemoveMCPServer { agent_id, server_name } => {
            web_sys::console::log_1(&format!("RemoveMCPServer for agent {}: {}", agent_id, server_name).into());
            
            // Convert to the message format expected by the existing handler
            commands.push(Command::SendMessage(Message::RemoveMcpServer {
                agent_id,
                server_name,
            }));
        },
        
        Message::TestMCPConnection { agent_id, url, name, auth_token } => {
            web_sys::console::log_1(&format!("TestMCPConnection for agent {}: {} at {}", agent_id, name, url).into());
            
            // Create the server config for testing
            let server_config = crate::state::McpServerConfig {
                name: name.clone(),
                url: Some(url.clone()),
                preset: None,
                auth_token: Some(auth_token.clone()),
            };
            
            // Convert to the message format expected by the existing handler
            commands.push(Command::SendMessage(Message::TestMcpConnection {
                agent_id,
                server_config,
            }));
        }
    }

    // -------------------------------------------------------------------
    // After handling the message: if any mutation occurred mark timestamp.
    // This centralised check guarantees `last_modified_ms` is always kept
    // in sync with the `state_modified` flag without sprinkling
    // `utils::now_ms()` across every reducer arm.
    // -------------------------------------------------------------------
    if state.state_modified {
        state.last_modified_ms = crate::utils::now_ms();
    }

    // For now, if needs_refresh is true, add a NoOp command
    // We'll replace this with proper UI refresh commands later
    if needs_refresh {
        commands.push(Command::NoOp);
    }

    commands
}

// Update thread list UI
#[allow(dead_code)]
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
#[allow(dead_code)]
pub fn update_thread_list_with_data(
    document: &Document,
    threads: &[ApiThread],
    current_thread_id: Option<u32>,
    thread_messages: &HashMap<u32, Vec<ApiThreadMessage>>
) -> Result<(), JsValue> {
    update_thread_list_ui(document, threads, current_thread_id, thread_messages)
}

// Update conversation UI
#[allow(dead_code)]
pub fn update_conversation(document: &Document) -> Result<(), JsValue> {
    APP_STATE.with(|state| {
        let state = state.borrow();
        if let Some(thread_id) = state.current_thread_id {
            if let Some(messages) = state.thread_messages.get(&thread_id) {
                let current_user_opt = crate::state::APP_STATE.with(|s| s.borrow().current_user.clone());
                return crate::components::chat_view::update_conversation_ui(document, messages, current_user_opt.as_ref());
            }
        }

        // (Removed misplaced match arm – correct implementation lives inside
        // the main `update()` reducer near other dashboard-related logic.)
        Ok(())
    })
}
