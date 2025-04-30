//! Extension trait providing **legacy helper methods** that used to live on
//! `CanvasNode`.
//!
//! These helpers expose convenience accessors like `set_status()` that, in the
//! past, directly modified *agent* state from a *node* method – a violation of
//! the new separation‐of-concerns guidelines (see `node_agent_task.md`).  They
//! are kept temporarily so that existing call-sites continue to compile while
//! the refactor is completed.  **Do not add new dependencies on these
//! helpers.**  They will be deleted once downstream code has migrated to the
//! agent-centric APIs.

use std::collections::HashMap;

use crate::models::{ApiAgent, CanvasNode, Message};

/// Trait implemented for `CanvasNode` to offer legacy agent-related helpers.
pub trait NodeAgentLegacyExt {
    // Pure node helpers retained for compatibility
    fn _id(&self) -> String;
    fn _set_id(&mut self, id: String);
    fn history(&self) -> Option<Vec<Message>>;
    fn status(&self) -> Option<String>;
    fn get_status_from_agents(&self, agents: &HashMap<u32, ApiAgent>) -> Option<String>;

    fn set_system_instructions(&mut self, instructions: Option<String>);
    fn set_status(&mut self, status: Option<String>);

    // Additional helpers still referenced in codebase
    fn system_instructions(&self) -> Option<String>;
    fn task_instructions(&self) -> Option<String>;
    fn _set_task_instructions(&mut self, instructions: Option<String>);
    fn _set_history(&mut self, history: Option<Vec<Message>>);
}

// ---------------------------------------------------------------------------
// Temporary default implementations (no-op / stub)
// ---------------------------------------------------------------------------

impl NodeAgentLegacyExt for CanvasNode {
    fn history(&self) -> Option<Vec<Message>> {
        // History is now stored on Agent/Thread level – nodes do not track it.
        None
    }

    fn _id(&self) -> String {
        self.node_id.clone()
    }

    fn _set_id(&mut self, id: String) {
        self.node_id = id;
    }

    fn status(&self) -> Option<String> {
        None
    }

    fn get_status_from_agents(&self, agents: &HashMap<u32, ApiAgent>) -> Option<String> {
        self.agent_id
            .and_then(|id| agents.get(&id))
            .and_then(|agent| agent.status.clone())
    }

    fn set_system_instructions(&mut self, _instructions: Option<String>) {
        // Intentionally left empty – nodes no longer own agent instructions.
    }

    fn set_status(&mut self, _status: Option<String>) {
        // Intentionally left empty.
    }

    fn system_instructions(&self) -> Option<String> {
        None
    }

    fn task_instructions(&self) -> Option<String> {
        None
    }

    fn _set_task_instructions(&mut self, _instructions: Option<String>) {}

    fn _set_history(&mut self, _history: Option<Vec<Message>>) {}
}
