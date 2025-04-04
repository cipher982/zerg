use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
use web_sys;

use crate::state::{APP_STATE, dispatch_global_message}; // Import dispatch_global_message
use crate::network::topic_manager::{TopicHandler, ITopicManager}; // Import Trait
use crate::network::messages::{ // Import relevant message structs
    ThreadHistoryMessage, 
    ThreadMessageData, // Use the specific type name
    ThreadUpdatedEventData,
    StreamStartMessage,
    StreamChunkMessage,
    StreamEndMessage,
};
use crate::messages::Message; // Import main Message enum
use crate::models::ApiThreadMessage; // Import the target model type

/// Helper to convert network message data to the application model
fn convert_network_message_to_model(network_msg: ThreadMessageData) -> ApiThreadMessage {
    ApiThreadMessage {
        id: network_msg.id.map(|i| i as u32), // Convert Option<i32> to Option<u32>
        thread_id: network_msg.thread_id as u32, // Convert i32 to u32
        role: network_msg.role,
        content: network_msg.content,
        created_at: network_msg.timestamp, // Use timestamp as created_at
    }
}

/// Manages WebSocket subscriptions and message handling for the Chat View lifecycle.
pub struct ChatViewWsManager {
    // Store the handler Rc to allow unsubscribing later.
    thread_subscription_handler: Option<TopicHandler>,
    // Store the topic we are subscribed to for cleanup
    subscribed_topic: Option<String>,
}

impl ChatViewWsManager {
    /// Create a new ChatViewWsManager instance.
    pub fn new() -> Self {
        Self {
            thread_subscription_handler: None,
            subscribed_topic: None,
        }
    }

    /// Initialize subscriptions for the chat view for a specific thread.
    /// Assumes global WebSocket is already connected or will connect.
    pub fn initialize(&mut self, thread_id: u64) -> Result<(), JsValue> {
        // Get the global TopicManager from AppState
        let topic_manager_rc = APP_STATE.with(|state_ref| {
            state_ref.borrow().topic_manager.clone() as Rc<RefCell<dyn ITopicManager>>
        });
        self.subscribe_to_thread_events(topic_manager_rc, thread_id)?;
        Ok(())
    }

    /// Subscribe to thread-specific events using the provided ITopicManager.
    pub(crate) fn subscribe_to_thread_events(
        &mut self,
        topic_manager_rc: Rc<RefCell<dyn ITopicManager>>,
        thread_id: u64
    ) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();
        let topic = format!("thread:{}", thread_id);

        // --- Define Handler ---
        let handler_topic = topic.clone(); // Clone topic for use inside the handler closure
        let handler = Rc::new(RefCell::new(move |data: serde_json::Value| {
            web_sys::console::log_1(&format!("ChatView handler received event for topic {}: {:?}", handler_topic, data).into());
            
            // Attempt to deserialize based on "type" field
            if let Some(event_type) = data.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    "thread_history" => {
                        match serde_json::from_value::<ThreadHistoryMessage>(data) {
                            Ok(history_msg) => {
                                web_sys::console::log_1(&format!("Parsed ThreadHistoryMessage for thread {}", history_msg.thread_id).into());
                                // Convert Vec<ThreadMessageData> to Vec<ApiThreadMessage>
                                let model_messages: Vec<ApiThreadMessage> = history_msg.messages.into_iter()
                                    .map(convert_network_message_to_model)
                                    .collect();
                                dispatch_global_message(Message::UpdateConversation(model_messages));
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse ThreadHistoryMessage: {}", e).into()),
                        }
                    },
                    "thread_message_created" => {
                         // Note: The backend sends the data payload nested under "data" for this event type.
                         match serde_json::from_value::<ThreadMessageData>(data.get("data").cloned().unwrap_or_default()) {
                            Ok(network_msg_data) => {
                                web_sys::console::log_1(&format!("Parsed ThreadMessageData for thread {}", network_msg_data.thread_id).into());
                                // Convert before dispatching
                                let model_message = convert_network_message_to_model(network_msg_data);
                                dispatch_global_message(Message::ReceiveNewMessage(model_message)); 
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse ThreadMessageData from event: {}", e).into()),
                        }
                    },
                    "thread_updated" => {
                         // Note: The backend sends the data payload nested under "data" for this event type.
                         match serde_json::from_value::<ThreadUpdatedEventData>(data.get("data").cloned().unwrap_or_default()) {
                            Ok(update_data) => {
                                web_sys::console::log_1(&format!("Parsed ThreadUpdatedEventData for thread {}", update_data.thread_id).into());
                                // Dispatch using the new variant
                                dispatch_global_message(Message::ReceiveThreadUpdate {
                                     thread_id: update_data.thread_id as u32,
                                     title: update_data.title,
                                });
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse ThreadUpdatedEventData: {}", e).into()),
                        }
                    },
                    "stream_start" => {
                         match serde_json::from_value::<StreamStartMessage>(data) {
                            Ok(start_msg) => {
                                web_sys::console::log_1(&format!("Parsed StreamStartMessage for thread {}", start_msg.thread_id).into());
                                dispatch_global_message(Message::ReceiveStreamStart(start_msg.thread_id as u32));
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse StreamStartMessage: {}", e).into()),
                        }
                    },
                     "stream_chunk" => {
                         match serde_json::from_value::<StreamChunkMessage>(data) {
                            Ok(chunk_msg) => {
                                dispatch_global_message(Message::ReceiveStreamChunk {
                                    thread_id: chunk_msg.thread_id as u32,
                                    content: chunk_msg.content,
                                });
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse StreamChunkMessage: {}", e).into()),
                        }
                    },
                     "stream_end" => {
                         match serde_json::from_value::<StreamEndMessage>(data) {
                            Ok(end_msg) => {
                                web_sys::console::log_1(&format!("Parsed StreamEndMessage for thread {}", end_msg.thread_id).into());
                                dispatch_global_message(Message::ReceiveStreamEnd(end_msg.thread_id as u32));
                            },
                            Err(e) => web_sys::console::error_1(&format!("Failed to parse StreamEndMessage: {}", e).into()),
                        }
                    },
                    _ => {
                         web_sys::console::warn_1(&format!("ChatView handler: Unhandled event type: {}", event_type).into());
                    }
                }
            } else {
                 web_sys::console::warn_1(&format!("ChatView handler: Received message without 'type' field: {:?}", data).into());
            }
        }));

        self.thread_subscription_handler = Some(handler.clone());
        self.subscribed_topic = Some(topic.clone());
        
        topic_manager.subscribe(topic, handler)?;
        Ok(())
    }

    /// Clean up WebSocket subscriptions for the chat view.
    pub fn cleanup(&mut self) -> Result<(), JsValue> {
        // Get the global TopicManager from AppState
        let topic_manager_rc = APP_STATE.with(|state_ref| {
             state_ref.borrow().topic_manager.clone() as Rc<RefCell<dyn ITopicManager>>
        });
        let mut topic_manager = topic_manager_rc.borrow_mut();
        
        if let (Some(handler), Some(topic)) = (self.thread_subscription_handler.take(), self.subscribed_topic.take()) {
             web_sys::console::log_1(&format!("ChatViewWsManager: Cleaning up subscription handler for topic {}", topic).into());
             topic_manager.unsubscribe_handler(&topic, &handler)?;
        } else {
            web_sys::console::warn_1(&"ChatViewWsManager cleanup: No handler or topic found to unsubscribe.".into());
        }
        Ok(())
    }
}

// Create a singleton instance of the ChatViewWsManager
thread_local! {
    pub static CHAT_VIEW_WS: RefCell<Option<ChatViewWsManager>> = RefCell::new(None);
}

/// Initialize the chat view WebSocket manager singleton for a specific thread
pub fn init_chat_view_ws(thread_id: u64) -> Result<(), JsValue> {
    CHAT_VIEW_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if manager_opt.is_none() {
            web_sys::console::log_1(&format!("Initializing ChatViewWsManager singleton for thread {}...", thread_id).into());
            let mut manager = ChatViewWsManager::new();
            manager.initialize(thread_id)?;
            *manager_opt = Some(manager);
        } else {
            // If already initialized, maybe re-initialize if thread_id changed?
            // Or assume component logic handles ensuring cleanup before new init.
            // For now, just warn if initializing again without cleanup.
            web_sys::console::warn_1(&"ChatViewWsManager singleton already initialized. Call cleanup first if changing threads.".into());
            // Optional: Re-initialize if needed
            // Need to handle potential errors from cleanup/initialize if re-initializing
            // let mut current_manager = manager_opt.as_mut().unwrap();
            // let current_topic = current_manager.subscribed_topic.clone();
            // let new_topic = format!("thread:{}", thread_id);
            // if current_topic.as_deref() != Some(&new_topic) {
            //     web_sys::console::log_1(&format!("Re-initializing ChatViewWsManager for new thread {}...", thread_id).into());
            //     current_manager.cleanup()?;
            //     current_manager.initialize(thread_id)?;
            // }
        }
        Ok(())
    })
}

/// Cleanup the chat view WebSocket manager singleton subscriptions
pub fn cleanup_chat_view_ws() -> Result<(), JsValue> {
    CHAT_VIEW_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if let Some(manager) = manager_opt.as_mut() {
            web_sys::console::log_1(&"Cleaning up ChatViewWsManager singleton...".into());
            manager.cleanup()?;
        }
        // Remove the manager instance itself after cleanup
        *manager_opt = None; 
        Ok(())
    })
} 