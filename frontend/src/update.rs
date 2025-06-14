// frontend/src/update.rs
//
use crate::messages::{Message, Command};
use crate::state::{AppState, APP_STATE, dispatch_global_message};
use crate::models::{ApiThread, ApiThreadMessage};
use web_sys::Document;
use wasm_bindgen::JsValue;
use std::collections::HashMap;
use crate::components::chat_view::update_thread_list_ui;
use crate::state::AgentConfigTab;
use crate::dom_utils::{hide, show, set_active, set_inactive};

// ---------------------------------------------------------------------------
// Internal helper – encapsulates all DOM + side-effects when the user switches
// between tabs inside the Agent Configuration modal.  Used by the unified
// `Message::SetAgentTab` handler.
// ---------------------------------------------------------------------------

pub fn handle_agent_tab_switch(state: &mut AppState, commands: &mut Vec<Command>, tab: AgentConfigTab) {
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
    if crate::reducers::canvas::update(state, &msg, &mut commands) {
        return commands;
    }
    if crate::reducers::agent::update(state, &msg, &mut commands) {
        return commands;
    }
    if crate::reducers::dashboard::update(state, &msg, &mut commands) {
        return commands;
    }
    if crate::reducers::mcp::update(state, &msg, &mut commands) {
        return commands;
    }
    if crate::reducers::triggers::update(state, &msg, &mut commands) {
        return commands;
    }

    match msg {
        Message::SetPowerMode(enabled) => {
            state.power_mode = enabled;
            if let Some(window) = web_sys::window() {
                if let Some(doc) = window.document() {
                    if enabled {
                        crate::register_global_shortcuts(&doc);
                        crate::toast::info("Power Mode enabled: Keyboard shortcuts are active. Press '?' for help.");
                    } else {
                        crate::remove_global_shortcuts(&doc);
                        crate::toast::info("Power Mode disabled: Keyboard shortcuts are off.");
                    }
                }
            }
            return vec![];
        }
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
                    {
                        let mut tm = topic_manager_rc.borrow_mut();
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
            let view_clone_for_shelf = view.clone();
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
            
            // If switching to Canvas, always fetch agents (will trigger shelf update when loaded)
            if view_clone_for_shelf == crate::storage::ActiveView::Canvas {
                commands.push(Command::FetchAgents);
                commands.push(Command::FetchWorkflows);
            }
        },
       
       
       
       
        Message::ResetDatabase => {
            // The actual database reset happens via API call (already done in dashboard.rs)
            // We don't need to do anything here because:
            // 1. The page will be refreshed immediately after this (in dashboard.rs)
            // 2. On refresh, it will automatically load the fresh state from the backend
            web_sys::console::log_1(&"Reset database message received - state will refresh".into());
        },
       
       

       
        // Thread-related messages
       
       
       
       
       
       
       
        // Navigation messages
       
       







        


        // Force re-render of Dashboard when active

        

       

        // --- NEW WebSocket Received Messages ---




        // -----------------------------------------------------------------
        // AssistantId – arrives once per *token-mode* stream right after the
        // backend persisted the assistant row.  We store the id so upcoming
        // tool_output chunks can link to this bubble.  Additionally we patch
        // the last assistant message (if id is still None) so future UI
        // operations have the correct PK.
        // -----------------------------------------------------------------

        
        // Toggle collapse/expand state for a tool call indicator
        
        // Toggle full vs truncated tool output view for a tool call

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
                commands.push(Command::UpdateUI(Box::new(move || {
                    dispatch_global_message(Message::UpdateConversation(messages_clone_for_dispatch));
                })));

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
            commands.push(Command::UpdateUI(Box::new(move || {
                // Dispatch a message to update the thread title UI
                dispatch_global_message(Message::UpdateThreadTitleUI(title_to_update));
            })));
        },

        // Agent Debug Modal



        // Model management
        Message::SetAvailableModels { models, default_model_id } => {
            state.available_models = models.clone();
            state.default_model_id = default_model_id.clone();
            state.state_modified = true;
            needs_refresh = false; // No UI refresh needed for model updates
        }

        // -----------------------------------------------------------------
        // Trigger management (Phase A minimal state sync)
        // -----------------------------------------------------------------





        // -----------------------------------------------------------
        // Gmail OAuth flow – Phase C (frontend-only stub)
        // -----------------------------------------------------------



        // Toggle compact/full run history view

        // -------------------------------------------------------------------
        // Dashboard scope toggle (My ⇄ All)
        // -------------------------------------------------------------------
        // -------------------------------------------------------------------
        // MCP Integration Messages
        // -------------------------------------------------------------------

        // --- MCP UI message handlers ---
        
        
        
        // Canvas-related messages are handled by the canvas reducer
        Message::UpdateNodePosition { .. }
        | Message::AddNode { .. }
        | Message::AddResponseNode { .. }
        | Message::ToggleAutoFit
        | Message::CenterView
        | Message::ResetView
        | Message::ClearCanvas
        | Message::CanvasNodeClicked { .. }
        | Message::MarkCanvasDirty
        | Message::StartDragging { .. }
        | Message::StopDragging
        | Message::StartCanvasDrag { .. }
        | Message::UpdateCanvasDrag { .. }
        | Message::StopCanvasDrag
        | Message::ZoomCanvas { .. }
        | Message::AddCanvasNode { .. }
        | Message::DeleteNode { .. }
        | Message::UpdateNodeText { .. }
        | Message::CompleteNodeResponse { .. }
        | Message::UpdateNodeStatus { .. } => {
            // These are handled by the canvas reducer which returns early
            unreachable!("Canvas messages should be handled by the canvas reducer")
        }
        // (handled later)
        | Message::AnimationTick => {
            // Fire async command to backend
            // These are handled elsewhere or in canvas reducer. No-op here.
        }

        // -------------------------------------------------------------------
        // SubscribeWorkflowExecution – create WS topic subscription
        // -------------------------------------------------------------------
        Message::SubscribeWorkflowExecution { execution_id } => {
            // Update state.current_execution with real id
            state.current_execution = Some(crate::state::ExecutionStatus {
                execution_id,
                status: crate::state::ExecPhase::Running,
            });

            let topic = format!("workflow_execution:{}", execution_id);
            let topic_manager_rc = state.topic_manager.clone();

            commands.push(Command::UpdateUI(Box::new(move || {
                use std::cell::RefCell;
                use std::rc::Rc;
                let mut tm = topic_manager_rc.borrow_mut();

                let handler: crate::network::topic_manager::TopicHandler = Rc::new(RefCell::new(move |payload: serde_json::Value| {
                    use crate::network::ws_schema::WsMessage;
                    if let Ok(parsed) = serde_json::from_value::<WsMessage>(payload) {
                        match parsed {
                            WsMessage::NodeState { data } => {
                                crate::state::dispatch_global_message(Message::UpdateNodeStatus {
                                    node_id: data.node_id.clone(),
                                    status: data.status.clone(),
                                });
                            }
                            WsMessage::ExecutionFinished { data } => {
                                crate::state::dispatch_global_message(Message::ExecutionFinished {
                                    execution_id: data.execution_id,
                                    status: data.status.clone(),
                                    error: data.error.clone(),
                                });
                            }
                            WsMessage::NodeLog { data } => {
                                crate::state::dispatch_global_message(Message::AppendExecutionLog {
                                    execution_id: data.execution_id,
                                    node_id: data.node_id.clone(),
                                    stream: data.stream.clone(),
                                    text: data.text.clone(),
                                });
                            }
                            _ => {}
                        }
                    }
                }));

                let _ = tm.subscribe(topic, handler);
            })));

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::update_run_button(&doc);
                }
            })));
        }

        // -------------------------------------------------------------------
        // Start workflow execution – triggers API call command
        // -------------------------------------------------------------------
        Message::StartWorkflowExecution { workflow_id } => {
            state.current_execution = Some(crate::state::ExecutionStatus {
                execution_id: 0,
                status: crate::state::ExecPhase::Starting,
            });
            state.execution_logs.clear();
            commands.push(Command::StartWorkflowExecutionApi { workflow_id });

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::update_run_button(&doc);
                }
            })));

            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::log_drawer::refresh(&doc);
                }
            })));
        }
        Message::CreateWorkflow { name } => {
            commands.push(Command::CreateWorkflowApi { name: name.clone() });
        }

        Message::RenameWorkflow { workflow_id, name, description } => {
            commands.push(Command::RenameWorkflowApi { workflow_id, name: name.clone(), description: description.clone() });
        }

        Message::DeleteWorkflow { workflow_id } => {
            commands.push(Command::DeleteWorkflowApi { workflow_id });
        }
        Message::SelectWorkflow { workflow_id } => {
            state.current_workflow_id = Some(workflow_id);
            needs_refresh = true;

            // Refresh run button & log drawer in UI
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::update_run_button(&doc);
                    let _ = crate::components::log_drawer::refresh(&doc);
                }
            })));
        }
        Message::WorkflowCreated(wf) => {
            // Remove any optimistic entry with same name or temp id (<=0)
            let wf_id = wf.id;
            state.workflows.retain(|_, v| v.id > 0 || v.name != wf.name);
            state.workflows.insert(wf_id, wf);
            state.current_workflow_id = Some(wf_id);
            needs_refresh = true;
            // UI update to refresh bar after new workflow added
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::refresh(&doc);
                }
            })));
        }

        // (ExecutionFinished & AppendExecutionLog handled in dedicated arms
        // below.  Any UI refresh logic is appended there to avoid duplicate
        // pattern matches that caused unreachable-pattern warnings.)
        Message::WorkflowDeleted { workflow_id } => {
            state.workflows.remove(&workflow_id);
            if state.current_workflow_id == Some(workflow_id) {
                state.current_workflow_id = state.workflows.keys().next().cloned();
            }
            needs_refresh = true;
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::refresh(&doc);
                }
            })));
        }
        Message::WorkflowUpdated(wf) => {
            state.workflows.insert(wf.id, wf.clone());
            needs_refresh = true;
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::workflow_switcher::refresh(&doc);
                }
            })));
        }

        // -------------------------------------------------------------------
        // Live workflow execution updates
        // -------------------------------------------------------------------

        Message::ExecutionFinished { execution_id, status, error } => {
            if let Some(exec) = &mut state.current_execution {
                if exec.execution_id == 0 || exec.execution_id == execution_id {
                    exec.execution_id = execution_id;
                    exec.status = if status == "success" {
                        crate::state::ExecPhase::Success
                    } else {
                        crate::state::ExecPhase::Failed
                    };
                }
            }
            if let Some(err) = error {
                state.execution_logs.push(crate::state::ExecutionLog {
                    node_id: "execution".to_string(),
                    stream: "error".to_string(),
                    text: format!("Execution failed: {}", err),
                });
            }
            needs_refresh = true;
        }

        Message::AppendExecutionLog { execution_id, node_id, stream, text } => {
            if let Some(exec) = &state.current_execution {
                if exec.execution_id == 0 || exec.execution_id == execution_id {
                    state.execution_logs.push(crate::state::ExecutionLog { node_id: node_id.clone(), stream: stream.clone(), text: text.clone() });
                    needs_refresh = true;
                }
            }

            // Live append means drawer needs repaint if open
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::log_drawer::refresh(&doc);
                }
            })));
        }

        // UI toggle for log drawer
        Message::ToggleLogDrawer => {
            state.logs_open = !state.logs_open;
            needs_refresh = true;
            commands.push(Command::UpdateUI(Box::new(|| {
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::log_drawer::refresh(&doc);
                }
            })));
        }
        Message::WorkflowsLoaded(workflows) => {
            state.workflows.clear();
            for wf in workflows {
                state.workflows.insert(wf.id, wf);
            }
            // Ensure current_workflow_id is set
            if state.current_workflow_id.is_none() {
                if let Some(first_id) = state.workflows.keys().next().cloned() {
                    state.current_workflow_id = Some(first_id);
                }
            }
            // Trigger UI refresh of workflow switcher
            commands.push(Command::UpdateUI(Box::new(|| {
                if let (Some(win),) = (web_sys::window(),) {
                    if let Some(doc) = win.document() {
                        let _ = crate::components::workflow_switcher::refresh(&doc);
                    }
                }
            })));
        }
        Message::AddEdge { .. } |
        Message::GenerateCanvasFromAgents => {
            // These are handled by the canvas reducer which returns early
            unreachable!("Canvas messages should be handled by the canvas reducer")
        }
        
        Message::InitializeParticleSystem { width, height } => {
            state.particle_system = Some(crate::canvas::background::ParticleSystem::new(width, height, 50));
            needs_refresh = true; // Particle system initialized, likely needs a redraw
        },
        Message::ClearParticleSystem => {
            state.particle_system = None;
            needs_refresh = true; // Particle system cleared, likely needs a redraw
        },
        // Catch-all for any unhandled messages
        _ => {
            // Warn about unexpected messages so nothing silently fails
            web_sys::console::warn_1(&format!(
                "[update.rs] Unexpected or unhandled message: {:?}", msg
            ).into());
            needs_refresh = false;
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
