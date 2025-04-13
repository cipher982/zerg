use wasm_bindgen::prelude::*;
use web_sys::{Document, Event, HtmlInputElement};
use crate::state::dispatch_global_message;
use crate::messages::Message;
use crate::models::{ApiThread, ApiThreadMessage};
use wasm_bindgen::JsCast;
use crate::storage::ActiveView;
use std::collections::HashMap;

// Main function to setup the chat view
pub fn setup_chat_view(document: &Document) -> Result<(), JsValue> {
    // Create the chat view container if it doesn't exist
    if document.get_element_by_id("chat-view-container").is_none() {
        let chat_container = document.create_element("div")?;
        chat_container.set_id("chat-view-container");
        chat_container.set_class_name("chat-view-container");
        
        // Initially hide the chat container
        chat_container.set_attribute("style", "display: none;")?;
        
        // Add the chat layout structure
        chat_container.set_inner_html(r#"
            <div class="chat-header">
                <div class="back-button">←</div>
                <div class="agent-info">
                    <div class="agent-name"></div>
                    <span class="thread-title-label">Thread: </span><span class="thread-title-text"></span>
                </div>
            </div>
            <div class="chat-body">
                <div class="thread-sidebar">
                    <div class="sidebar-header">
                        <h3>Threads</h3>
                        <button class="new-thread-btn">New Thread</button>
                    </div>
                    <div class="thread-list"></div>
                </div>
                <div class="conversation-area">
                    <div class="messages-container"></div>
                </div>
            </div>
            <div class="chat-input-area">
                <input type="text" class="chat-input" placeholder="Type your message...">
                <button class="send-button">Send</button>
            </div>
        "#);
        
        // Add the chat container to the app container
        let app_container = document
            .get_element_by_id("app-container")
            .ok_or(JsValue::from_str("Could not find app-container"))?;
        
        app_container.append_child(&chat_container)?;
        
        // Setup event handlers
        setup_chat_event_handlers(document)?;
    }
    
    Ok(())
}

// Setup event handlers for the chat view
fn setup_chat_event_handlers(document: &Document) -> Result<(), JsValue> {
    // Back button handler
    if let Some(back_button) = document.query_selector(".back-button")? {
        let back_handler = Closure::wrap(Box::new(move |_: Event| {
            dispatch_global_message(Message::NavigateToDashboard);
        }) as Box<dyn FnMut(_)>);
        
        back_button.add_event_listener_with_callback("click", back_handler.as_ref().unchecked_ref())?;
        back_handler.forget();
    }
    
    // New thread button handler
    if let Some(new_thread_btn) = document.query_selector(".new-thread-btn")? {
        let new_thread_handler = Closure::wrap(Box::new(move |_: Event| {
            // Request current agent ID and create thread
            dispatch_global_message(Message::RequestNewThread);
        }) as Box<dyn FnMut(_)>);
        
        new_thread_btn.add_event_listener_with_callback("click", new_thread_handler.as_ref().unchecked_ref())?;
        new_thread_handler.forget();
    }
    
    // Send message button handler
    if let Some(send_button) = document.query_selector(".send-button")? {
        let document_clone = document.clone();
        let send_handler = Closure::wrap(Box::new(move |_: Event| {
            if let Some(input_el) = document_clone.query_selector(".chat-input").ok().flatten() {
                let input = input_el.dyn_into::<HtmlInputElement>().unwrap();
                let message = input.value();
                
                if !message.is_empty() {
                    // Request to send message to current thread
                    dispatch_global_message(Message::RequestSendMessage(message.clone()));
                    input.set_value("");
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        send_button.add_event_listener_with_callback("click", send_handler.as_ref().unchecked_ref())?;
        send_handler.forget();
    }
    
    // Chat input key press handler
    if let Some(input_el) = document.query_selector(".chat-input")? {
        let document_clone = document.clone();
        let keypress_handler = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Enter" {
                e.prevent_default();
                
                let input = document_clone.query_selector(".chat-input").ok().flatten()
                    .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
                    .unwrap();
                
                let message = input.value();
                
                if !message.is_empty() {
                    // Request to send message to current thread
                    dispatch_global_message(Message::RequestSendMessage(message.clone()));
                    input.set_value("");
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        input_el.add_event_listener_with_callback("keypress", keypress_handler.as_ref().unchecked_ref())?;
        keypress_handler.forget();
    }
    
    Ok(())
}

// Function to show the chat view
pub fn show_chat_view(document: &Document, agent_id: u32) -> Result<(), JsValue> {
    // Hide other views
    if let Some(dashboard) = document.get_element_by_id("dashboard-container") {
        dashboard.set_attribute("style", "display: none;")?;
    }
    
    if let Some(canvas) = document.get_element_by_id("canvas-container") {
        canvas.set_attribute("style", "display: none;")?;
    }
    
    // Show chat view
    if let Some(chat_view) = document.get_element_by_id("chat-view-container") {
        chat_view.set_attribute("style", "display: flex;")?;
    }
    
    // IMPORTANT: Instead of directly dispatching messages here, which would cause
    // a recursive borrow of APP_STATE, use setTimeout to schedule these dispatches
    // to occur after the current message handler has completed
    let agent_id_clone = agent_id;
    let load_agent_info = Closure::once(Box::new(move || {
        dispatch_global_message(Message::LoadAgentInfo(agent_id_clone));
    }) as Box<dyn FnOnce()>);
    
    let agent_id_clone = agent_id;
    let load_threads = Closure::once(Box::new(move || {
        dispatch_global_message(Message::LoadThreads(agent_id_clone));
    }) as Box<dyn FnOnce()>);
    
    // Schedule the dispatches to occur after the current function returns
    web_sys::window()
        .expect("no global window exists")
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            load_agent_info.as_ref().unchecked_ref(),
            0
        )?;
    
    web_sys::window()
        .expect("no global window exists")
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            load_threads.as_ref().unchecked_ref(),
            0
        )?;
    
    // Prevent these closures from being garbage collected
    load_agent_info.forget();
    load_threads.forget();
    
    Ok(())
}

// Update agent info in the UI when received
pub fn update_agent_info(document: &Document, agent_name: &str) -> Result<(), JsValue> {
    if let Some(agent_name_el) = document.query_selector(".agent-name").ok().flatten() {
        agent_name_el.set_text_content(Some(agent_name));
    }
    Ok(())
}

// Update the thread list in the sidebar when thread data is received
pub fn update_thread_list_ui(
    document: &Document,
    threads: &[ApiThread],
    current_thread_id: Option<u32>,
    thread_messages: &HashMap<u32, Vec<ApiThreadMessage>>
) -> Result<(), JsValue> {
    if let Some(thread_list) = document.query_selector(".thread-list").ok().flatten() {
        // Clear existing threads
        thread_list.set_inner_html("");

        // If no threads, show a message
        if threads.is_empty() {
            let empty_message = document.create_element("div")?;
            empty_message.set_class_name("thread-list-empty");
            empty_message.set_text_content(Some("No threads found"));
            thread_list.append_child(&empty_message)?;
            return Ok(());
        }

        // Sort threads by updated_at (newest first)
        let mut threads = threads.to_vec();
        threads.sort_by(|a, b| {
            b.updated_at.as_ref().unwrap_or(&"".to_string())
                .cmp(a.updated_at.as_ref().unwrap_or(&"".to_string()))
        });
        
        // Create thread items
        for thread in threads {
            if let Some(thread_id) = thread.id {
                let thread_item = document.create_element("div")?;
                
                // Add selected class if this is the current thread
                if current_thread_id == Some(thread_id) {
                    thread_item.set_class_name("thread-item selected");
                } else {
                    thread_item.set_class_name("thread-item");
                }
                
                // Set data-id attribute for event handling
                thread_item.set_attribute("data-id", &thread_id.to_string())?;
                
                // Add thread information
                let title = document.create_element("div")?;
                title.set_class_name("thread-item-title");
                title.set_text_content(Some(&thread.title));
                
                let timestamp = document.create_element("div")?;
                timestamp.set_class_name("thread-item-time");
                timestamp.set_text_content(Some(
                    &format_timestamp(thread.updated_at.as_ref().unwrap_or(&thread.created_at.clone().unwrap_or_default()))
                ));
                
                // Add edit button for thread title
                let edit_button = document.create_element("div")?;
                edit_button.set_class_name("thread-edit-button");
                edit_button.set_text_content(Some("✎"));
                
                // Add preview of last message if available
                let preview = document.create_element("div")?;
                preview.set_class_name("thread-item-preview");
                
                if let Some(messages) = thread_messages.get(&thread_id) {
                    if !messages.is_empty() {
                        let last_message = messages.last().unwrap();
                        preview.set_text_content(Some(&truncate_text(&last_message.content, 50)));
                    } else {
                        preview.set_text_content(Some("No messages"));
                    }
                } else {
                    preview.set_text_content(Some("No messages"));
                }
                
                // Add elements to thread item
                thread_item.append_child(&title)?;
                thread_item.append_child(&timestamp)?;
                thread_item.append_child(&edit_button)?;
                thread_item.append_child(&preview)?;
                
                // Add click handler for thread selection
                let thread_id_clone = thread_id;
                let click_handler = Closure::wrap(Box::new(move |_: Event| {
                    dispatch_global_message(Message::SelectThread(thread_id_clone));
                }) as Box<dyn FnMut(_)>);
                
                thread_item.add_event_listener_with_callback("click", click_handler.as_ref().unchecked_ref())?;
                click_handler.forget();
                
                // Add edit button click handler
                let thread_id_for_edit = thread_id;
                let title_for_edit = thread.title.clone();
                let edit_handler = Closure::wrap(Box::new(move |e: Event| {
                    e.stop_propagation();
                    
                    // Fix the type mismatch by properly handling the Result before the Option
                    let new_title = web_sys::window()
                        .and_then(|w| {
                            match w.prompt_with_message_and_default("Edit thread title:", &title_for_edit) {
                                Ok(Some(title)) => Some(title),
                                _ => Some(title_for_edit.clone()), // Fall back to original title on error or cancel
                            }
                        })
                        .unwrap_or_else(|| title_for_edit.clone());
                    
                    if !new_title.is_empty() {
                        dispatch_global_message(Message::UpdateThreadTitle(thread_id_for_edit, new_title));
                    }
                }) as Box<dyn FnMut(_)>);
                
                edit_button.add_event_listener_with_callback("click", edit_handler.as_ref().unchecked_ref())?;
                edit_handler.forget();
                
                // Add the thread item to the list
                thread_list.append_child(&thread_item)?;
            }
        }
        
        // Enable or disable chat input based on whether a thread is selected
        if let (Some(input), Some(button)) = (
            document.query_selector(".chat-input").ok().flatten(), 
            document.query_selector(".send-button").ok().flatten()
        ) {
            if current_thread_id.is_none() {
                // No thread selected, disable input
                if let Some(html_input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    html_input.set_disabled(true);
                    html_input.set_placeholder("Create a thread before chatting...");
                }
                if let Some(btn_el) = button.dyn_ref::<web_sys::HtmlElement>() {
                    btn_el.set_attribute("disabled", "true")?;
                    btn_el.set_class_name("send-button disabled");
                }
            } else {
                // Thread is selected, enable input
                if let Some(html_input) = input.dyn_ref::<web_sys::HtmlInputElement>() {
                    html_input.set_disabled(false);
                    html_input.set_placeholder("Type your message...");
                }
                if let Some(btn_el) = button.dyn_ref::<web_sys::HtmlElement>() {
                    btn_el.remove_attribute("disabled")?;
                    btn_el.set_class_name("send-button");
                }
            }
        }
    }
    
    Ok(())
}

// Update the conversation area with messages
pub fn update_conversation_ui(
    document: &Document,
    messages: &[ApiThreadMessage]
) -> Result<(), JsValue> {
    if let Some(messages_container) = document.query_selector(".messages-container").ok().flatten() {
        // Clear existing messages
        messages_container.set_inner_html("");
        
        // Display thread messages, but filter out system messages
        for message in messages.iter().filter(|m| m.role != "system") {
            let message_element = document.create_element("div")?;
            
            // Set class based on message role
            let mut class_name = if message.role == "user" {
                "message user-message".to_string()
            } else {
                "message assistant-message".to_string()
            };
            
            // Check for optimistic messages (they have IDs close to u32::MAX)
            if let Some(id) = &message.id {
                if *id > 4_000_000_000 { // Close to u32::MAX (4,294,967,295)
                    class_name = format!("{} pending", class_name);
                }
            }
            
            message_element.set_class_name(&class_name);
            
            // Create content element
            let content = document.create_element("div")?;
            content.set_class_name("message-content");
            content.set_inner_html(&message.content.replace("\n", "<br>"));
            
            // Create timestamp element
            let timestamp = document.create_element("div")?;
            timestamp.set_class_name("message-time");
            
            let time_text = if let Some(id) = &message.id {
                if *id > 4_000_000_000 { // Close to u32::MAX
                    "Sending...".to_string()
                } else {
                    format_timestamp(&message.created_at.clone().unwrap_or_default())
                }
            } else {
                format_timestamp(&message.created_at.clone().unwrap_or_default())
            };
            
            timestamp.set_text_content(Some(&time_text));
            
            // Add content and timestamp to message element
            message_element.append_child(&content)?;
            message_element.append_child(&timestamp)?;
            
            // Add message to container
            messages_container.append_child(&message_element)?;
        }
        
        // Scroll to the bottom
        messages_container.set_scroll_top(messages_container.scroll_height());
    }
    
    Ok(())
}

// Update the thread title in the header
pub fn update_thread_title(document: &Document) -> Result<(), JsValue> {
    // Instead of accessing APP_STATE directly, dispatch a message to get the current title
    // This prevents borrowing conflicts since state access will happen in update()
    
    // This message will cause the update() function to get the current thread title 
    // and then dispatch an UpdateThreadTitleUI message
    dispatch_global_message(Message::RequestThreadTitleUpdate);
    
    Ok(())
}

// New version that accepts the title directly - no APP_STATE access needed
pub fn update_thread_title_with_data(document: &Document, title: &str) -> Result<(), JsValue> {
    if let Some(title_el) = document.query_selector(".thread-title-text").ok().flatten() {
        title_el.set_text_content(Some(title));
    }
    
    Ok(())
}

// Helper function to get the current agent ID from the state
fn get_current_agent_id(state: &std::cell::Ref<crate::state::AppState>) -> Option<u32> {
    // Get the selected agent ID based on the current thread
    if let Some(thread_id) = state.current_thread_id {
        if let Some(thread) = state.threads.get(&thread_id) {
            return Some(thread.agent_id);
        }
    }
    
    // If no thread is selected, check if we're in chat view with an agent
    if state.active_view == ActiveView::ChatView {
        // We would need some way to store the agent ID when transitioning to chat view
        // For now, return None
    }
    
    None
}

// Helper function to format timestamps
fn format_timestamp(timestamp: &str) -> String {
    // Basic formatting - in a real app you'd want more sophisticated date handling
    if timestamp.is_empty() {
        return "Just now".to_string();
    }
    
    // Parse the timestamp (assuming ISO format)
    // In a real app, use a proper date library
    let date_parts: Vec<&str> = timestamp.split('T').collect();
    if date_parts.len() >= 2 {
        let time_parts: Vec<&str> = date_parts[1].split(':').collect();
        if time_parts.len() >= 2 {
            return format!("{} {}:{}", date_parts[0], time_parts[0], time_parts[1]);
        }
    }
    
    timestamp.to_string()
}

// Helper function to truncate text with ellipsis
fn truncate_text(text: &str, max_length: usize) -> String {
    if text.len() <= max_length {
        text.to_string()
    } else {
        format!("{}...", &text[0..max_length])
    }
}

// Update loading state in the UI
pub fn update_loading_state(document: &Document, is_loading: bool) -> Result<(), JsValue> {
    if let Some(thread_list) = document.query_selector(".thread-list").ok().flatten() {
        if is_loading {
            thread_list.set_inner_html("");
            let loading_el = document.create_element("div")?;
            loading_el.set_class_name("thread-loading");
            loading_el.set_text_content(Some("Loading threads..."));
            thread_list.append_child(&loading_el)?;
        }
    }
    Ok(())
} 