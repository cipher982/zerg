use serde::{Deserialize, Serialize};
use super::event_types::MessageType;

/// Base message structure that all WebSocket messages must implement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
}

/// Error message sent when something goes wrong
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub error: String,
    pub details: Option<serde_json::Value>,
}

/// Client ping to keep connection alive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub timestamp: Option<i64>,
}

/// Server response to ping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PongMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub timestamp: Option<i64>,
}

/// Request to subscribe to topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub topics: Vec<String>,
}

/// Request to unsubscribe from topics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub topics: Vec<String>,
}

/// Thread history sent in response to subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadHistoryMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub thread_id: i32,
    pub messages: Vec<ThreadMessageData>,
}

/// Individual thread message data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadMessageData {
    pub id: Option<i32>,
    pub thread_id: i32,
    pub role: String,
    pub content: String,
    pub processed: Option<bool>,
    pub timestamp: Option<String>,
}

/// Message for stream start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStartMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub thread_id: i32,
}

/// Message for stream chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunkMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub thread_id: i32,
    pub content: String,
    // New fields for tool message display
    #[serde(default)]
    pub chunk_type: Option<String>,  // "tool_output" or "assistant_message"
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Optional serialized inputs for the tool call (if provided by backend)
    #[serde(default)]
    pub tool_input: Option<String>,
}

/// Message for stream end
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEndMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub thread_id: i32,
}

/// Data payload for agent-related events (AGENT_CREATED, AGENT_UPDATED, AGENT_DELETED)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventData {
    pub id: i32,
    pub name: String,
    pub status: Option<String>,
}

/// Data payload for THREAD_CREATED event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadCreatedEventData {
    pub thread_id: i32,
    pub agent_id: Option<i32>,
    pub title: String,
}

/// Data payload for THREAD_UPDATED event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadUpdatedEventData {
    pub thread_id: i32,
    pub title: Option<String>,
}

/// Data payload for THREAD_DELETED event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadDeletedEventData {
    pub thread_id: i32,
    pub agent_id: Option<i32>,
}

/// Data payload for THREAD_MESSAGE_CREATED event
/// This reuses the ThreadMessageData structure
#[allow(dead_code)]
pub type ThreadMessageCreatedEventData = ThreadMessageData;

/// Agent State Message (sent on successful agent topic subscription)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub data: AgentEventData,
    pub message_id: Option<String>,
}

/// Unsubscribe success message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeSuccessMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
}

/// Message containing available models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelListMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub message_id: Option<String>,
    pub models: Vec<serde_json::Value>,
}

/// Helper functions for creating messages
pub mod builders {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    /// Helper: generate a random UUID for the `message_id` field.
    fn new_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Build a `PingMessage` (typed) ready for serialisation.
    pub fn create_ping() -> PingMessage {
        PingMessage {
            message_type: MessageType::Ping,
            message_id: Some(new_id()),
            timestamp: Some(js_sys::Date::now() as i64),
        }
    }

    /// Build a `SubscribeMessage` for one or more topics.
    pub fn create_subscribe(topics: Vec<String>) -> SubscribeMessage {
        SubscribeMessage {
            message_type: MessageType::Subscribe,
            message_id: Some(new_id()),
            topics,
        }
    }

    /// Build an `UnsubscribeMessage` for the given topics.
    pub fn create_unsubscribe(topics: Vec<String>) -> UnsubscribeMessage {
        UnsubscribeMessage {
            message_type: MessageType::Unsubscribe,
            message_id: Some(new_id()),
            topics,
        }
    }

    #[allow(dead_code)]
    pub fn create_models_request() -> ModelListMessage {
        ModelListMessage {
            message_type: MessageType::Models,
            message_id: Some(new_id()),
            models: Vec::new(), // request carries empty list
        }
    }

    #[allow(dead_code)]
    pub fn create_send_message(thread_id: i32, content: &str) -> serde_json::Value {
        // For now keep raw JSON because `SendMessage` struct is not defined yet.
        json!({
            "type": "send_message",
            "message_id": new_id(),
            "thread_id": thread_id,
            "content": content
        })
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_ping_message_serialization() {
        let ping = builders::create_ping();
        let json = serde_json::to_string(&ping).unwrap();
        let parsed: PingMessage = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.message_type, MessageType::Ping);
        assert!(parsed.message_id.is_some());
        assert!(parsed.timestamp.is_some());
    }

    #[wasm_bindgen_test]
    fn test_subscribe_message() {
        let topics = vec!["thread:123".to_string(), "agent:456".to_string()];
        let msg = builders::create_subscribe(topics.clone());
        
        assert_eq!(msg.message_type, MessageType::Subscribe);
        assert!(msg.message_id.is_some());
        assert_eq!(msg.topics, topics);
    }

    #[wasm_bindgen_test]
    fn test_message_parsing() {
        // Regression check: generic JSON -> strongly-typed struct parsing for an
        // `error` WebSocket frame.

        let error_json = r#"{
            "type": "error",
            "message_id": "test-123",
            "error": "Test error",
            "details": null
        }"#;

        // Since the dedicated parsing helper was removed during the
        // WebSocket-refactor we can directly deserialize into `ErrorMessage`.
        let parsed: ErrorMessage = serde_json::from_str(error_json).unwrap();

        assert_eq!(parsed.message_type, MessageType::Error);
        assert_eq!(parsed.message_id.unwrap(), "test-123");
    }

    #[wasm_bindgen_test]
    fn test_agent_event_data_serialization() {
        let data = AgentEventData {
            id: 123,
            name: "Test Agent".to_string(),
            status: Some("active".to_string()),
        };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("\"id\":123"));
        assert!(json.contains("\"name\":\"Test Agent\""));
        assert!(json.contains("\"status\":\"active\""));
    }

    #[wasm_bindgen_test]
    fn test_thread_created_event_data_serialization() {
        let data = ThreadCreatedEventData {
            thread_id: 456,
            agent_id: Some(123),
            title: "New Conversation".to_string(),
        };
        let json = serde_json::to_string(&data).unwrap();
         assert!(json.contains("\"thread_id\":456"));
         assert!(json.contains("\"agent_id\":123"));
         assert!(json.contains("\"title\":\"New Conversation\""));
    }

    #[wasm_bindgen_test]
    fn test_agent_state_message_serialization() {
        let agent_data = AgentEventData {
            id: 789,
            name: "State Agent".to_string(),
            status: Some("idle".to_string()),
        };
        let msg = AgentStateMessage {
            message_type: MessageType::AgentState,
            data: agent_data,
            message_id: Some("msg-1".to_string()),
        };
         let json = serde_json::to_string(&msg).unwrap();
         assert!(json.contains("\"type\":\"agent_state\""));
         assert!(json.contains("\"message_id\":\"msg-1\""));
         assert!(json.contains("\"data\":{\"id\":789")); // Check nested data
    }
}
