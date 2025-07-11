use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys;

use crate::messages::Message; // UI message enum
use crate::models::ApiThreadMessage;
use crate::network::topic_manager::{ITopicManager, TopicHandler};
use crate::generated::ws_messages::{ThreadMessageData, StreamChunkData, AssistantIdData};
use crate::state::dispatch_global_message; // Import dispatch_global_message

// Conversion from generated ThreadMessageData to ApiThreadMessage
impl From<ThreadMessageData> for ApiThreadMessage {
    fn from(data: ThreadMessageData) -> Self {
        // Extract fields from the message dictionary
        let message = &data.message;
        ApiThreadMessage {
            id: message.get("id").and_then(|v| v.as_u64()).map(|v| v as u32),
            thread_id: data.thread_id,
            role: message.get("role").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            content: message.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            timestamp: message.get("timestamp").and_then(|v| v.as_str()).map(|s| s.to_string()),
            message_type: message.get("message_type").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tool_name: message.get("tool_name").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tool_call_id: message.get("tool_call_id").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tool_input: message.get("tool_input").and_then(|v| v.as_str()).map(|s| s.to_string()),
            parent_id: message.get("parent_id").and_then(|v| v.as_u64()).map(|v| v as u32),
        }
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
        topic_manager: Rc<RefCell<dyn ITopicManager>>,
    ) -> Result<(), JsValue> {
        // Store the thread ID we're subscribing to
        self.current_thread_id = Some(thread_id);

        // Use the passed topic manager instead of accessing APP_STATE
        self.subscribe_to_thread_events(topic_manager, thread_id)?;
        Ok(())
    }

    /// Internal helper to convert a structured `StreamChunkData` into the
    /// `Message::ReceiveStreamChunk` UI event.
    fn forward_stream_chunk(thread_id: u32, chunk: StreamChunkData) {
        let StreamChunkData {
            thread_id: _tid,
            chunk_type,
            content,
            tool_name,
            tool_call_id,
        } = chunk;

        dispatch_global_message(Message::ReceiveStreamChunk {
            thread_id,
            content: content.unwrap_or_default(),
            chunk_type: Some(chunk_type),
            tool_name,
            tool_call_id,
            message_id: None, // Not available in generated StreamChunkData
        });
    }

    /// Subscribe to thread-related events using the provided ITopicManager.
    pub(crate) fn subscribe_to_thread_events(
        &mut self,
        topic_manager_rc: Rc<RefCell<dyn ITopicManager>>,
        thread_id: u32,
    ) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();

        // -----------------------------------------------------------------
        // Strongly-typed handler using generated envelope format
        // -----------------------------------------------------------------
        let handler = Rc::new(RefCell::new(move |data: serde_json::Value| {
            web_sys::console::log_1(
                &format!("🔍 [CHAT WS] Received message for thread {}: {:?}", thread_id, data).into(),
            );
            
            // Extract message type and data from envelope-style payload
            let message_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown");
            let message_data = data.get("data").unwrap_or(&data);
            
            web_sys::console::log_1(
                &format!("🔍 [CHAT WS] Processing message type: {}", message_type).into(),
            );
            
            match message_type {
                "thread_message" => {
                    match serde_json::from_value::<ThreadMessageData>(message_data.clone()) {
                        Ok(msg) => {
                            let api_msg: ApiThreadMessage = msg.into();
                            dispatch_global_message(Message::ReceiveNewMessage(api_msg));
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse thread_message: {}", e).into(),
                            );
                        }
                    }
                    return;
                }
                "stream_start" => {
                    dispatch_global_message(Message::ReceiveStreamStart(thread_id));
                    return;
                }
                "stream_chunk" => {
                    match serde_json::from_value::<StreamChunkData>(message_data.clone()) {
                        Ok(chunk) => {
                            Self::forward_stream_chunk(thread_id, chunk);
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse stream_chunk: {}", e).into(),
                            );
                        }
                    }
                    return;
                }
                "stream_end" => {
                    dispatch_global_message(Message::ReceiveStreamEnd(thread_id));
                    return;
                }
                "assistant_id" => {
                    match serde_json::from_value::<AssistantIdData>(message_data.clone()) {
                        Ok(data) => {
                            dispatch_global_message(Message::ReceiveAssistantId {
                                thread_id: data.thread_id,
                                message_id: data.message_id,
                            });
                        }
                        Err(e) => {
                            web_sys::console::error_1(
                                &format!("Failed to parse assistant_id: {}", e).into(),
                            );
                        }
                    }
                    return;
                }
                _ => {
                    // Unhandled message type
                    web_sys::console::warn_1(
                        &format!("ChatViewWsManager: unhandled message type: {}", message_type).into(),
                    );
                }
            }
        }));

        // Subscribe to thread-specific events
        let topic = format!("thread:{}", thread_id);
        self.thread_subscription_handler = Some(handler.clone());
        topic_manager.subscribe(topic.clone(), handler)?;
        web_sys::console::log_1(
            &format!(
                "ChatViewWsManager: Subscribed to topic '{}' for thread {}",
                topic, thread_id
            )
            .into(),
        );
        Ok(())
    }

    /// Clean up WebSocket subscriptions for the chat view.
    pub fn cleanup(
        &mut self,
        topic_manager_rc: Rc<RefCell<dyn ITopicManager>>,
    ) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();

        if let Some(handler) = self.thread_subscription_handler.take() {
            if let Some(thread_id) = self.current_thread_id {
                web_sys::console::log_1(
                    &format!(
                        "ChatViewWsManager: Cleaning up thread subscription handler for thread {}",
                        thread_id
                    )
                    .into(),
                );
                let topic = format!("thread:{}", thread_id);
                topic_manager.unsubscribe_handler(&topic, &handler)?;
            }
        } else {
            web_sys::console::warn_1(
                &"ChatViewWsManager cleanup: No handler found to unsubscribe.".into(),
            );
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
pub fn init_chat_view_ws(
    thread_id: u32,
    topic_manager: Rc<RefCell<dyn ITopicManager>>,
) -> Result<(), JsValue> {
    CHAT_VIEW_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if manager_opt.is_none() {
            web_sys::console::log_1(
                &format!(
                    "Initializing ChatViewWsManager singleton for thread {}...",
                    thread_id
                )
                .into(),
            );
            let mut manager = ChatViewWsManager::new();
            manager.initialize(thread_id, topic_manager)?;
            *manager_opt = Some(manager);
        } else {
            // If manager exists but thread changed, reinitialize
            if let Some(manager) = manager_opt.as_mut() {
                if manager.current_thread_id != Some(thread_id) {
                    web_sys::console::log_1(
                        &format!(
                            "Reinitializing ChatViewWsManager for new thread {}...",
                            thread_id
                        )
                        .into(),
                    );
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
#[allow(dead_code)]
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
