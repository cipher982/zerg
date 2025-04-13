use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents all possible WebSocket message types in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    // Connection messages
    Connect,
    Disconnect,
    Error,
    Ping,
    Pong,
    
    // Generic Subscriptions (NEW)
    Subscribe,
    Unsubscribe,
    UnsubscribeSuccess,
    
    // Thread messages
    SubscribeThread,
    ThreadHistory,
    ThreadMessage,
    SendMessage,
    
    // Streaming messages
    StreamStart,
    StreamChunk,
    StreamEnd,
    
    // System events
    SystemStatus,
    
    // Agent messages
    AgentState,
    
    // Special cases
    Unknown,
}

/// Represents all possible event types that can be received from the backend
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Agent events
    AgentCreated,
    AgentUpdated,
    AgentDeleted,
    
    // Thread events
    ThreadCreated,
    ThreadUpdated,
    ThreadDeleted,
    ThreadMessageCreated,
    
    // System events
    SystemStatus,
    Error,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Convert enum variants to snake_case strings
        let s = match self {
            MessageType::Connect => "connect",
            MessageType::Disconnect => "disconnect",
            MessageType::Error => "error",
            MessageType::Ping => "ping",
            MessageType::Pong => "pong",
            MessageType::Subscribe => "subscribe",
            MessageType::Unsubscribe => "unsubscribe",
            MessageType::SubscribeThread => "subscribe_thread",
            MessageType::ThreadHistory => "thread_history",
            MessageType::ThreadMessage => "thread_message",
            MessageType::SendMessage => "send_message",
            MessageType::StreamStart => "stream_start",
            MessageType::StreamChunk => "stream_chunk",
            MessageType::StreamEnd => "stream_end",
            MessageType::SystemStatus => "system_status",
            MessageType::AgentState => "agent_state",
            MessageType::Unknown => "unknown",
            MessageType::UnsubscribeSuccess => "unsubscribe_success",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EventType::AgentCreated => "agent_created",
            EventType::AgentUpdated => "agent_updated",
            EventType::AgentDeleted => "agent_deleted",
            EventType::ThreadCreated => "thread_created",
            EventType::ThreadUpdated => "thread_updated",
            EventType::ThreadDeleted => "thread_deleted",
            EventType::ThreadMessageCreated => "thread_message_created",
            EventType::SystemStatus => "system_status",
            EventType::Error => "error",
        };
        write!(f, "{}", s)
    }
}

/// Helper functions for topic formatting
pub mod topics {
    /// Creates a topic string for an agent subscription
    pub fn agent_topic(id: i32) -> String {
        format!("agent:{}", id)
    }

    /// Creates a topic string for a thread subscription
    pub fn thread_topic(id: i32) -> String {
        format!("thread:{}", id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_serialization() {
        let msg_type = MessageType::ThreadMessage;
        let serialized = serde_json::to_string(&msg_type).unwrap();
        assert_eq!(serialized, "\"thread_message\"");
        
        let deserialized: MessageType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, MessageType::ThreadMessage);
    }

    #[test]
    fn test_event_type_serialization() {
        let event_type = EventType::AgentCreated;
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"agent_created\"");
        
        let deserialized: EventType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, EventType::AgentCreated);
    }

    #[test]
    fn test_topic_formatting() {
        assert_eq!(topics::agent_topic(123), "agent:123");
        assert_eq!(topics::thread_topic(456), "thread:456");
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::Connect.to_string(), "connect");
        assert_eq!(MessageType::ThreadHistory.to_string(), "thread_history");
        assert_eq!(MessageType::StreamEnd.to_string(), "stream_end");
    }

    #[test]
    fn test_event_type_display() {
        assert_eq!(EventType::AgentCreated.to_string(), "agent_created");
        assert_eq!(EventType::ThreadMessageCreated.to_string(), "thread_message_created");
        assert_eq!(EventType::Error.to_string(), "error");
    }
} 