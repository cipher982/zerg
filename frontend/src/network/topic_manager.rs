use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::JsValue;
// Ensure uuid crate is added to Cargo.toml

use super::messages::builders::{create_subscribe, create_unsubscribe};
use super::ws_client_v2::IWsClient; // Update import

/// Represents a topic string like "agent:123" or "thread:45"
pub type Topic = String;

/// Message handler type for topic-specific messages
/// Receives the "data" part of the incoming WebSocket message
pub type TopicHandler = Rc<RefCell<dyn FnMut(serde_json::Value)>>;

// --- Define the Trait ---
pub trait ITopicManager {
    fn subscribe(&mut self, topic: Topic, handler: TopicHandler) -> Result<(), JsValue>;
    fn unsubscribe_handler(
        &mut self,
        topic: &Topic,
        handler_to_remove: &TopicHandler,
    ) -> Result<(), JsValue>;
    // Add other methods used by consumers if any (e.g., route_incoming_message if needed elsewhere)
    // fn route_incoming_message(&self, message: serde_json::Value); // Example
}

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

            let msg = create_subscribe(vec![topic]);
            let msg_json = serde_json::to_string(&msg)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

            match self.ws_client.try_borrow() {
                Ok(client) => client.send_serialized_message(&msg_json)?,
                Err(_) => return Err(JsValue::from_str("Failed to borrow WsClient for subscribe")),
            }
        }

        Ok(())
    }

    /// Unsubscribe from a topic, removing all handlers associated with it.
    ///
    /// If handlers existed for the topic, an "unsubscribe" message is sent to the backend.
    /// Note: This removes *all* handlers for the given topic.
    #[allow(dead_code)]
    pub fn unsubscribe(&mut self, topic: &Topic) -> Result<(), JsValue> {
        web_sys::console::warn_1(&"Using unsubscribe by topic - this removes ALL handlers. Consider using unsubscribe_handler.".into());
        // Remove handlers only if the topic exists in our handler map
        if self.topic_handlers.remove(topic).is_some() {
            // Only unsubscribe from the backend if we were actually subscribed
            if self.subscribed_topics.remove(topic) {
                self.send_unsubscribe_message(topic)?;
            } else {
                web_sys::console::log_1(
                    &format!(
                        "Removed local handlers for topic {} but was not subscribed on backend.",
                        topic
                    )
                    .into(),
                );
            }
        } else {
            web_sys::console::log_1(
                &format!(
                    "Attempted to unsubscribe from topic {} with no handlers.",
                    topic
                )
                .into(),
            );
        }

        Ok(())
    }

    /// Unsubscribes a specific handler from a topic.
    ///
    /// If this was the last handler for the topic, an "unsubscribe" message is sent to the backend.
    #[allow(dead_code)]
    pub fn unsubscribe_handler(
        &mut self,
        topic: &Topic,
        handler_to_remove: &TopicHandler,
    ) -> Result<(), JsValue> {
        let mut removed = false;
        let mut topic_is_empty = false;

        if let Some(handlers) = self.topic_handlers.get_mut(topic) {
            // Find the position of the handler to remove using Rc pointer equality
            if let Some(pos) = handlers
                .iter()
                .position(|h| Rc::ptr_eq(h, handler_to_remove))
            {
                handlers.remove(pos);
                web_sys::console::log_1(
                    &format!("Removed specific handler for topic: {}", topic).into(),
                );
                removed = true;
                topic_is_empty = handlers.is_empty();
            } else {
                web_sys::console::warn_1(
                    &format!("Handler not found for topic {} during unsubscribe.", topic).into(),
                );
            }
        }

        // If we removed a handler and the topic list is now empty, clean up the topic subscription
        if removed && topic_is_empty {
            web_sys::console::log_1(
                &format!(
                    "Last handler removed for topic: {}. Cleaning up subscription.",
                    topic
                )
                .into(),
            );
            self.topic_handlers.remove(topic);
            // Only send unsubscribe to backend if we were actually tracking this subscription
            if self.subscribed_topics.remove(topic) {
                self.send_unsubscribe_message(topic)?;
            }
        }

        Ok(())
    }

    // Helper to send the unsubscribe message
    fn send_unsubscribe_message(&self, topic: &Topic) -> Result<(), JsValue> {
        web_sys::console::log_1(
            &format!("Sending unsubscribe request for topic: {}", topic).into(),
        );
        let msg = create_unsubscribe(vec![topic.clone()]);
        let msg_json = serde_json::to_string(&msg)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

        match self.ws_client.try_borrow() {
            Ok(client) => client.send_serialized_message(&msg_json)?,
            Err(_) => {
                return Err(JsValue::from_str(
                    "Failed to borrow WsClient for unsubscribe",
                ))
            }
        }
        Ok(())
    }

    /// Resubscribes to all topics currently tracked in `subscribed_topics`.
    /// Typically called after a successful reconnection.
    pub fn resubscribe_all_topics(&self) -> Result<(), JsValue> {
        web_sys::console::log_1(&"Resubscribing to all topics after connection...".into());
        let topics_to_resubscribe: Vec<Topic> = self.subscribed_topics.iter().cloned().collect();

        if !topics_to_resubscribe.is_empty() {
            web_sys::console::log_1(
                &format!(
                    "Sending resubscribe request for topics: {:?}",
                    topics_to_resubscribe
                )
                .into(),
            );
            let msg = create_subscribe(topics_to_resubscribe);
            let msg_json = serde_json::to_string(&msg)
                .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

            match self.ws_client.try_borrow() {
                Ok(client) => client.send_serialized_message(&msg_json)?,
                Err(_) => {
                    return Err(JsValue::from_str(
                        "Failed to borrow WsClient for resubscribe",
                    ))
                }
            }
        } else {
            web_sys::console::log_1(&"No topics to resubscribe.".into());
        }

        Ok(())
    }

    /// Routes an incoming message (parsed JSON) to the appropriate topic handlers.
    /// Determines the topic based on message content (e.g., `type` and `data` fields).
    pub fn route_incoming_message(&self, message: serde_json::Value) {
        use crate::network::ws_schema::Envelope;

        // Attempt to parse as a v2 Envelope first
        if let Ok(envelope) = serde_json::from_value::<Envelope>(message.clone()) {
            // v2 path: need to transform Envelope → legacy flat shape that
            // downstream handlers (`ChatViewWsManager`, dashboard managers…)
            // still expect: `{ "type": "…", <payload-fields…> }`.
            // Move every key from `envelope.data` to the root object and add
            // the `type` attribute from the envelope itself so the existing
            // `WsMessage` enum can deserialize successfully.

            use serde_json::{json, Value};

            // For certain message types, we need to preserve the nested structure
            // instead of flattening. This matches what the WsMessage enum expects.
            let message_type = envelope.r#type.to_lowercase();
            let needs_data_wrapper = matches!(
                message_type.as_str(),
                "execution_finished" | "node_state" | "node_log" | "user_update"
            );

            if needs_data_wrapper {
                // Keep the data nested as expected by WsMessage enum variants
                let wrapped = json!({
                    "type": message_type,
                    "data": envelope.data,
                });
                self.dispatch_to_topic_handlers(&envelope.topic, wrapped);
            } else {
                // Original flattening behavior for other message types
                let mut merged = match envelope.data {
                    Value::Object(map) => map,
                    other => {
                        // In the unlikely case `data` is not an object pass the
                        // original envelope for debugging – downstream will log
                        // an error for unmatched shape.
                        self.dispatch_to_topic_handlers(
                            &envelope.topic,
                            json!({
                                "type": envelope.r#type,
                                "data": other,
                            }),
                        );
                        return;
                    }
                };

                // Insert the `type` discriminator (always lower-case to match
                // `WsMessage` discriminators and avoid a huge alias list).
                merged.insert("type".to_string(), Value::String(message_type));

                self.dispatch_to_topic_handlers(&envelope.topic, Value::Object(merged));
            }
        } else {
            // Fallback to v1 parsing
            use crate::network::ws_schema::WsMessage;
            if let Ok(parsed) = serde_json::from_value::<WsMessage>(message.clone()) {
                if let Some(topic_str) = parsed.topic() {
                    self.dispatch_to_topic_handlers(&topic_str, message);
                }
            } else {
                web_sys::console::error_1(
                    &format!(
                        "Failed to parse incoming message as Envelope or WsMessage: {:?}",
                        message
                    )
                    .into(),
                );
            }
        }
    }

    /// Helper: run all handlers registered for *topic* with the provided
    /// payload.
    ///
    /// **Borrowing caveat** – Executing handlers **inside** the immutable
    /// borrow of `TopicManager` (established via `RefCell::borrow()` in the
    /// WebSocket `on_message` callback) causes a `BorrowMutError` whenever a
    /// handler dispatches a message that, in turn, tries to obtain a *mutable*
    /// borrow of the same `TopicManager` (e.g. to subscribe to a new topic).
    ///
    /// To avoid this re-entrancy issue we:
    /// 1. Clone the relevant handler list while the immutable borrow is still
    ///    active.
    /// 2. Drop that borrow (by letting the `handlers_ref` go out of scope)
    ///    *before* executing any user code.
    /// 3. Execute each handler **asynchronously** on the next micro-task via
    ///    `wasm_bindgen_futures::spawn_local`, ensuring the original borrow
    ///    is fully released.
    fn dispatch_to_topic_handlers(&self, topic: &str, payload: serde_json::Value) {
        use wasm_bindgen_futures::spawn_local;

        // 1. Clone handlers while the immutable borrow of `self` is held.
        let handlers_cloned: Option<Vec<TopicHandler>> = self
            .topic_handlers
            .get(topic)
            .map(|vec| vec.iter().cloned().collect());

        // 2. Borrow ends here (handlers_cloned owns independent `Rc`s).

        if let Some(handlers) = handlers_cloned {
            for handler_rc in handlers {
                let payload_clone = payload.clone();
                spawn_local(async move {
                    // Run handler in its own micro-task so we never execute
                    // user callbacks while a borrow on the TopicManager is
                    // active.  This eliminates the runtime RefCell panic seen
                    // on rapid subscribe/unsubscribe cycles.
                    (handler_rc.borrow_mut())(payload_clone);
                });
            }
        } else {
            web_sys::console::debug_1(
                &format!("No handlers registered for determined topic: {}", topic).into(),
            );
        }
    }

    #[cfg(test)]
    pub fn has_subscription(&self, topic: &str) -> bool {
        self.subscribed_topics.contains(topic)
    }
}

// --- Implement Trait for Real TopicManager ---
impl ITopicManager for TopicManager {
    fn subscribe(&mut self, topic: Topic, handler: TopicHandler) -> Result<(), JsValue> {
        // Delegate to the inherent implementation to avoid duplication.
        TopicManager::subscribe(self, topic, handler)
    }

    fn unsubscribe_handler(
        &mut self,
        topic: &Topic,
        handler_to_remove: &TopicHandler,
    ) -> Result<(), JsValue> {
        TopicManager::unsubscribe_handler(self, topic, handler_to_remove)
    }

    // Implement other trait methods if they were added to ITopicManager
    // fn route_incoming_message(&self, message: serde_json::Value) {
    //     self.route_incoming_message(message); // Just call the existing method
    // }
}

// Remove old `unsubscribe` if no longer needed, or keep if used elsewhere.
// pub fn unsubscribe(&mut self, topic: &Topic) -> Result<(), JsValue> { ... }
