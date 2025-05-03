//! Typed structures for WebSocket messages produced by the backend.
//!
//! Keeping a strongly-typed layer simplifies routing and shrinks the amount of
//! stringly-typed JSON plumbing in upper layers.
//! The structs purposefully include only the fields the *frontend* currently
//! needs – additional attributes sent by the backend are captured via Serde
//! `flatten` so they do not break deserialisation when new fields appear.

use serde::Deserialize;
use serde_json::Value;

/// Top-level WebSocket frame coming from the backend.
///
/// The backend always uses `{ "type": ..., "data": {...} }`.
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "run_update")]
    RunUpdate { data: WsRunUpdate },

    // Accept both the generic `agent_event` wrapper and the more specialised
    // *event_type* labels emitted by the backend event bus ("agent_updated",
    // "agent_created", "agent_deleted" ...).  They all carry the same payload
    // shape so we don’t distinguish further on the frontend.
    #[serde(rename = "agent_event", alias = "agent_state", alias = "agent_created", alias = "agent_updated", alias = "agent_deleted")]
    AgentEvent { data: WsAgentEvent },

    // Thread lifecycle updates share payload structure – expose a single
    // variant and accept all specific type strings via alias.
    #[serde(
        rename = "thread_created",
        alias = "thread_updated",
        alias = "thread_deleted",
        alias = "thread_event"
    )]
    ThreadEvent { data: WsThreadEvent },

    // Token-level streaming coming from AgentRunner.
    // Streaming events are emitted by the backend with *flat* payload – no
    // nested `data` object.  Therefore we map them directly onto the helper
    // structs using *tuple* variants.
    #[serde(rename = "stream_start")]
    StreamStart(WsStreamStart),
    #[serde(rename = "stream_chunk")]
    StreamChunk(WsStreamChunk),
    #[serde(rename = "stream_end")]
    StreamEnd(WsStreamEnd),

    // Phase-2: id of the assistant bubble currently being streamed.
    #[serde(rename = "assistant_id")]
    AssistantId(WsAssistantId),

    // A freshly created message in a thread – emitted both when the user
    // posts a message (`thread_message`) and when the backend inserts a new
    // assistant/tool message automatically (alias `thread_message_created`).
    #[serde(
        rename = "thread_message",
        alias = "thread_message_created"
    )]
    ThreadMessage { data: WsThreadMessage },

    #[serde(other)]
    Unknown,
}

/// Slim payload for a run_update event.
/// Field names follow the backend schema.  The backend sometimes emits
/// `run_id` instead of `id`, therefore we accept both via `alias`.
#[derive(Debug, Deserialize, Clone)]
pub struct WsRunUpdate {
    #[serde(alias = "id", alias = "run_id")]
    pub id: u32,

    pub agent_id: u32,

    #[serde(default)]
    pub thread_id: Option<u32>,

    pub status: String, // queued | running | success | failed

    #[serde(default)]
    pub trigger: Option<String>, // manual | schedule | api

    #[serde(default)]
    pub started_at: Option<String>,
    #[serde(default)]
    pub finished_at: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,

    #[serde(default)]
    pub error: Option<String>,

    #[serde(flatten)]
    pub extra: Value, // Future-proofing for new keys
}

/// Slim payload for agent_event (status / metadata update).
#[derive(Debug, Deserialize, Clone)]
pub struct WsAgentEvent {
    pub id: u32,

    #[serde(default)]
    pub status: Option<String>,

    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub next_run_at: Option<String>,

    #[serde(default)]
    pub last_error: Option<String>,

    #[serde(flatten)]
    pub extra: Value,
}

/// Thread-level events (creation, update, message inserted, deletion).
/// Backend always includes a `thread_id` so we can determine the topic.
#[derive(Debug, Deserialize, Clone)]
pub struct WsThreadEvent {
    pub thread_id: u32,

    #[serde(default)]
    pub agent_id: Option<u32>,

    #[serde(flatten)]
    pub extra: Value,
}

/// Streaming helper payloads – *large* fields omitted on purpose, handlers can
/// fetch extra data if necessary.
#[derive(Debug, Deserialize, Clone)]
pub struct WsStreamStart {
    pub thread_id: u32,

    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WsStreamChunk {
    pub thread_id: u32,

    // chunk_type: assistant_token, assistant_message, tool_output …
    pub chunk_type: String,

    #[serde(default)]
    pub content: Option<String>,

    #[serde(flatten)]
    pub extra: Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WsStreamEnd {
    pub thread_id: u32,

    #[serde(flatten)]
    pub extra: Value,
}

// ---------------------------------------------------------------------------
//   AssistantId helper payload
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Clone)]
pub struct WsAssistantId {
    pub thread_id: u32,
    pub message_id: u32,
}

// ---------------------------------------------------------------------------
//   Conversions into full REST models used by the dashboard UI
// ---------------------------------------------------------------------------

use crate::models::ApiAgentRun;

impl From<WsRunUpdate> for ApiAgentRun {
    fn from(ws: WsRunUpdate) -> Self {
        ApiAgentRun {
            id: ws.id,
            agent_id: ws.agent_id,
            thread_id: ws.thread_id,
            status: ws.status,
            trigger: ws.trigger.unwrap_or_else(|| "manual".to_string()),
            started_at: ws.started_at,
            finished_at: ws.finished_at,
            duration_ms: ws.duration_ms,
            total_tokens: None,
            total_cost_usd: None,
            error: ws.error,
        }
    }
}

// ---------------------------------------------------------------------------
//   Helper utilities
// ---------------------------------------------------------------------------

impl WsMessage {
    /// Return the topic string ("agent:{id}" / "thread:{id}") this message
    /// belongs to.  Returns `None` for administrative frames such as pong or
    /// messages we do not recognise.
    pub fn topic(&self) -> Option<String> {
        match self {
            WsMessage::RunUpdate { data } => Some(format!("agent:{}", data.agent_id)),
            WsMessage::AgentEvent { data } => Some(format!("agent:{}", data.id)),
            WsMessage::ThreadEvent { data } => Some(format!("thread:{}", data.thread_id)),
            WsMessage::StreamStart(data) => Some(format!("thread:{}", data.thread_id)),
            WsMessage::StreamChunk(data) => Some(format!("thread:{}", data.thread_id)),
            WsMessage::StreamEnd(data) => Some(format!("thread:{}", data.thread_id)),
            WsMessage::AssistantId(data) => Some(format!("thread:{}", data.thread_id)),
            WsMessage::ThreadMessage { data } => Some(format!("thread:{}", data.thread_id)),
            WsMessage::Unknown => None,
        }
    }
}

// ---------------------------------------------------------------------------
//   Thread message helpers
// ---------------------------------------------------------------------------

use crate::models::ApiThreadMessage;

/// Payload for an individual message inserted into a thread.
#[derive(Debug, Deserialize, Clone)]
pub struct WsThreadMessage {
    pub thread_id: u32,

    pub message: ApiThreadMessage,

    #[serde(flatten)]
    pub extra: Value,
}

// Allow convenient conversion when the dashboard/chat needs the actual
// `ApiThreadMessage` instance.
impl From<WsThreadMessage> for ApiThreadMessage {
    fn from(ws: WsThreadMessage) -> Self {
        ws.message
    }
}
