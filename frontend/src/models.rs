/*
 * Models for the frontend application.
 *
 * ARCHITECTURE OVERVIEW
 * ---------------------
 * This file defines the data structures for our application with a clear separation of concerns:
 *
 * 1. Agent Domain Models:
 *    - ApiAgent, ApiAgentCreate, ApiAgentUpdate - these represent the business logic of agents
 *    - They are API models that store only agent functionality data (no visual properties)
 *    - Stored in the backend database and cached in the frontend state.agents HashMap
 *
 * 2. Canvas/Visualization Models:
 *    - CanvasNode - purely frontend visual representation with x, y coordinates
 *    - References an Agent by ID but doesn't embed all agent properties
 *    - Stored in the frontend state.canvas_nodes HashMap
 *
 * 3. Workflow Models:
 *    - Workflow - collection of CanvasNodes and Edges forming a user-defined workflow
 *    - Edge - represents connections between nodes in a workflow
 *    - Stored in frontend state.workflows HashMap
 * 
 * BACKEND INTEGRATION
 * ------------------
 * The frontend models align with the backend FastAPI models as follows:
 * 
 * 1. Backend Agent Model:
 *    - Stored in the database with id, name, status, instructions, etc.
 *    - No visual/coordinate properties (x, y)
 *    - Accessed via /api/agents/ endpoints
 * 
 * 2. Frontend Agent Representation:
 *    - ApiAgent instances are loaded from the backend and stored in state.agents
 *    - CanvasNode instances reference agents by ID and add visual properties
 *    - Changes to ApiAgent instances are synced with the backend
 *
 * 3. Frontend-only Models (for now):
 *    - Workflow and Edge are currently frontend-only and stored in localStorage
 *    - Can be extended to backend persistence in the future
 *
 * The original Node struct is gradually being phased out in favor of this separation.
 * This separation ensures that agent logic is independent of its visual representation,
 * allowing for cleaner code, better persistence, and more advanced visualizations.
 */

use serde::{Serialize, Deserialize};

/// Type of node (e.g., AgentIdentity, UserInput, ResponseOutput)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    AgentIdentity,
    UserInput, 
    ResponseOutput,
    GenericNode,
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

/// CanvasNode represents a visual element on the canvas with layout information
/// This separates the visual/layout concerns from the agent business logic
#[derive(Clone, Serialize, Deserialize)]
pub struct CanvasNode {
    pub node_id: String,           // Unique identifier for this canvas node
    pub agent_id: Option<u32>,     // Optional reference to a backend agent
    pub x: f64,                    // X position on canvas
    pub y: f64,                    // Y position on canvas
    pub width: f64,                // Node width
    pub height: f64,               // Node height
    pub color: String,             // Node color
    pub text: String,              // Display text for the node
    pub node_type: NodeType,       // Type of node (AgentIdentity, UserInput, etc.)
    pub parent_id: Option<String>, // Optional parent node reference
    pub is_selected: bool,
    pub is_dragging: bool,
}

/// Edge represents a connection between two nodes in a workflow
#[derive(Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,                // Unique identifier for this edge
    pub from_node_id: String,      // Source node ID
    pub to_node_id: String,        // Target node ID
    pub label: Option<String>,     // Optional label for the edge
}

/// Workflow represents a collection of nodes and edges that form a complete workflow
#[derive(Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u32,                   // Unique identifier for this workflow
    pub name: String,              // Name of the workflow
    pub nodes: Vec<CanvasNode>,    // Nodes in this workflow
    pub edges: Vec<Edge>,          // Edges connecting nodes in this workflow
}

// New API models that match the backend schema
// These are used for API requests and responses

/// ApiAgent represents an agent in the backend database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiAgent {
    pub id: Option<u32>,
    pub name: String,
    pub status: Option<String>,
    pub system_instructions: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    // Add other fields as needed
}

/// ApiAgentCreate is used when creating a new agent
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiAgentCreate {
    pub name: String,
    pub system_instructions: String,
    pub task_instructions: String,
    pub model: Option<String>,
    pub schedule: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// ApiAgentUpdate is used when updating an existing agent
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiAgentUpdate {
    pub name: Option<String>,
    pub status: Option<String>,
    pub system_instructions: Option<String>,
    pub task_instructions: Option<String>,
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