use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
pub enum NodeType {
    UserInput,
    AgentResponse,
    AgentIdentity, // For agents that can receive instructions
}

/// Message represents a conversation entry between user and agent
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,      // "user" or "assistant"
    pub content: String,   // The actual message text
    pub timestamp: u64,    // Unix timestamp
}

/// Node represents a visual element in our graph
#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub x: f64,
    pub y: f64,
    pub text: String,
    pub width: f64,
    pub height: f64,
    pub color: String,
    pub parent_id: Option<String>,
    pub node_type: NodeType,
    
    // New fields for agent functionality
    pub system_instructions: Option<String>,  // System-level instructions for agents
    pub task_instructions: Option<String>,    // Persistent task instructions for what the agent should do when run
    pub history: Option<Vec<Message>>,        // Conversation history for this node
    pub status: Option<String>,               // "idle", "processing", "error", etc.
}

// New API models that match the backend schema
// These are used for API requests and responses

/// ApiAgent represents an agent in the backend database
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiAgent {
    pub id: Option<u32>,
    pub name: String,
    pub status: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub schedule: Option<String>,
    pub config: Option<serde_json::Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// ApiAgentCreate is used when creating a new agent
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiAgentCreate {
    pub name: String,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub schedule: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// ApiAgentUpdate is used when updating an existing agent
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiAgentUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub schedule: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// ApiMessage represents a message in the agent conversation
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub id: Option<u32>,
    pub agent_id: u32,
    pub role: String,
    pub content: String,
    pub created_at: Option<String>,
}

/// ApiMessageCreate is used when creating a new message
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiMessageCreate {
    pub role: String,
    pub content: String,
} 