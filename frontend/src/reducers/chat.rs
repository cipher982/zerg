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
                state.threads.insert(thread_id, thread.clone());
                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::SelectThread(thread_id),
                ));
            } else {
                web_sys::console::error_1(
                    &format!("ThreadCreated message missing thread ID: {:?}", thread).into(),
                );
            }
            true
        }
        crate::messages::Message::DeleteThread(thread_id) => {
            // Delete the thread optimistically
            state.threads.remove(thread_id);
            state.thread_messages.remove(thread_id);

            // If this was the current thread, clear the current thread
            if state.current_thread_id == Some(*thread_id) {
                state.current_thread_id = None;
            }

            // Collect thread data for UI updates
            let threads: Vec<crate::models::ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();

            // Store updates to be executed after the borrow is released
            let threads_clone = threads.clone();
            let thread_messages_clone = thread_messages.clone();

            cmds.push(Command::UpdateUI(Box::new(move || {
                if let Some(_document) = web_sys::window()
                    .expect("no global window exists")
                    .document()
                {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::UpdateThreadList(
                            threads_clone,
                            current_thread_id,
                            thread_messages_clone,
                        ),
                    );
                    if current_thread_id.is_none() {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateConversation(Vec::new()),
                        );
                    }
                }
            })));

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
            web_sys::console::log_1(&format!("State: Selecting thread {}", thread_id).into());
            if state.current_thread_id != Some(*thread_id) {
                state.current_thread_id = Some(*thread_id);
                state.is_chat_loading = true;
                state.thread_messages.remove(thread_id);
                state.active_streams.remove(thread_id);

                let selected_thread_title = state
                    .threads
                    .get(thread_id)
                    .expect("Selected thread not found in state")
                    .title
                    .clone();

                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::LoadThreadMessages(*thread_id),
                ));

                let threads: Vec<crate::models::ApiThread> =
                    state.threads.values().cloned().collect();
                let current_thread_id = state.current_thread_id;
                let thread_messages = state.thread_messages.clone();

                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::UpdateThreadList(
                        threads,
                        current_thread_id,
                        thread_messages,
                    ),
                ));

                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::UpdateThreadTitleUI(selected_thread_title),
                ));

                let topic_manager_clone = state.topic_manager.clone();
                let thread_id_clone = *thread_id;
                cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                    if let Err(e) = crate::components::chat::init_chat_view_ws(
                        thread_id_clone,
                        topic_manager_clone,
                    ) {
                        web_sys::console::error_1(
                            &format!(
                                "Failed to initialize WebSocket for thread {}: {:?}",
                                thread_id_clone, e
                            )
                            .into(),
                        );
                    } else {
                        web_sys::console::log_1(
                            &format!(
                                "Initialized WebSocket subscription for thread {}",
                                thread_id_clone
                            )
                            .into(),
                        );
                    }
                })));
            }
            true
        }
        crate::messages::Message::LoadThreadMessages(thread_id) => {
            state.is_chat_loading = true;
            cmds.push(crate::messages::Command::FetchThreadMessages(*thread_id));
            true
        }
        crate::messages::Message::ThreadMessagesLoaded(thread_id, messages) => {
            state.thread_messages.insert(*thread_id, messages.clone());
            if state.current_thread_id == Some(*thread_id) {
                let messages_clone = messages.clone();
                cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                        let current_user_opt =
                            crate::state::APP_STATE.with(|s| s.borrow().current_user.clone());
                        let _ = crate::components::chat_view::update_conversation_ui(
                            &document,
                            &messages_clone,
                            current_user_opt.as_ref(),
                        );
                    }
                })));
            }
            let threads_data: Vec<crate::models::ApiThread> =
                state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                crate::state::dispatch_global_message(crate::messages::Message::UpdateThreadList(
                    threads_data.clone(),
                    current_thread_id,
                    thread_messages.clone(),
                ));
            })));
            true
        }
        crate::messages::Message::UpdateThreadTitle(thread_id, title) => {
            if let Some(thread) = state.threads.get_mut(thread_id) {
                thread.title = title.clone();
            }
            let threads: Vec<crate::models::ApiThread> = state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
            let title_clone = title.clone();
            let thread_id_clone = *thread_id;
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                if let Some(_document) = web_sys::window()
                    .expect("no global window exists")
                    .document()
                {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::UpdateThreadList(
                            threads.clone(),
                            current_thread_id,
                            thread_messages.clone(),
                        ),
                    );
                    if current_thread_id == Some(thread_id_clone) {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateThreadTitleUI(title_clone.clone()),
                        );
                    }
                }
            })));
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
            let thread_id_opt = state.current_thread_id;
            if let Some(thread_id) = thread_id_opt {
                let title_clone = title.clone();
                cmds.push(crate::messages::Command::SendMessage(
                    crate::messages::Message::UpdateThreadTitle(thread_id, title_clone),
                ));
            }
            true
        }
        crate::messages::Message::RequestThreadListUpdate(agent_id) => {
            let threads: Vec<crate::models::ApiThread> = state
                .threads
                .values()
                .filter(|t| t.agent_id == *agent_id)
                .cloned()
                .collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();
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
                let messages = state.thread_messages.entry(*thread_id).or_default();
                let need_new_bubble = match messages.last() {
                    Some(last) => {
                        last.role != "assistant" || !state.streaming_threads.contains(thread_id)
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
                let messages = state.thread_messages.entry(*thread_id).or_default();
                messages.push(tool_message);
                web_sys::console::log_1(
                    &format!("Added tool message for thread {}: {}", thread_id, content).into(),
                );
            } else {
                let mid_u32 = message_id.as_ref().and_then(|s| s.parse::<u32>().ok());
                let current_mid = state.current_assistant_id(*thread_id);
                let start_new = current_mid != mid_u32;
                let messages = state.thread_messages.entry(*thread_id).or_default();
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
            if state.current_thread_id == Some(*thread_id) {
                if let Some(messages) = state.thread_messages.get(thread_id) {
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
            if let Some(messages) = state.thread_messages.get_mut(thread_id) {
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
                if state.current_thread_id == Some(*thread_id) {
                    let messages_clone = messages.clone();
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateConversation(messages_clone),
                        );
                    })));
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
            if let Some(messages) = state.thread_messages.get_mut(thread_id) {
                if let Some(last) = messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.role == "assistant" && m.id.is_none())
                {
                    last.id = Some(*message_id);
                }
            }
            true
        }
        crate::messages::Message::ReceiveNewMessage(message) => {
            let thread_id = message.thread_id;
            let messages = state.thread_messages.entry(thread_id).or_default();
            messages.push(message.clone());
            if state.current_thread_id == Some(thread_id) {
                let messages_clone = messages.clone();
                cmds.push(Command::UpdateUI(Box::new(move || {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::UpdateConversation(messages_clone),
                    );
                })));
            }
            let threads_data: Vec<crate::models::ApiThread> =
                state.threads.values().cloned().collect();
            let thread_messages_map = state.thread_messages.clone();
            let current_thread_id = state.current_thread_id;
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                crate::state::dispatch_global_message(crate::messages::Message::UpdateThreadList(
                    threads_data,
                    current_thread_id,
                    thread_messages_map,
                ));
            })));
            true
        }
        crate::messages::Message::ReceiveThreadUpdate { thread_id, title } => {
            if let Some(thread) = state.threads.get_mut(thread_id) {
                if let Some(new_title) = title {
                    thread.title = new_title.clone();
                }
                if state.current_thread_id == Some(*thread_id) {
                    let title_clone = thread.title.clone();
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateThreadTitleUI(title_clone),
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
            if let Some(active_thread_id) = state.current_thread_id {
                let messages_clone_for_dispatch = messages.clone();
                state
                    .thread_messages
                    .insert(active_thread_id, messages.clone());
                cmds.push(Command::UpdateUI(Box::new(move || {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::UpdateConversation(messages_clone_for_dispatch),
                    );
                })));
            } else {
                web_sys::console::warn_1(
                    &"Received thread history but no active thread selected in state.".into(),
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
            if let Some(thread_id) = state.current_thread_id {
                if let Some(messages) = state.thread_messages.get(&thread_id) {
                    let messages_clone = messages.clone();
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateConversation(messages_clone),
                        );
                    })));
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
            if let Some(thread_id) = state.current_thread_id {
                if let Some(messages) = state.thread_messages.get(&thread_id) {
                    let messages_clone = messages.clone();
                    cmds.push(Command::UpdateUI(Box::new(move || {
                        crate::state::dispatch_global_message(
                            crate::messages::Message::UpdateConversation(messages_clone),
                        );
                    })));
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
        crate::messages::Message::NavigateToChatView(agent_id) => {
            state.active_view = crate::storage::ActiveView::ChatView;
            state.is_chat_loading = true;
            state.current_agent_id = Some(*agent_id);
            let agent_id_for_effects = *agent_id;
            cmds.push(Command::UpdateUI(Box::new(move || {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    let _ = crate::components::chat_view::setup_chat_view(&document);
                    let _ = crate::components::chat_view::show_chat_view(
                        &document,
                        agent_id_for_effects,
                    );
                }
                wasm_bindgen_futures::spawn_local(async move {
                    match crate::network::api_client::ApiClient::get_threads(Some(
                        agent_id_for_effects,
                    ))
                    .await
                    {
                        Ok(response) => {
                            match serde_json::from_str::<Vec<crate::models::ApiThread>>(&response) {
                                Ok(threads) => {
                                    crate::state::dispatch_global_message(
                                        crate::messages::Message::ThreadsLoaded(threads),
                                    );
                                }
                                Err(e) => {
                                    web_sys::console::error_1(
                                        &format!("Failed to parse threads: {:?}", e).into(),
                                    );
                                    crate::state::dispatch_global_message(
                                        crate::messages::Message::UpdateLoadingState(false),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to load threads: {:?}", e).into(),
                            );
                            crate::state::dispatch_global_message(
                                crate::messages::Message::UpdateLoadingState(false),
                            );
                        }
                    }
                });
            })));
            true
        }
        crate::messages::Message::NavigateToThreadView(thread_id) => {
            state.current_thread_id = Some(*thread_id);
            if let Some(thread) = state.threads.get(thread_id) {
                let agent_id = thread.agent_id;
                crate::state::dispatch_global_message(
                    crate::messages::Message::NavigateToChatView(agent_id),
                );
            }
            true
        }
        crate::messages::Message::RequestNewThread => {
            let agent_id_opt = state
                .current_thread_id
                .and_then(|thread_id| state.threads.get(&thread_id))
                .map(|thread| thread.agent_id)
                .or(state.current_agent_id);
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
            if let Some(thread_id) = state.current_thread_id {
                let ui_callback = crate::thread_handlers::handle_send_thread_message(
                    thread_id,
                    content.clone(),
                    &mut state.thread_messages,
                    &state.threads,
                    state.current_thread_id,
                );
                cmds.push(crate::messages::Command::UpdateUI(ui_callback));
            } else {
                web_sys::console::error_1(&"RequestSendMessage but no current_thread_id".into());
            }
            true
        }
        crate::messages::Message::SendThreadMessage(_, _) => {
            web_sys::console::warn_1(
                &"Received legacy SendThreadMessage; ignoring to avoid duplicate network call"
                    .into(),
            );
            true
        }
        crate::messages::Message::ThreadMessageSent(_response, _client_id) => {
            web_sys::console::warn_1(
                &"ThreadMessageSent is deprecated, use ThreadMessagesLoaded instead".into(),
            );
            true
        }
        crate::messages::Message::ThreadMessageFailed(_thread_id, _client_id) => {
            web_sys::console::warn_1(&"ThreadMessageFailed is deprecated".into());
            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                crate::toast::error("Message failed to send. Please try again.");
            })));
            true
        }
        crate::messages::Message::LoadThreads(agent_id) => {
            web_sys::console::log_1(&format!("Loading threads for agent: {}", agent_id).into());
            cmds.push(crate::messages::Command::FetchThreads(*agent_id));
            true
        }
        crate::messages::Message::ThreadsLoaded(threads) => {
            web_sys::console::log_1(&format!("Threads loaded: {} threads", threads.len()).into());

            // Update state with loaded threads
            for thread in threads {
                if let Some(thread_id) = thread.id {
                    state.threads.insert(thread_id, thread.clone());
                }
            }

            // Auto-select the first thread if none is selected and threads exist
            if state.current_thread_id.is_none() && !state.threads.is_empty() {
                // Sort threads by updated_at (newest first), fallback to created_at
                let mut threads_vec: Vec<_> = state.threads.values().collect();
                threads_vec.sort_by(|a, b| {
                    let a_time = a
                        .updated_at
                        .as_ref()
                        .or(a.created_at.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let b_time = b
                        .updated_at
                        .as_ref()
                        .or(b.created_at.as_ref())
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    b_time.cmp(a_time)
                });
                if let Some(first_thread) = threads_vec.first() {
                    if let Some(first_thread_id) = first_thread.id {
                        cmds.push(crate::messages::Command::SendMessage(
                            crate::messages::Message::SelectThread(first_thread_id),
                        ));
                    }
                }
            }

            // Update UI with thread list
            let threads_data: Vec<crate::models::ApiThread> =
                state.threads.values().cloned().collect();
            let current_thread_id = state.current_thread_id;
            let thread_messages = state.thread_messages.clone();

            cmds.push(crate::messages::Command::UpdateUI(Box::new(move || {
                crate::state::dispatch_global_message(crate::messages::Message::UpdateThreadList(
                    threads_data,
                    current_thread_id,
                    thread_messages,
                ));
            })));

            // Stop loading state
            cmds.push(crate::messages::Command::SendMessage(
                crate::messages::Message::UpdateLoadingState(false),
            ));

            true
        }
        _ => false,
    }
}
