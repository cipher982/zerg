// Generated types module

pub mod tool_definitions;
pub mod workflow;

pub use tool_definitions::{ServerName, ToolName};
pub use workflow::{NodeType, WorkflowCanvas, WorkflowEdge, WorkflowNode};

// Type alias for compatibility
pub type WorkflowNodeType = NodeType;
