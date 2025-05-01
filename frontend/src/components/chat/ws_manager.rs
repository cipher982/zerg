use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
use web_sys;

use crate::state::dispatch_global_message; // Import dispatch_global_message
use crate::network::topic_manager::{TopicHandler, ITopicManager};
use crate::messages::Message; // UI message enum
use crate::models::ApiThreadMessage;
use crate::network::ws_schema::{WsMessage, WsStreamChunk};

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

        // -----------------------------------------------------------------
        // Strongly-typed handler using WsMessage enum
        // -----------------------------------------------------------------
        let handler = Rc::new(RefCell::new(move |data: serde_json::Value| {
            match serde_json::from_value::<WsMessage>(data.clone()) {
                Ok(WsMessage::ThreadMessage { data: msg }) => {
                    let api_msg: ApiThreadMessage = msg.into();
                    dispatch_global_message(Message::ReceiveNewMessage(api_msg));
                }

                Ok(WsMessage::StreamStart { .. }) => {
                    dispatch_global_message(Message::ReceiveStreamStart(thread_id));
                }

                Ok(WsMessage::StreamChunk { data: chunk }) => {
                    // Convert to global message
                    let WsStreamChunk {
                        thread_id: _tid,
                        chunk_type,
                        content,
                        extra,
                    } = chunk;

                    // Extract optional tool metadata
                    let tool_name = extra
                        .get("tool_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let tool_call_id = extra
                        .get("tool_call_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    let message_id = extra
                        .get("message_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    dispatch_global_message(Message::ReceiveStreamChunk {
                        thread_id,
                        content: content.unwrap_or_default(),
                        chunk_type: Some(chunk_type),
                        tool_name,
                        tool_call_id,
                        message_id,
                    });
                }

                Ok(WsMessage::StreamEnd { .. }) => {
                    dispatch_global_message(Message::ReceiveStreamEnd(thread_id));
                }

                // Thread updates (title change, etc.) – still use old JSON nibble
                Ok(WsMessage::ThreadEvent { data: thread_evt }) => {
                    let maybe_title = thread_evt.extra.get("title").and_then(|v| v.as_str());
                    if let Some(title) = maybe_title {
                        dispatch_global_message(Message::ReceiveThreadUpdate {
                            thread_id: thread_evt.thread_id,
                            title: Some(title.to_string()),
                        });
                    }
                }

                _ => {
                    // Fallback: log once
                    if let Some(t) = data.get("type").and_then(|v| v.as_str()) {
                        web_sys::console::warn_1(&format!("ChatViewWsManager: unhandled WS message type {}", t).into());
                    }
                }
            }
        }));

        // Subscribe to thread-specific events
        let topic = format!("thread:{}", thread_id);
        self.thread_subscription_handler = Some(handler.clone());
        topic_manager.subscribe(topic.clone(), handler)?;
        web_sys::console::log_1(&format!("ChatViewWsManager: Subscribed to topic '{}' for thread {}", topic, thread_id).into());
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