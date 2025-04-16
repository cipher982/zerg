use serde::{Deserialize, Serialize};
use uuid::Uuid;
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
    
    pub fn create_ping() -> PingMessage {
        PingMessage {
            message_type: MessageType::Ping,
            message_id: Some(Uuid::new_v4().to_string()),
            timestamp: Some(js_sys::Date::now() as i64),
        }
    }

    pub fn create_subscribe(topics: Vec<String>) -> SubscribeMessage {
        SubscribeMessage {
            message_type: MessageType::Subscribe,
            message_id: Some(Uuid::new_v4().to_string()),
            topics,
        }
    }

    pub fn create_unsubscribe(topics: Vec<String>) -> UnsubscribeMessage {
        UnsubscribeMessage {
            message_type: MessageType::Unsubscribe,
            message_id: Some(Uuid::new_v4().to_string()),
            topics,
        }
    }
    
    pub fn create_models_request() -> BaseMessage {
        BaseMessage {
            message_type: MessageType::Models,
            message_id: Some(Uuid::new_v4().to_string()),
        }
    }
}

/// Message parsing and handling
pub mod handlers {
    use super::*;
    use web_sys::console;
    use serde::de::Error;

    /// Parse a raw JSON message into the appropriate message type
    pub fn parse_message(json: &str) -> Result<Box<dyn WsMessage>, serde_json::Error> {
        // First parse as generic Value to get the type
        let value: serde_json::Value = serde_json::from_str(json)?;
        
        // Get the message type
        let msg_type = value["type"].as_str().unwrap_or("unknown");
        
        // Parse into specific message type based on the type field
        match msg_type {
            "error" => Ok(Box::new(serde_json::from_str::<ErrorMessage>(json)?)),
            "pong" => Ok(Box::new(serde_json::from_str::<PongMessage>(json)?)),
            "thread_history" => Ok(Box::new(serde_json::from_str::<ThreadHistoryMessage>(json)?)),
            "stream_start" => Ok(Box::new(serde_json::from_str::<StreamStartMessage>(json)?)),
            "stream_chunk" => Ok(Box::new(serde_json::from_str::<StreamChunkMessage>(json)?)),
            "stream_end" => Ok(Box::new(serde_json::from_str::<StreamEndMessage>(json)?)),
            "unsubscribe_success" => Ok(Box::new(serde_json::from_str::<UnsubscribeSuccessMessage>(json)?)),
            _ => {
                console::warn_1(&format!("Unknown message type: {}", msg_type).into());
                Err(serde_json::Error::custom(format!("Unknown message type: {}", msg_type)))
            }
        }
    }
}

/// Trait for common message functionality
pub trait WsMessage: std::fmt::Debug {
    fn message_type(&self) -> MessageType;
    fn message_id(&self) -> Option<String>;
}

// Implement WsMessage for all message types
macro_rules! impl_ws_message {
    ($($t:ty),*) => {
        $(
            impl WsMessage for $t {
                fn message_type(&self) -> MessageType {
                    self.message_type.clone()
                }
                
                fn message_id(&self) -> Option<String> {
                    self.message_id.clone()
                }
            }
        )*
    }
}

impl_ws_message!(
    BaseMessage,
    ErrorMessage,
    PingMessage,
    PongMessage,
    SubscribeMessage,
    UnsubscribeMessage,
    UnsubscribeSuccessMessage,
    ThreadHistoryMessage,
    StreamStartMessage,
    StreamChunkMessage,
    StreamEndMessage,
    AgentStateMessage
);

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
        // Test error message parsing
        let error_json = r#"{
            "type": "error",
            "message_id": "test-123",
            "error": "Test error",
            "details": null
        }"#;
        
        let parsed = handlers::parse_message(error_json).unwrap();
        assert_eq!(parsed.message_type(), MessageType::Error);
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