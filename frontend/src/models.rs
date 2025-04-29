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

// Needed for AgentDebug modal (Phase 1)
use serde_json::Value;

/// Type of node (e.g., AgentIdentity, UserInput, ResponseOutput)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    AgentIdentity,
    UserInput, 
    ResponseOutput,
    GenericNode,
}

/// Message represents a thread entry between user and agent
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
    // -------- Scheduling metadata (Milestone 0) --------
    /// Cron expression defining when the agent should run (None means not scheduled)
    pub schedule: Option<String>,
    /// ISO‑8601 timestamp of the next scheduled run (set by backend)
    pub next_run_at: Option<String>,
    /// ISO‑8601 timestamp when the agent last finished a run
    pub last_run_at: Option<String>,

    /// If the last run produced an error the backend stores the message here.
    /// `None` or empty string means the agent is healthy.
    #[serde(default)]
    pub last_error: Option<String>,
}

// -----------------------------------------------------------------------------
// Convenience impls
// -----------------------------------------------------------------------------

impl ApiAgent {
    /// Returns `true` if the `schedule` field contains a non-empty cron string.
    ///
    /// The backend no longer stores a separate `run_on_schedule` flag – an
    /// agent is considered *scheduled* whenever the cron expression is set.
    pub fn is_scheduled(&self) -> bool {
        self.schedule
            .as_ref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_is_scheduled() {
        // Case 1: None -> not scheduled
        let agent_none = ApiAgent {
            id: Some(1),
            name: "A".to_string(),
            status: None,
            system_instructions: None,
            model: None,
            temperature: None,
            created_at: None,
            updated_at: None,
            schedule: None,
            next_run_at: None,
            last_run_at: None,
            last_error: None,
        };
        assert!(!agent_none.is_scheduled());

        // Case 2: Empty string -> not scheduled
        let agent_empty = ApiAgent { schedule: Some("   ".to_string()), ..agent_none.clone() };
        assert!(!agent_empty.is_scheduled());

        // Case 3: Valid cron -> scheduled
        let agent_cron = ApiAgent { schedule: Some("0 * * * *".to_string()), ..agent_none };
        assert!(agent_cron.is_scheduled());
    }
}

/// Wrapper returned by `/api/agents/{id}/details`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiAgentDetails {
    pub agent: ApiAgent,

    // Optional heavy payloads – currently None / empty in Phase 1
    #[serde(default)]
    pub threads: Option<Vec<ApiThread>>,

    #[serde(default)]
    pub runs: Option<Vec<Value>>,

    #[serde(default)]
    pub stats: Option<Value>,
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

    /// Optional error string – only present if we need to update/clear it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// ApiMessage represents a message in the agent thread
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub id: Option<u32>,
    pub agent_id: u32,
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
}

/// ApiMessageCreate is used when creating a new message
#[derive(Clone, Serialize, Deserialize)]
pub struct ApiMessageCreate {
    pub role: String,
    pub content: String,
}

/// Thread model from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThread {
    pub id: Option<u32>,
    pub agent_id: u32,
    pub title: String,
    pub active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadCreate {
    pub agent_id: u32,
    pub title: String,
    pub active: bool,
}

/// Update a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadUpdate {
    pub title: Option<String>,
    pub active: Option<bool>,
}

/// ApiThreadMessage represents a message in a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadMessage {
    pub id: Option<u32>,
    pub thread_id: u32,
    pub role: String,
    pub content: String,
    pub timestamp: Option<String>,
    // New fields for tool message display
    #[serde(default)]
    pub message_type: Option<String>,  // "tool_output" or "assistant_message"
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    /// Optional serialized inputs for the tool call (if provided by backend)
    #[serde(default)]
    pub tool_input: Option<String>,
    #[serde(default)]
    pub parent_id: Option<u32>,
}

/// Create a new thread message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadMessageCreate {
    pub role: String,
    pub content: String,
}

/// Extension methods for Node to provide backward compatibility with legacy code
#[allow(dead_code)] // These methods are kept for backward compatibility
impl Node {
    // Property getters that map old field names to new ones
    pub fn _id(&self) -> String {
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
    pub fn _set_id(&mut self, id: String) {
        self.node_id = id;
    }
    
    pub fn set_system_instructions(&mut self, _instructions: Option<String>) {
        // Don't modify agent data from a node method
        // This should be done through an explicit agent update message
    }
    
    pub fn _set_task_instructions(&mut self, _instructions: Option<String>) {
        // No direct mapping in new model
    }
    
    pub fn _set_history(&mut self, _history: Option<Vec<crate::models::Message>>) {
        // No direct mapping in new model
    }
    
    pub fn set_status(&mut self, _status: Option<String>) {
        // Don't modify agent data from a node method
        // This should be done through an explicit agent update message
    }
} 