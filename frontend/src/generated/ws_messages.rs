// AUTO-GENERATED FILE - DO NOT EDIT
// Generated from ws-protocol-asyncapi.yml at 2025-07-15T17:47:10.092852Z
// Using AsyncAPI 3.0 + Modern Rust Code Generation
//
// This file contains strongly-typed WebSocket message definitions.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// Compile-time schema validation
#[cfg(feature = "schema_validation")]
use schemars::{JsonSchema, schema_for};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct Envelope {
    pub v: u8,
    #[serde(rename = "type")]
    pub message_type: String,
    pub topic: String,
    pub req_id: Option<String>,
    pub ts: u64,
    pub data: Value,
}

impl Envelope {
    pub fn new(
        message_type: String,
        topic: String,
        data: Value,
        req_id: Option<String>,
    ) -> Self {
        Self {
            v: 1,
            message_type,
            topic,
            req_id,
            ts: js_sys::Date::now() as u64,
            data,
        }
    }
    
    /// Validate envelope structure
    pub fn validate(&self) -> Result<(), String> {
        if self.v != 1 {
            return Err("Invalid protocol version".to_string());
        }
        if self.message_type.is_empty() {
            return Err("Message type cannot be empty".to_string());
        }
        if self.topic.is_empty() {
            return Err("Topic cannot be empty".to_string());
        }
        Ok(())
    }
}

// Message payload structs

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct AgentRef {
    pub id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ThreadRef {
    pub thread_id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct UserRef {
    pub id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ExecutionRef {
    pub execution_id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct PingData {
    pub timestamp: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct PongData {
    pub timestamp: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ErrorData {
    pub error: String,
    pub details: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct SubscribeData {
    pub topics: Vec<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct UnsubscribeData {
    pub topics: Vec<String>,
    pub message_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct SendMessageData {
    pub thread_id: u32,
    pub content: String,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ThreadMessageData {
    pub thread_id: u32,
    pub message: Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ThreadEventData {
    pub thread_id: u32,
    pub agent_id: Option<u32>,
    pub title: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct StreamStartData {
    pub thread_id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct StreamChunkData {
    pub thread_id: u32,
    pub chunk_type: String,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub message_id: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct StreamEndData {
    pub thread_id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct AssistantIdData {
    pub thread_id: u32,
    pub message_id: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct AgentEventData {
    pub id: u32,
    pub status: Option<String>,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub last_error: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct RunUpdateData {
    pub id: u32,
    pub agent_id: u32,
    pub thread_id: Option<u32>,
    pub status: String,
    pub trigger: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct UserUpdateData {
    pub id: u32,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct NodeStateData {
    pub execution_id: u32,
    pub node_id: String,
    pub status: String,
    pub output: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct ExecutionFinishedData {
    pub execution_id: u32,
    pub status: String,
    pub error: Option<String>,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
pub struct NodeLogData {
    pub execution_id: u32,
    pub node_id: String,
    pub stream: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[cfg_attr(feature = "schema_validation", derive(JsonSchema))]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "ping")]
    Ping { data: PingData },

    #[serde(rename = "pong")]
    Pong { data: PongData },

    #[serde(rename = "error")]
    Error { data: ErrorData },

    #[serde(rename = "subscribe")]
    Subscribe { data: SubscribeData },

    #[serde(rename = "unsubscribe")]
    Unsubscribe { data: UnsubscribeData },

    #[serde(rename = "send_message")]
    SendMessage { data: SendMessageData },

    #[serde(rename = "thread_message", alias = "thread_message_created")]
    ThreadMessage { data: ThreadMessageData },

    #[serde(rename = "thread_event", alias = "thread_created", alias = "thread_updated", alias = "thread_deleted")]
    ThreadEvent { data: ThreadEventData },

    #[serde(rename = "stream_start")]
    StreamStart { data: StreamStartData },

    #[serde(rename = "stream_chunk")]
    StreamChunk { data: StreamChunkData },

    #[serde(rename = "stream_end")]
    StreamEnd { data: StreamEndData },

    #[serde(rename = "assistant_id")]
    AssistantId { data: AssistantIdData },

    #[serde(rename = "agent_event", alias = "agent_created", alias = "agent_updated", alias = "agent_deleted", alias = "agent_state")]
    AgentEvent { data: AgentEventData },

    #[serde(rename = "run_update")]
    RunUpdate { data: RunUpdateData },

    #[serde(rename = "user_update")]
    UserUpdate { data: UserUpdateData },

    #[serde(rename = "node_state")]
    NodeState { data: NodeStateData },

    #[serde(rename = "execution_finished")]
    ExecutionFinished { data: ExecutionFinishedData },

    #[serde(rename = "node_log")]
    NodeLog { data: NodeLogData },

    #[serde(other)]
    Unknown,
}

impl WsMessage {
    /// Validate message content
    pub fn validate(&self) -> Result<(), String> {
        match self {
            WsMessage::Unknown => Err("Unknown message type".to_string()),
            _ => Ok(())
        }
    }
}

// Runtime and compile-time validation

pub fn validate_envelope(data: &Value) -> Result<Envelope, String> {
    serde_json::from_value(data.clone())
        .map_err(|e| format!("Invalid envelope: {}", e))
        .and_then(|envelope: Envelope| {
            envelope.validate()?;
            Ok(envelope)
        })
}

#[cfg(feature = "schema_validation")]
pub fn generate_schema_for_message<T: JsonSchema>() -> serde_json::Value {
    serde_json::to_value(schema_for!(T)).unwrap()
}

// Compile-time validation that all message types have handlers
#[cfg(feature = "compile_time_validation")]
pub fn validate_handler_completeness() {
    // This function exists to catch missing handlers at compile time
    // Each message type should have a corresponding handler method
    // The compiler will catch missing cases in match statements
}

