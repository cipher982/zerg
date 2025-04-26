// Default values for agent configuration - these are the single source of truth for defaults
pub const DEFAULT_AGENT_NAME: &str = "New Agent";
pub const DEFAULT_SYSTEM_INSTRUCTIONS: &str = "You are a helpful AI assistant.";
pub const DEFAULT_TASK_INSTRUCTIONS: &str = "These are the default task instructions. Begin your job now based on prompt.";
pub const DEFAULT_TEMPERATURE: f64 = 0.3;
pub const DEFAULT_THREAD_TITLE: &str = "New Thread";

// Node visual defaults
pub const DEFAULT_NODE_WIDTH: f64 = 200.0;
pub const DEFAULT_NODE_HEIGHT: f64 = 80.0;
pub const DEFAULT_AGENT_NODE_COLOR: &str = "#ffecb3"; // Light amber color 

// --- Canvas & Node Colors ---

// Background
pub const CANVAS_BACKGROUND_COLOR: &str = "#2a2a3a"; // Match dashboard background color

// Node Fill Colors
pub const NODE_COLOR_USER_INPUT: &str = "#3498db";    // Blue
pub const NODE_COLOR_RESPONSE: &str = "#d5f5e3";      // Light green
pub const NODE_COLOR_GENERIC: &str = "#95a5a6";       // Gray
pub const NODE_FILL_AGENT_IDENTITY: &str = "rgba(255, 255, 255, 0.1)"; // Semi-transparent white

// Node Borders & Strokes
pub const NODE_BORDER_DEFAULT: &str = "#000000";      // Black
pub const NODE_BORDER_SELECTED: &str = "#3498db";     // Blue highlight
pub const NODE_BORDER_AGENT_IDLE: &str = "rgba(149, 165, 166, 0.8)"; // Gray
pub const NODE_BORDER_AGENT_PROCESSING_BASE: &str = "46, 204, 113"; // RGB for Green (alpha applied dynamically)
pub const NODE_BORDER_AGENT_ERROR: &str = "rgba(231, 76, 60, 0.8)";   // Red
pub const NODE_BORDER_AGENT_SCHEDULED: &str = "rgba(52, 152, 219, 0.8)"; // Blue
pub const NODE_BORDER_AGENT_PAUSED: &str = "rgba(243, 156, 18, 0.8)";  // Orange

// Other Elements
pub const NODE_TEXT_COLOR: &str = "#DDDDDD";         // Light gray for text on dark background
pub const CONNECTION_LINE_COLOR: &str = "#95a5a6";    // Gray for connection lines/arrows
pub const SHADOW_COLOR: &str = "rgba(0, 0, 0, 0.2)"; // Shadow for nodes 