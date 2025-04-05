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
use crate::models::{ApiThreadMessage, ApiThread}; // Added ApiThread here
use crate::network::TopicManager;
use crate::network::ws_client_v2::IWsClient;

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
    pub(crate) thread_subscription_handler: Option<TopicHandler>,
    current_thread_id: Option<u32>,
}

impl ChatViewWsManager {
    /// Create a new ChatViewWsManager instance.
    pub fn new() -> Self {
        Self {
            thread_subscription_handler: None,
            current_thread_id: None,
        }
    }

    /// Initialize subscriptions for the chat view with a specific thread ID.
    pub fn initialize(
        &mut self,
        thread_id: u32,
        topic_manager: Rc<RefCell<dyn ITopicManager>>
    ) -> Result<(), JsValue> {
        // Store the thread ID we're subscribing to
        self.current_thread_id = Some(thread_id);
        
        // Use the passed topic manager instead of accessing APP_STATE
        self.subscribe_to_thread_events(topic_manager, thread_id)?;
        Ok(())
    }

    /// Subscribe to thread-related events using the provided ITopicManager.
    pub(crate) fn subscribe_to_thread_events(
        &mut self,
        topic_manager_rc: Rc<RefCell<dyn ITopicManager>>,
        thread_id: u32
    ) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();

        // Create a handler for thread events
        let handler = Rc::new(RefCell::new(move |data: serde_json::Value| {
            web_sys::console::log_1(&format!("Chat handler received message: {:?}", data).into()); // Renamed log for clarity

            if let Some(event_type) = data.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    "thread_history" => {
                        // Parse the messages array from the history message
                        if let Some(messages_val) = data.get("messages") {
                            if let Ok(messages) = serde_json::from_value::<Vec<ApiThreadMessage>>(messages_val.clone()) {
                                dispatch_global_message(Message::ReceiveThreadHistory(messages));
                            } else {
                                web_sys::console::error_1(&"Failed to parse messages array from thread_history".into());
                            }
                        } else {
                            web_sys::console::error_1(&"thread_history message missing 'messages' field".into());
                        }
                    },
                    "thread_message_created" => {
                        // Parse the message data and dispatch to update UI
                        // Assuming data field for this event contains ApiThreadMessage
                        if let Some(message_data_val) = data.get("data") { // Look inside "data" for this event type
                            if let Ok(message_data) = serde_json::from_value::<ApiThreadMessage>(message_data_val.clone()) {
                                dispatch_global_message(Message::ReceiveNewMessage(message_data));
                            } else {
                                web_sys::console::error_1(&"Failed to parse message_data from thread_message_created".into());
                            }
                        } else {
                             web_sys::console::error_1(&"thread_message_created event missing 'data' field".into());
                        }
                    },
                    "thread_updated" => {
                        // Handle thread updates (title changes, etc.)
                         if let Some(thread_data_val) = data.get("data") { // Look inside "data" for this event type
                            if let Ok(thread_data) = serde_json::from_value::<ApiThread>(thread_data_val.clone()) {
                                dispatch_global_message(Message::ReceiveThreadUpdate {
                                    thread_id: thread_data.id.unwrap_or(0), 
                                    title: Some(thread_data.title),
                                });
                            } else {
                                web_sys::console::error_1(&"Failed to parse thread_data from thread_updated".into());
                            }
                         } else {
                              web_sys::console::error_1(&"thread_updated event missing 'data' field".into());
                         }
                    },
                    _ => {
                        web_sys::console::warn_1(&format!("Chat handler: Unhandled message type: {}", event_type).into());
                    }
                }
            }
        }));

        // Subscribe to thread-specific events
        let topic = format!("thread:{}", thread_id);
        self.thread_subscription_handler = Some(handler.clone());
        topic_manager.subscribe(topic, handler)?;
        Ok(())
    }

    /// Clean up WebSocket subscriptions for the chat view.
    pub fn cleanup(&mut self, topic_manager_rc: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();
        
        if let Some(handler) = self.thread_subscription_handler.take() {
            if let Some(thread_id) = self.current_thread_id {
                web_sys::console::log_1(&format!("ChatViewWsManager: Cleaning up thread subscription handler for thread {}", thread_id).into());
                let topic = format!("thread:{}", thread_id);
                topic_manager.unsubscribe_handler(&topic, &handler)?;
            }
        } else {
            web_sys::console::warn_1(&"ChatViewWsManager cleanup: No handler found to unsubscribe.".into());
        }
        
        self.current_thread_id = None;
        Ok(())
    }
}

// Create a singleton instance of the ChatViewWsManager
thread_local! {
    pub static CHAT_VIEW_WS: RefCell<Option<ChatViewWsManager>> = RefCell::new(None);
}

/// Initialize the chat view WebSocket manager singleton
pub fn init_chat_view_ws(thread_id: u32, topic_manager: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
    CHAT_VIEW_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if manager_opt.is_none() {
            web_sys::console::log_1(&format!("Initializing ChatViewWsManager singleton for thread {}...", thread_id).into());
            let mut manager = ChatViewWsManager::new();
            manager.initialize(thread_id, topic_manager)?;
            *manager_opt = Some(manager);
        } else {
            // If manager exists but thread changed, reinitialize
            if let Some(manager) = manager_opt.as_mut() {
                if manager.current_thread_id != Some(thread_id) {
                    web_sys::console::log_1(&format!("Reinitializing ChatViewWsManager for new thread {}...", thread_id).into());
                    // Clean up existing subscriptions
                    manager.cleanup(topic_manager.clone())?;
                    // Initialize for new thread
                    manager.initialize(thread_id, topic_manager)?;
                }
            }
        }
        Ok(())
    })
}

/// Cleanup the chat view WebSocket manager singleton subscriptions
pub fn cleanup_chat_view_ws(topic_manager: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
    CHAT_VIEW_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if let Some(manager) = manager_opt.as_mut() {
            web_sys::console::log_1(&"Cleaning up ChatViewWsManager singleton...".into());
            manager.cleanup(topic_manager)?;
        }
        Ok(())
    })
} 