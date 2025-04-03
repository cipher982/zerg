use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use serde::{Serialize};
use wasm_bindgen::JsValue;
use uuid; // Ensure uuid crate is added to Cargo.toml
use serde_json::Value;

use super::ws_client_v2::{WsClientV2, IWsClient}; // Update import
use super::messages::{SubscribeMessage, UnsubscribeMessage};
use super::messages::builders::{create_subscribe, create_unsubscribe};

/// Represents a topic string like "agent:123" or "thread:45"
pub type Topic = String;

/// Message handler type for topic-specific messages
/// Receives the "data" part of the incoming WebSocket message
pub type TopicHandler = Rc<RefCell<dyn FnMut(serde_json::Value)>>;

/// Manages topic subscriptions and message routing for WsClientV2
pub struct TopicManager {
    /// Map of topics to their handlers
    topic_handlers: HashMap<Topic, Vec<TopicHandler>>,
    /// Set of currently subscribed topics (sent to backend)
    subscribed_topics: HashSet<Topic>,
    /// Reference to the WebSocket client for sending messages
    ws_client: Rc<RefCell<dyn IWsClient>>,
}

// --- Implementation ---

impl TopicManager {
    /// Creates a new TopicManager linked to a WsClientV2 instance.
    pub fn new(ws_client: Rc<RefCell<dyn IWsClient>>) -> Self {
        Self {
            topic_handlers: HashMap::new(),
            subscribed_topics: HashSet::new(),
            ws_client,
        }
    }

    /// Subscribe to a topic and register a handler to be called for messages on that topic.
    ///
    /// If this is the first handler for the topic, a "subscribe" message is sent to the backend.
    pub fn subscribe(&mut self, topic: Topic, handler: TopicHandler) -> Result<(), JsValue> {
        let is_new_topic_subscription = !self.subscribed_topics.contains(&topic);

        // Add the handler to the list for this topic
        self.topic_handlers
            .entry(topic.clone())
            .or_default()
            .push(handler);

        // If it's the first time we're interested in this topic, subscribe via WebSocket
        if is_new_topic_subscription {
            self.subscribed_topics.insert(topic.clone());
            web_sys::console::log_1(&format!("Sending subscribe request for topic: {}", topic).into());

            let msg = create_subscribe(vec![topic]);
            let msg_json = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

            match self.ws_client.try_borrow() {
                Ok(client) => client.send_serialized_message(&msg_json)?,
                Err(_) => return Err(JsValue::from_str("Failed to borrow WsClient for subscribe")),
            }
        } else {
            web_sys::console::log_1(&format!("Adding additional handler for already subscribed topic: {}", topic).into());
        }

        Ok(())
    }

    /// Unsubscribe from a topic, removing all handlers associated with it.
    ///
    /// If handlers existed for the topic, an "unsubscribe" message is sent to the backend.
    /// Note: This removes *all* handlers for the given topic. Fine-grained handler removal
    /// would require handler IDs.
    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<(), JsValue> {
        // Remove handlers only if the topic exists in our handler map
        if self.topic_handlers.remove(topic).is_some() {
             // Only unsubscribe from the backend if we were actually subscribed
            if self.subscribed_topics.remove(topic) {
                web_sys::console::log_1(&format!("Sending unsubscribe request for topic: {}", topic).into());

                let msg = create_unsubscribe(vec![topic.clone()]);
                let msg_json = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

                match self.ws_client.try_borrow() {
                     Ok(client) => client.send_serialized_message(&msg_json)?,
                     Err(_) => return Err(JsValue::from_str("Failed to borrow WsClient for unsubscribe")),
                }
            } else {
                 web_sys::console::log_1(&format!("Removed local handlers for topic {} but was not subscribed on backend.", topic).into());
            }
        } else {
            web_sys::console::log_1(&format!("Attempted to unsubscribe from topic {} with no handlers.", topic).into());
        }

        Ok(())
    }

    /// Resubscribes to all topics currently tracked in `subscribed_topics`.
    /// Typically called after a successful reconnection.
    pub fn resubscribe_all_topics(&self) -> Result<(), JsValue> {
        web_sys::console::log_1(&"Resubscribing to all topics after connection...".into());
        let topics_to_resubscribe: Vec<Topic> = self.subscribed_topics.iter().cloned().collect();

        if !topics_to_resubscribe.is_empty() {
            web_sys::console::log_1(&format!("Sending resubscribe request for topics: {:?}", topics_to_resubscribe).into());
            let msg = create_subscribe(topics_to_resubscribe);
            let msg_json = serde_json::to_string(&msg).map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

            match self.ws_client.try_borrow() {
                Ok(client) => client.send_serialized_message(&msg_json)?,
                Err(_) => return Err(JsValue::from_str("Failed to borrow WsClient for resubscribe")),
            }
        } else {
            web_sys::console::log_1(&"No topics to resubscribe.".into());
        }

        Ok(())
    }

    /// Routes an incoming message (parsed JSON) to the appropriate topic handlers.
    /// Determines the topic based on message content (e.g., `type` and `data` fields).
    pub fn route_incoming_message(&self, message: serde_json::Value) {
        web_sys::console::log_1(&format!("TopicManager received message: {:?}", message).into());

        // --- Determine the topic from the message --- 
        // This logic needs to align with how the backend structures event messages.
        // Assuming backend sends: {"type": "event_type", "data": {"id": resource_id, ...}}
        // Or for thread messages: {"type": "event_type", "data": {"thread_id": resource_id, ...}}

        let message_type = message.get("type").and_then(|t| t.as_str());
        let data = message.get("data");

        let topic_str_option: Option<String> = match (message_type, data) {
            (Some(mt), Some(d)) if mt.starts_with("agent_") => {
                d.get("id").and_then(|id| id.as_u64()).map(|id| format!("agent:{}", id))
            }
            (Some(mt), Some(d)) if mt.starts_with("thread_") => {
                d.get("thread_id").and_then(|id| id.as_u64()).map(|id| format!("thread:{}", id))
            }
             // Handle specific message types that might not follow the event_type/data pattern, if any
             // e.g., maybe a direct thread history message?
             (Some("thread_history"), Some(d)) => { // Check schema ws_messages.py ThreadHistoryMessage
                 d.get("thread_id").and_then(|id| id.as_u64()).map(|id| format!("thread:{}", id))
             }
              (Some("agent_state"), Some(d)) => { // Check schema new_handlers.py agent_state
                 d.get("id").and_then(|id| id.as_u64()).map(|id| format!("agent:{}", id))
             }
            _ => {
                web_sys::console::warn_1(&format!("Could not determine topic for message type: {:?}", message_type).into());
                None
            }
        };

        // --- Call Handlers --- 
        if let Some(topic_str) = topic_str_option {
             web_sys::console::log_1(&format!("Routing message to handlers for topic: {}", topic_str).into());
            if let Some(handlers) = self.topic_handlers.get(&topic_str) {
                // Extract the "data" part to pass to handlers, or the whole message if no "data" field
                 let payload_to_handlers = data.cloned().unwrap_or(message);

                for handler_rc in handlers {
                     if let Ok(mut handler) = handler_rc.try_borrow_mut() {
                         // Call the handler with the relevant payload
                        (*handler)(payload_to_handlers.clone()); 
                    } else {
                        web_sys::console::error_1(&format!("Failed to borrow handler for topic {}", topic_str).into());
                    }
                }
            } else {
                web_sys::console::log_1(&format!("No handlers registered for determined topic: {}", topic_str).into());
            }
        } else {
             // Handle messages that don't map to a specific topic? (e.g., Pong, Errors from WS itself)
             if let Some("pong") = message_type {
                 // Pong received, maybe update last pong time if needed
             } else if let Some("error") = message_type {
                 // Handle potential error messages from the backend WebSocket layer
                 web_sys::console::error_1(&format!("Received WebSocket error message: {:?}", message).into());
             }
        }
    }

    #[cfg(test)]
    pub fn has_subscription(&self, topic: &str) -> bool {
        self.subscribed_topics.contains(topic)
    }
} 