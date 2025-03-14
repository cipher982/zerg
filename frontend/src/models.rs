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
 *    - Node - purely frontend visual representation with x, y coordinates
 *    - References an Agent by ID but doesn't embed all agent properties
 *    - Stored in the frontend state.nodes HashMap
 *
 * 3. Workflow Models:
 *    - Workflow - collection of Nodes and Edges forming a user-defined workflow
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
 *    - Node instances reference agents by ID and add visual properties
 *    - Changes to ApiAgent instances are synced with the backend
 *
 * 3. Frontend-only Models (for now):
 *    - Workflow and Edge are currently frontend-only and stored in localStorage
 *    - Can be extended to backend persistence in the future
 */

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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

/// Node represents a visual element on the canvas with layout information
/// This separates the visual/layout concerns from the agent business logic
#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub node_id: String,           // Unique identifier for this node
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
    pub nodes: Vec<Node>,          // Nodes in this workflow
    pub edges: Vec<Edge>,          // Edges connecting nodes in this workflow
}

// API models that match the backend schema
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

/// Extension methods for Node to provide backward compatibility with legacy code
impl Node {
    // Property getters that map old field names to new ones
    pub fn id(&self) -> String {
        self.node_id.clone()
    }
    
    pub fn system_instructions(&self) -> Option<String> {
        // Don't directly access agent data - this is just a node method
        // Return None to indicate nodes don't have system instructions
        None
    }
    
    pub fn task_instructions(&self) -> Option<String> {
        // For backward compatibility
        None
    }
    
    pub fn history(&self) -> Option<Vec<crate::models::Message>> {
        // In the new model, history is stored with agent data, not on nodes
        None
    }
    
    pub fn status(&self) -> Option<String> {
        // Don't directly access agent data - return None
        None
    }
    
    // New method that accepts agents HashMap to avoid APP_STATE borrowing
    pub fn get_status_from_agents(&self, agents: &HashMap<u32, ApiAgent>) -> Option<String> {
        // If node is linked to an agent, get its status directly from the provided agents HashMap
        if let Some(agent_id) = self.agent_id {
            agents.get(&agent_id).and_then(|agent| agent.status.clone())
        } else {
            None
        }
    }
    
    // Setters for backward compatibility
    pub fn set_id(&mut self, id: String) {
        self.node_id = id;
    }
    
    pub fn set_system_instructions(&mut self, _instructions: Option<String>) {
        // Don't modify agent data from a node method
        // This should be done through an explicit agent update message
    }
    
    pub fn set_task_instructions(&mut self, _instructions: Option<String>) {
        // No direct mapping in new model
    }
    
    pub fn set_history(&mut self, _history: Option<Vec<crate::models::Message>>) {
        // No direct mapping in new model
    }
    
    pub fn set_status(&mut self, _status: Option<String>) {
        // Don't modify agent data from a node method
        // This should be done through an explicit agent update message
    }
} 