use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum NodeType {
    UserInput,
    AgentResponse,
    AgentIdentity, // For future use to represent persistent agents
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
} 