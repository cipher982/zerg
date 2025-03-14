// Default values for agent configuration - these are the single source of truth for defaults
pub const DEFAULT_AGENT_NAME: &str = "New Agent";
pub const DEFAULT_SYSTEM_INSTRUCTIONS: &str = "You are a helpful AI assistant.";
pub const DEFAULT_TASK_INSTRUCTIONS: &str = "Respond to user questions accurately and concisely.";
pub const DEFAULT_MODEL: &str = "gpt-3.5-turbo";
pub const DEFAULT_TEMPERATURE: f64 = 0.7;

// Node visual defaults
pub const DEFAULT_NODE_WIDTH: f64 = 200.0;
pub const DEFAULT_NODE_HEIGHT: f64 = 80.0;
pub const DEFAULT_AGENT_NODE_COLOR: &str = "#ffecb3"; // Light amber color 