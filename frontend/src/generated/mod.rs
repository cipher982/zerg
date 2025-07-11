// Generated types module

pub mod tool_definitions;
pub mod workflow;
pub mod ws_messages;

pub use tool_definitions::{ServerName, ToolName};
pub use workflow::{NodeType, WorkflowCanvas, WorkflowEdge, WorkflowNode};
pub use ws_messages::{Envelope, WsMessage};

// Type alias for compatibility
pub type WorkflowNodeType = NodeType;
