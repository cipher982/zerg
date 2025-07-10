//! Chat-view specific reducer split out from `update.rs`.
//!
//! For the moment this file only contains *place-holders* so the project
//! compiles.  Real variants and logic will move here incrementally over the
//! next refactor commits.

use crate::messages::{Command, Message};
use crate::state::AppState;

/// Returns `true` when the message was handled by the chat reducer.
pub fn update(state: &mut AppState, msg: &Message, cmds: &mut Vec<Command>) -> bool {
    match msg {
        crate::messages::Message::CreateThread(agent_id, title) => {
            // Log for debugging
            web_sys::console::log_1(
                &format!(
                    "Creating thread for agent {} with title: {}",
                    agent_id, title
                )
                .into(),
            );
            // Instead of spawning directly, return a command that will be executed after state update
            cmds.push(crate::messages::Command::CreateThread {
                agent_id: *agent_id,
                title: title.clone(),
            });
            true
        }
        crate::messages::Message::ThreadCreated(thread) => {
            web_sys::console::log_1(
                &format!("Update: Handling ThreadCreated: {:?}", thread).into(),
            );
            if let Some(thread_id) = thread.id {
                let agent_id = thread.agent_id;
                
                // Add thread to agent-scoped state
                let agent_state = state.ensure_agent_state(agent_id);
                agent_state.threads.insert(thread_id, thread.clone());
                
                // Select the new thread in agent context
                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::SelectAgentThread { 
                        agent_id, 
                        thread_id 
                    },
                ));
            } else {
                web_sys::console::error_1(
                    &format!("ThreadCreated message missing thread ID: {:?}", thread).into(),
                );
            }
            true
        }
        crate::messages::Message::DeleteThread(thread_id) => {
            // Find which agent owns this thread and delete from agent state
            let mut agent_id_opt = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    agent_id_opt = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = agent_id_opt {
                if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    // Delete the thread from agent state
                    agent_state.threads.remove(thread_id);
                    agent_state.thread_messages.remove(thread_id);

                    // If this was the current thread, clear it
                    if agent_state.current_thread_id == Some(*thread_id) {
                        agent_state.current_thread_id = None;
                    }

                    // Update UI with new thread list
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::RequestThreadListUpdate(agent_id),
                        );
                    })));
                }
            }

            // Send the delete request to the backend
            let thread_id_clone = *thread_id;
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::delete_thread(thread_id_clone).await {
                    Ok(_) => {}
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to delete thread: {:?}", e).into(),
                        );
                    }
                }
            });

            true
        }
        crate::messages::Message::SelectThread(thread_id) => {
            web_sys::console::warn_1(&"DEPRECATED: SelectThread called. Use SelectAgentThread instead.".into());
            
            // Find which agent owns this thread
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    return update(state, &crate::messages::Message::SelectAgentThread { 
                        agent_id: *agent_id, 
                        thread_id: *thread_id 
                    }, cmds);
                }
            }
            
            web_sys::console::error_1(&format!("Thread {} not found in any agent state", thread_id).into());
            true
        }
        crate::messages::Message::LoadThreadMessages(thread_id) => {
            state.is_chat_loading = true;
            cmds.push(crate::messages::Command::FetchThreadMessages(*thread_id));
            true
        }
        crate::messages::Message::ThreadMessagesLoaded(thread_id, messages) => {
            // Find which agent owns this thread and update agent-scoped state
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                let current_agent_id = state.current_agent_id;
                let should_update_ui = if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    agent_state.thread_messages.insert(*thread_id, messages.clone());
                    
                    // Check if UI should be updated
                    current_agent_id == Some(agent_id) && 
                    agent_state.current_thread_id == Some(*thread_id)
                } else {
                    false
                };
                
                if should_update_ui {
                    // Use agent-scoped UI refresh instead of global dispatches
                    cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                        crate::components::chat_view::refresh_chat_ui_from_agent_state(agent_id);
                    })));
                }
            } else {
                web_sys::console::warn_1(&format!("ThreadMessagesLoaded: No agent found owning thread {}", thread_id).into());
            }
            true
        }
        crate::messages::Message::UpdateThreadTitle(thread_id, title) => {
            // Find which agent owns this thread and update agent-scoped state
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    if let Some(thread) = agent_state.threads.get_mut(thread_id) {
                        thread.title = title.clone();
                    }
                    
                    let title_clone = title.clone();
                    let thread_id_clone = *thread_id;
                    cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                        // Update thread list for this agent
                        crate::state::dispatch_global_message(
                            crate::messages::Message::RequestThreadListUpdate(agent_id)
                        );
                        
                        // Update title UI if this is the current thread
                        if let Some(current_agent_state) = crate::state::APP_STATE.with(|s| {
                            s.borrow().get_agent_state(agent_id).map(|as_| as_.clone())
                        }) {
                            if current_agent_state.current_thread_id == Some(thread_id_clone) {
                                crate::state::dispatch_global_message(
                                    crate::messages::Message::UpdateThreadTitleUI(title_clone.clone()),
                                );
                            }
                        }
                    })));
                }
            }
            
            // Send update to backend
            cmds.push(crate::messages::Command::UpdateThreadTitle {
                thread_id: *thread_id,
                title: title.clone(),
            });
            true
        }
        crate::messages::Message::UpdateThreadList(threads, current_thread_id, thread_messages) => {
            if let Some(document) = web_sys::window()
                .expect("no global window exists")
                .document()
            {
                let _ = crate::components::chat_view::update_thread_list_ui(
                    &document,
                    &threads,
                    *current_thread_id,
                    &thread_messages,
                );
            }
            true
        }
        crate::messages::Message::UpdateConversation(messages) => {
            let messages_clone = messages.clone();
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window().unwrap().document() {
                    let current_user_opt =
                        crate::state::APP_STATE.with(|s| s.borrow().current_user.clone());
                    let _ = crate::components::chat_view::update_conversation_ui(
                        &document,
                        &messages_clone,
                        current_user_opt.as_ref(),
                    );
                }
            })));
            true
        }
        crate::messages::Message::UpdateThreadTitleUI(title) => {
            let title_clone = title.clone();
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window()
                    .expect("no global window exists")
                    .document()
                {
                    let _ = crate::components::chat_view::update_thread_title_with_data(
                        &document,
                        &title_clone,
                    );
                }
            })));
            true
        }
        crate::messages::Message::RequestUpdateThreadTitle(title) => {
            // Use current agent's current thread
            if let Some(agent_state) = state.current_agent() {
                if let Some(thread_id) = agent_state.current_thread_id {
                    let title_clone = title.clone();
                    cmds.push(crate::messages::Command::SendMessage(
                        crate::messages::Message::UpdateThreadTitle(thread_id, title_clone),
                    ));
                }
            }
            true
        }
        crate::messages::Message::RequestThreadListUpdate(agent_id) => {
            // Use agent-scoped state only
            let threads: Vec<crate::models::ApiThread> = if let Some(agent_state) = state.get_agent_state(*agent_id) {
                agent_state.get_threads_sorted().into_iter().cloned().collect()
            } else {
                // No agent state found - return empty list
                Vec::new()
            };
            
            let current_thread_id = if let Some(agent_state) = state.get_agent_state(*agent_id) {
                agent_state.current_thread_id
            } else {
                None
            };
            
            let thread_messages = if let Some(agent_state) = state.get_agent_state(*agent_id) {
                agent_state.thread_messages.clone()
            } else {
                std::collections::HashMap::new()
            };
            
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                crate::state::dispatch_global_message(crate::messages::Message::UpdateThreadList(
                    threads,
                    current_thread_id,
                    thread_messages,
                ));
            })));
            true
        }
        crate::messages::Message::ReceiveStreamChunk {
            thread_id,
            content,
            chunk_type,
            tool_name,
            tool_call_id,
            message_id,
        } => {
            // (migrated logic from update.rs)
            if chunk_type.as_deref() == Some("assistant_token") {
                state.token_mode_threads.insert(*thread_id);
                
                // Find which agent owns this thread
                let mut owning_agent_id = None;
                for (agent_id, agent_state) in &state.agent_states {
                    if agent_state.threads.contains_key(thread_id) {
                        owning_agent_id = Some(*agent_id);
                        break;
                    }
                }
                
                if let Some(agent_id) = owning_agent_id {
                    let is_streaming = state.streaming_threads.contains(thread_id);
                    if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                        let messages = agent_state.thread_messages.entry(*thread_id).or_default();
                        let need_new_bubble = match messages.last() {
                            Some(last) => {
                                last.role != "assistant" || !is_streaming
                            }
                            None => true,
                        };
                        if need_new_bubble {
                            let now = chrono::Utc::now().to_rfc3339();
                            messages.push(crate::models::ApiThreadMessage {
                                id: None,
                                thread_id: *thread_id,
                                role: "assistant".to_string(),
                                content: content.clone(),
                                timestamp: Some(now),
                                message_type: Some("assistant_message".to_string()),
                                tool_name: None,
                                tool_call_id: None,
                                tool_input: None,
                                parent_id: None,
                            });
                        } else if let Some(last) = messages.last_mut() {
                            last.content.push_str(content);
                        }
                    }
                } else {
                    web_sys::console::warn_1(&format!("ReceiveStreamChunk: No agent found owning thread {}", thread_id).into());
                }
            } else if chunk_type.as_deref() == Some("tool_output") {
                let now = chrono::Utc::now().to_rfc3339();
                let parent_id = state.current_assistant_id(*thread_id);
                let tool_message = crate::models::ApiThreadMessage {
                    id: None,
                    thread_id: *thread_id,
                    role: "tool".to_string(),
                    content: content.clone(),
                    timestamp: Some(now),
                    message_type: chunk_type.clone(),
                    tool_name: tool_name.clone(),
                    tool_call_id: tool_call_id.clone(),
                    tool_input: None,
                    parent_id,
                };
                
                // Find which agent owns this thread
                let mut owning_agent_id = None;
                for (agent_id, agent_state) in &state.agent_states {
                    if agent_state.threads.contains_key(thread_id) {
                        owning_agent_id = Some(*agent_id);
                        break;
                    }
                }
                
                if let Some(agent_id) = owning_agent_id {
                    if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                        let messages = agent_state.thread_messages.entry(*thread_id).or_default();
                        messages.push(tool_message);
                        web_sys::console::log_1(
                            &format!("Added tool message for thread {}: {}", thread_id, content).into(),
                        );
                    }
                } else {
                    web_sys::console::warn_1(&format!("ReceiveStreamChunk (tool_output): No agent found owning thread {}", thread_id).into());
                }
            } else {
                let mid_u32 = message_id.as_ref().and_then(|s| s.parse::<u32>().ok());
                let current_mid = state.current_assistant_id(*thread_id);
                let start_new = current_mid != mid_u32;
                
                // Find which agent owns this thread
                let mut owning_agent_id = None;
                for (agent_id, agent_state) in &state.agent_states {
                    if agent_state.threads.contains_key(thread_id) {
                        owning_agent_id = Some(*agent_id);
                        break;
                    }
                }
                
                if let Some(agent_id) = owning_agent_id {
                    if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                        let messages = agent_state.thread_messages.entry(*thread_id).or_default();
                        if start_new {
                            web_sys::console::log_1(&"Update: starting NEW assistant bubble".into());
                            let now = chrono::Utc::now().to_rfc3339();
                            let assistant_message = crate::models::ApiThreadMessage {
                                id: mid_u32,
                                thread_id: *thread_id,
                                role: "assistant".to_string(),
                                content: content.clone(),
                                timestamp: Some(now),
                                message_type: chunk_type.clone(),
                                tool_name: None,
                                tool_call_id: None,
                                tool_input: None,
                                parent_id: None,
                            };
                            messages.push(assistant_message);
                            state.active_streams.insert(*thread_id, mid_u32);
                        } else if let Some(last_message) = messages.last_mut() {
                            last_message.content.push_str(content);
                        }
                    }
                } else {
                    web_sys::console::warn_1(&format!("ReceiveStreamChunk (assistant): No agent found owning thread {}", thread_id).into());
                }
            }
            // Update UI if this is the current agent's current thread
            if let Some(agent_state) = state.current_agent() {
                if agent_state.current_thread_id == Some(*thread_id) {
                    if let Some(messages) = agent_state.thread_messages.get(thread_id) {
                        web_sys::console::log_1(&format!(
                            "[DEBUG] Scheduling UI update after ReceiveStreamChunk for thread {} ({} messages)",
                            thread_id,
                            messages.len()
                        ).into());
                        let messages_clone = messages.clone();
                        cmds.push(Command::UpdateUI(Box::new(move || {
                            crate::state::dispatch_global_message(
                                crate::messages::Message::UpdateConversation(messages_clone),
                            );
                        })));
                    }
                }
            }
            true
        }
        crate::messages::Message::ReceiveStreamStart(thread_id) => {
            state.streaming_threads.insert(*thread_id);
            state.active_streams.insert(*thread_id, None);
            web_sys::console::log_1(&format!("Stream started for thread {}.", thread_id).into());
            true
        }
        crate::messages::Message::ReceiveStreamEnd(thread_id) => {
            state.streaming_threads.remove(thread_id);
            state.token_mode_threads.remove(thread_id);
            
            // Find which agent owns this thread
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                let current_agent_id = state.current_agent_id;
                let mut should_update_ui = false;
                let mut messages_for_ui = None;
                
                if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    if let Some(messages) = agent_state.thread_messages.get_mut(thread_id) {
                        if let Some(last_user_message) =
                            messages.iter_mut().filter(|msg| msg.role == "user").last()
                        {
                            last_user_message.id = None;
                            web_sys::console::log_1(
                                &format!(
                                    "Stream ended: Set last user message ID to None for thread {}.",
                                    thread_id
                                )
                                .into(),
                            );
                        }
                        
                        // Check if UI should be updated
                        if current_agent_id == Some(agent_id) && 
                           agent_state.current_thread_id == Some(*thread_id) {
                            should_update_ui = true;
                            messages_for_ui = Some(messages.clone());
                        }
                    }
                }
                
                if should_update_ui {
                    if let Some(messages_clone) = messages_for_ui {
                        cmds.push(Command::UpdateUI(Box::new(move || {
                            crate::state::dispatch_global_message(
                                crate::messages::Message::UpdateConversation(messages_clone),
                            );
                        })));
                    }
                }
            }
            web_sys::console::log_1(&format!("Stream ended for thread {}.", thread_id).into());
            true
        }
        crate::messages::Message::ReceiveAssistantId {
            thread_id,
            message_id,
        } => {
            web_sys::console::log_1(
                &format!(
                    "Received AssistantId {} for thread {}",
                    message_id, thread_id
                )
                .into(),
            );
            state.active_streams.insert(*thread_id, Some(*message_id));
            
            // Find which agent owns this thread
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    if let Some(messages) = agent_state.thread_messages.get_mut(thread_id) {
                        if let Some(last) = messages
                            .iter_mut()
                            .rev()
                            .find(|m| m.role == "assistant" && m.id.is_none())
                        {
                            last.id = Some(*message_id);
                        }
                    }
                }
            }
            true
        }
        crate::messages::Message::ReceiveNewMessage(message) => {
            let thread_id = message.thread_id;
            
            // Find which agent owns this thread and update agent-scoped state
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(&thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                let current_agent_id = state.current_agent_id;
                let should_update_ui = if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    let messages = agent_state.thread_messages.entry(thread_id).or_default();
                    messages.push(message.clone());
                    
                    // Check if UI should be updated
                    current_agent_id == Some(agent_id) && 
                    agent_state.current_thread_id == Some(thread_id)
                } else {
                    false
                };
                
                if should_update_ui {
                    // Get messages for UI update in a separate borrow
                    if let Some(agent_state) = state.get_agent_state(agent_id) {
                        if let Some(messages) = agent_state.thread_messages.get(&thread_id) {
                            let messages_clone = messages.clone();
                            cmds.push(Command::UpdateUI(Box::new(move || {
                                crate::state::dispatch_global_message(
                                    crate::messages::Message::UpdateConversation(messages_clone),
                                );
                            })));
                        }
                    }
                }
                
                // Update thread list for this agent
                cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::RequestThreadListUpdate(agent_id)
                    );
                })));
            } else {
                web_sys::console::warn_1(&format!("ReceiveNewMessage: No agent found owning thread {}", thread_id).into());
            }
            true
        }
        crate::messages::Message::ReceiveThreadUpdate { thread_id, title } => {
            // Find which agent owns this thread and update agent-scoped state
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                let current_agent_id = state.current_agent_id;
                let mut should_update_ui = false;
                let mut title_for_ui = String::new();
                
                if let Some(agent_state) = state.get_agent_state_mut(agent_id) {
                    if let Some(thread) = agent_state.threads.get_mut(thread_id) {
                        if let Some(new_title) = title {
                            thread.title = new_title.clone();
                        }
                        
                        // Check if UI should be updated
                        if current_agent_id == Some(agent_id) && 
                           agent_state.current_thread_id == Some(*thread_id) {
                            should_update_ui = true;
                            title_for_ui = thread.title.clone();
                        }
                    }
                }
                
                if should_update_ui {
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateThreadTitleUI(title_for_ui),
                        );
                    })));
                }
            }
            true
        }
        crate::messages::Message::ReceiveThreadHistory(messages) => {
            let system_messages_count = messages.iter().filter(|msg| msg.role == "system").count();
            if system_messages_count > 0 {
                web_sys::console::log_1(&format!("Thread history contains {} system messages which won't be displayed in the chat UI", system_messages_count).into());
            }
            web_sys::console::log_1(
                &format!(
                    "Update handler: Received thread history ({} messages, {} displayable)",
                    messages.len(),
                    messages.len() - system_messages_count
                )
                .into(),
            );
            
            // Use current agent's current thread
            if let Some(agent_state) = state.current_agent() {
                if let Some(active_thread_id) = agent_state.current_thread_id {
                    // Update agent-scoped thread messages
                    if let Some(agent_state_mut) = state.current_agent_mut() {
                        agent_state_mut.thread_messages.insert(active_thread_id, messages.clone());
                    }
                    
                    let messages_clone_for_dispatch = messages.clone();
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateConversation(messages_clone_for_dispatch),
                        );
                    })));
                } else {
                    web_sys::console::warn_1(
                        &"Received thread history but no active thread selected in current agent.".into(),
                    );
                }
            } else {
                web_sys::console::warn_1(
                    &"Received thread history but no active agent selected.".into(),
                );
            }
            true
        }
        crate::messages::Message::ToggleToolExpansion { tool_call_id } => {
            let entry = state.tool_ui_states.entry(tool_call_id.clone()).or_insert(
                crate::state::ToolUiState {
                    expanded: false,
                    show_full: false,
                },
            );
            entry.expanded = !entry.expanded;
            
            // Use current agent's current thread
            if let Some(agent_state) = state.current_agent() {
                if let Some(thread_id) = agent_state.current_thread_id {
                    if let Some(messages) = agent_state.thread_messages.get(&thread_id) {
                        let messages_clone = messages.clone();
                        cmds.push(Command::UpdateUI(Box::new(move || {
                            crate::state::dispatch_global_message(
                                crate::messages::Message::UpdateConversation(messages_clone),
                            );
                        })));
                    }
                }
            }
            true
        }
        crate::messages::Message::ToggleToolShowMore { tool_call_id } => {
            let entry = state.tool_ui_states.entry(tool_call_id.clone()).or_insert(
                crate::state::ToolUiState {
                    expanded: false,
                    show_full: false,
                },
            );
            entry.show_full = !entry.show_full;
            
            // Use current agent's current thread
            if let Some(agent_state) = state.current_agent() {
                if let Some(thread_id) = agent_state.current_thread_id {
                    if let Some(messages) = agent_state.thread_messages.get(&thread_id) {
                        let messages_clone = messages.clone();
                        cmds.push(Command::UpdateUI(Box::new(move || {
                            crate::state::dispatch_global_message(
                                crate::messages::Message::UpdateConversation(messages_clone),
                            );
                        })));
                    }
                }
            }
            true
        }
        crate::messages::Message::UpdateLoadingState(is_loading) => {
            state.is_chat_loading = *is_loading;
            if let Some(document) = web_sys::window()
                .expect("no global window exists")
                .document()
            {
                let _ = crate::components::chat_view::update_loading_state(&document, *is_loading);
            }
            true
        }
        crate::messages::Message::NavigateToThreadView(thread_id) => {
            // Find which agent owns this thread
            let mut owning_agent_id = None;
            for (agent_id, agent_state) in &state.agent_states {
                if agent_state.threads.contains_key(thread_id) {
                    owning_agent_id = Some(*agent_id);
                    break;
                }
            }
            
            if let Some(agent_id) = owning_agent_id {
                // Navigate to agent chat and select the thread
                crate::state::dispatch_global_message(
                    crate::messages::Message::NavigateToAgentChat(agent_id),
                );
                crate::state::dispatch_global_message(
                    crate::messages::Message::SelectAgentThread { 
                        agent_id, 
                        thread_id: *thread_id 
                    },
                );
            } else {
                web_sys::console::error_1(&format!("Thread {} not found in any agent state", thread_id).into());
            }
            true
        }
        crate::messages::Message::RequestNewThread => {
            let agent_id_opt = state.current_agent_id;
            web_sys::console::log_1(
                &format!("RequestNewThread - agent_id: {:?}", agent_id_opt).into(),
            );
            if let Some(agent_id) = agent_id_opt {
                let title = crate::constants::DEFAULT_THREAD_TITLE.to_string();
                web_sys::console::log_1(
                    &format!("Creating new thread for agent: {}", agent_id).into(),
                );
                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::CreateThread(agent_id, title),
                ));
            } else {
                web_sys::console::error_1(&"Cannot create thread: No agent selected".into());
            }
            true
        }
        crate::messages::Message::RequestSendMessage(content) => {
            if let Some(agent_id) = state.current_agent_id {
                if let Some(agent_state) = state.get_agent_state(agent_id) {
                    if let Some(thread_id) = agent_state.current_thread_id {
                        web_sys::console::log_1(&format!("Sending message to thread {}: {}", thread_id, content).into());
                        
                        // Send message via command
                        cmds.push(crate::messages::Command::SendThreadMessage {
                            thread_id,
                            content: content.clone(),
                            client_id: None,
                        });
                    } else {
                        web_sys::console::error_1(&"RequestSendMessage but no current thread selected".into());
                    }
                } else {
                    web_sys::console::error_1(&"RequestSendMessage but no agent state found".into());
                }
            } else {
                web_sys::console::error_1(&"RequestSendMessage but no current agent selected".into());
            }
            true
        }
        crate::messages::Message::SendThreadMessage(thread_id, content) => {
            web_sys::console::log_1(&format!("Processing SendThreadMessage for thread {}: {}", thread_id, content).into());
            // This message might be sent from UI components that expect it to trigger the command
            cmds.push(crate::messages::Command::SendThreadMessage {
                thread_id: *thread_id,
                content: content.clone(),
                client_id: None,
            });
            true
        }
        crate::messages::Message::ThreadMessageSent(_response, _client_id) => {
            web_sys::console::error_1(&"ThreadMessageSent is fully deprecated and should not be used".into());
            true
        }
        crate::messages::Message::ThreadMessageFailed(_thread_id, _client_id) => {
            web_sys::console::error_1(&"ThreadMessageFailed is fully deprecated and should not be used".into());
            // Remove functionality - this should be handled by proper error handling in commands
            true
        }
        crate::messages::Message::LoadThreads(agent_id) => {
            web_sys::console::log_1(&format!("Loading threads for agent: {}", agent_id).into());
            cmds.push(crate::messages::Command::FetchThreads(*agent_id));
            true
        }
        crate::messages::Message::ThreadsLoaded(_threads) => {
            web_sys::console::error_1(&"DEPRECATED: ThreadsLoaded should not be used. Use AgentThreadsLoaded instead.".into());
            // No longer redirecting - this should be fixed at the source
            true
        }
        
        // NEW: Agent-Scoped Thread Handlers
        crate::messages::Message::NavigateToAgentChat(agent_id) => {
            web_sys::console::log_1(&format!("NavigateToAgentChat: agent_id={}", agent_id).into());
            
            // Set up chat view and clean agent state
            state.active_view = crate::storage::ActiveView::ChatView;
            state.is_chat_loading = true;
            state.set_current_agent(*agent_id);
            
            let agent_id_for_effects = *agent_id;
            cmds.push(Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::chat_view::setup_chat_view(&document);
                    let _ = crate::components::chat_view::show_chat_view(&document, agent_id_for_effects);
                }
                
                // Dispatch agent-scoped thread loading
                crate::state::dispatch_global_message(
                    crate::messages::Message::LoadAgentThreads(agent_id_for_effects),
                );
            })));
            true
        }
        
        crate::messages::Message::LoadAgentThreads(agent_id) => {
            web_sys::console::log_1(&format!("LoadAgentThreads: agent_id={}", agent_id).into());
            
            let agent_id_for_fetch = *agent_id;
            wasm_bindgen_futures::spawn_local(async move {
                match crate::network::api_client::ApiClient::get_threads(Some(agent_id_for_fetch)).await {
                    Ok(response) => {
                        match serde_json::from_str::<Vec<crate::models::ApiThread>>(&response) {
                            Ok(threads) => {
                                crate::state::dispatch_global_message(
                                    crate::messages::Message::AgentThreadsLoaded {
                                        agent_id: agent_id_for_fetch,
                                        threads,
                                    },
                                );
                            }
                            Err(e) => {
                                web_sys::console::error_1(
                                    &format!("Failed to parse threads for agent {}: {:?}", agent_id_for_fetch, e).into(),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        web_sys::console::error_1(
                            &format!("Failed to load threads for agent {}: {:?}", agent_id_for_fetch, e).into(),
                        );
                    }
                }
            });
            true
        }
        
        crate::messages::Message::AgentThreadsLoaded { agent_id, threads } => {
            web_sys::console::log_1(&format!("AgentThreadsLoaded: agent_id={}, {} threads", agent_id, threads.len()).into());
            
            // Get or create agent state
            let agent_state = state.ensure_agent_state(*agent_id);
            
            // Clear existing threads and load new ones (clean slate)
            agent_state.clear_threads();
            
            // Add new threads
            for thread in threads {
                if let Some(thread_id) = thread.id {
                    agent_state.threads.insert(thread_id, thread.clone());
                }
            }
            
            // Auto-select the first thread if none is selected
            if agent_state.current_thread_id.is_none() && !agent_state.threads.is_empty() {
                let threads_sorted = agent_state.get_threads_sorted();
                if let Some(first_thread) = threads_sorted.first() {
                    if let Some(thread_id) = first_thread.id {
                        agent_state.current_thread_id = Some(thread_id);
                    }
                }
            }
            
            // Update UI - agent-scoped state is already updated above
            state.is_chat_loading = false;
            
            // Trigger UI refresh for the current agent
            let current_agent_id = *agent_id;
            cmds.push(Command::UpdateUI(Box::new(move || {
                crate::components::chat_view::refresh_chat_ui_from_agent_state(current_agent_id);
            })));
            true
        }
        
        crate::messages::Message::SelectAgentThread { agent_id, thread_id } => {
            web_sys::console::log_1(&format!("SelectAgentThread: agent_id={}, thread_id={}", agent_id, thread_id).into());
            
            if let Some(agent_state) = state.get_agent_state_mut(*agent_id) {
                agent_state.current_thread_id = Some(*thread_id);
                
                // Update UI
                let current_agent_id = *agent_id;
                let current_thread_id = *thread_id;
                cmds.push(Command::UpdateUI(Box::new(move || {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::RequestThreadListUpdate(current_agent_id),
                    );
                    crate::state::dispatch_global_message(
                        crate::messages::Message::LoadThreadMessages(current_thread_id),
                    );
                })));
            }
            true
        }
        
        _ => false,
    }
}
