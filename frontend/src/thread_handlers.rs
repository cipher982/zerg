use crate::models::{ApiThread, ApiThreadMessage};
use crate::messages::Message;
use crate::state::dispatch_global_message;
use std::collections::HashMap;
use wasm_bindgen_futures::spawn_local;
use rand;

/// Message status for optimistic UI updates
#[derive(Debug, Clone, PartialEq)]
pub enum MessageStatus {
    /// Message is pending confirmation from the server
    Pending,
    /// Message has been confirmed by the server
    Confirmed,
    /// Message failed to send
    Failed,
}

/// Handles when a user sends a new message to a thread
/// Implements the optimistic UI pattern:
/// 1. Immediately display the message in the UI with pending status
/// 2. Send the message to the backend
/// 3. Update the message with the confirmed status when the server responds
pub fn handle_send_thread_message(
    thread_id: u32,
    content: String,
    thread_messages: &mut HashMap<u32, Vec<ApiThreadMessage>>,
    threads: &HashMap<u32, ApiThread>,
    current_thread_id: Option<u32>,
) -> Box<dyn FnOnce()> {
    // Create an optimistic message with a client-generated ID for tracking
    // Use negative numbers to indicate optimistic messages
    let now = chrono::Utc::now().to_rfc3339();
    let client_id: u32 = u32::MAX - rand::random::<u32>() % 1000; // Generate a random large negative-like number
    
    let user_message = ApiThreadMessage {
        id: Some(client_id), // Use the numeric ID to track this message
        thread_id,
        role: "user".to_string(),
        content: content.clone(),
        created_at: Some(now),
    };
    
    // Prepare data for UI updates
    let mut conversation_messages = Vec::new();
    let current_thread_id_opt = current_thread_id;
    let mut thread_messages_map = thread_messages.clone();
    let threads_data: Vec<ApiThread> = threads.values().cloned().collect();
    
    // Add optimistic message to state
    if let Some(messages) = thread_messages.get_mut(&thread_id) {
        messages.push(user_message.clone());
        conversation_messages = messages.clone();
    } else {
        thread_messages.insert(thread_id, vec![user_message.clone()]);
        conversation_messages = vec![user_message.clone()];
    }
    
    // Clone data for async operations
    let content_clone = content.clone();
    
    // Return a closure to be executed after the borrow is released
    Box::new(move || {
        // Update the UI with the optimistic message
        if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
            dispatch_global_message(Message::UpdateConversation(conversation_messages));
            dispatch_global_message(Message::UpdateThreadList(
                threads_data,
                current_thread_id_opt,
                thread_messages_map
            ));
        }
        
        // Send the message to the backend
        spawn_local(async move {
            match crate::network::api_client::ApiClient::create_thread_message(thread_id, &content_clone).await {
                Ok(response) => {
                    // Dispatch a message with the sent message and the client ID for tracking
                    dispatch_global_message(Message::ThreadMessageSent(response, client_id.to_string()));
                },
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to send thread message: {:?}", e).into());
                    // Could dispatch a message to mark the message as failed
                    dispatch_global_message(Message::ThreadMessageFailed(thread_id, client_id.to_string()));
                }
            }
        });
    })
}

/// Handles the response from the server after sending a message
/// Updates the optimistic message with the confirmed one from the server
pub fn handle_thread_message_sent(
    response: String,
    client_id: String,
    thread_messages: &mut HashMap<u32, Vec<ApiThreadMessage>>,
) -> Option<Box<dyn FnOnce()>> {
    // Parse the response from the server
    if let Ok(thread_message) = serde_json::from_str::<ApiThreadMessage>(&response) {
        let thread_id = thread_message.thread_id;
        
        // Try to parse the client_id string back to u32
        if let Ok(client_id_num) = client_id.parse::<u32>() {
            // Find and replace the optimistic message with the confirmed message
            if let Some(messages) = thread_messages.get_mut(&thread_id) {
                // Find the index of the optimistic message
                if let Some(index) = messages.iter().position(|msg| 
                    msg.id.as_ref().map_or(false, |id| *id == client_id_num)
                ) {
                    // Replace the optimistic message with the confirmed message
                    messages[index] = thread_message.clone();
                    
                    // Return a closure to update the UI
                    let messages_clone = messages.clone();
                    return Some(Box::new(move || {
                        if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                            // Update the conversation UI
                            dispatch_global_message(Message::UpdateConversation(messages_clone));
                        }
                    }));
                }
            }
        }
    }
    
    None
}

/// Handles marking a message as failed
pub fn handle_thread_message_failed(
    thread_id: u32,
    client_id: String,
    thread_messages: &mut HashMap<u32, Vec<ApiThreadMessage>>,
) -> Option<Box<dyn FnOnce()>> {
    // Try to parse the client_id string back to u32
    if let Ok(client_id_num) = client_id.parse::<u32>() {
        // Find and update the status of the optimistic message
        if let Some(messages) = thread_messages.get_mut(&thread_id) {
            // Find the optimistic message by its client ID
            if let Some(message) = messages.iter_mut().find(|msg| 
                msg.id.as_ref().map_or(false, |id| *id == client_id_num)
            ) {
                // Mark as failed by adding a special tag to the content
                message.content = format!("[Failed to send] {}", message.content);
                
                // Return a closure to update the UI
                let messages_clone = messages.clone();
                return Some(Box::new(move || {
                    if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                        // Update the conversation UI with the failed message status
                        dispatch_global_message(Message::UpdateConversation(messages_clone));
                    }
                }));
            }
        }
    }
    
    None
}

/// Handles when a message is received from the websocket
pub fn handle_thread_message_received(
    message_str: String,
    thread_messages: &mut HashMap<u32, Vec<ApiThreadMessage>>,
    threads: &HashMap<u32, ApiThread>,
    current_thread_id: Option<u32>,
) -> Option<Box<dyn FnOnce()>> {
    // Parse the received message
    if let Ok(message_value) = serde_json::from_str::<serde_json::Value>(&message_str) {
        if let Ok(message) = serde_json::from_value::<ApiThreadMessage>(message_value) {
            let thread_id = message.thread_id;
            
            // Variables to collect data for UI updates
            let mut conversation_messages = Vec::new();
            let threads_data: Vec<ApiThread> = threads.values().cloned().collect();
            let current_thread_id_opt = current_thread_id;
            let mut thread_messages_map = thread_messages.clone();
            
            // Add the message to the thread
            if let Some(messages) = thread_messages.get_mut(&thread_id) {
                messages.push(message.clone());
                conversation_messages = messages.clone();
            } else {
                thread_messages.insert(thread_id, vec![message.clone()]);
                conversation_messages = vec![message.clone()];
                
                // Update our thread_messages_map clone with the new message
                thread_messages_map.insert(thread_id, vec![message.clone()]);
            }
            
            // Return a closure to update the UI
            return Some(Box::new(move || {
                // Update the UI using message dispatch
                if let Some(_document) = web_sys::window().and_then(|w| w.document()) {
                    dispatch_global_message(Message::UpdateConversation(conversation_messages));
                    dispatch_global_message(Message::UpdateThreadList(
                        threads_data,
                        current_thread_id_opt,
                        thread_messages_map
                    ));
                }
            }));
        }
    }
    None
}
