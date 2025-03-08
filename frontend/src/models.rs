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