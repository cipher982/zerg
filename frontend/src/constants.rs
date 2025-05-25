//! Constants for the frontend application
//! 
//! This module centralizes commonly used string literals to prevent typos
//! and enable safe refactoring across the codebase.

// CSS Class Names
pub const CSS_TAB_BUTTON: &str = "tab-button";
pub const CSS_TAB_BUTTON_ACTIVE: &str = "tab-button active";
pub const CSS_MODAL: &str = "modal";
pub const CSS_HIDDEN: &str = "hidden";
pub const CSS_VISIBLE: &str = "visible";
pub const CSS_MESSAGE_USER: &str = "message user-message";
pub const CSS_MESSAGE_ASSISTANT: &str = "message assistant-message";

// Utility CSS Classes (extracted from inline styles)
pub const CSS_FORM_ROW: &str = "form-row";
pub const CSS_FORM_ROW_SM: &str = "form-row-sm";
pub const CSS_FORM_ROW_XS: &str = "form-row-xs";
pub const CSS_CARD: &str = "card";
pub const CSS_ACTIONS_ROW: &str = "actions-row";
pub const CSS_SUCCESS_TEXT: &str = "success-text";
pub const CSS_SCHEDULE_SUMMARY: &str = "schedule-summary";
pub const CSS_TRIGGERS_LIST: &str = "triggers-list";
pub const CSS_EMPTY_STATE: &str = "empty-state";
pub const CSS_PRESERVE_WHITESPACE: &str = "preserve-whitespace";

// Message Roles
pub const ROLE_USER: &str = "user";
pub const ROLE_ASSISTANT: &str = "assistant";
pub const ROLE_TOOL: &str = "tool";
pub const ROLE_SYSTEM: &str = "system";

// Agent/Run Status Values
pub const STATUS_RUNNING: &str = "running";
pub const STATUS_IDLE: &str = "idle";
pub const STATUS_ERROR: &str = "error";
pub const STATUS_SCHEDULED: &str = "scheduled";
pub const STATUS_PROCESSING: &str = "processing";
pub const STATUS_COMPLETE: &str = "complete";
pub const STATUS_SUCCESS: &str = "success";

// Element IDs (commonly referenced)
pub const ID_GLOBAL_CANVAS_TAB: &str = "global-canvas-tab";
pub const ID_GLOBAL_DASHBOARD_TAB: &str = "global-dashboard-tab";
pub const ID_GLOBAL_LOGIN_OVERLAY: &str = "global-login-overlay";
pub const ID_AGENT_TRIGGERS_TAB: &str = "agent-triggers-tab";

// WebSocket/Network Message Types
pub const WS_TYPE_ERROR: &str = "error";
pub const WS_TYPE_PONG: &str = "pong";
pub const WS_TYPE_DISCONNECT: &str = "disconnect";
pub const WS_TYPE_PING: &str = "ping";

// Event Types
pub const EVENT_SYSTEM_STATUS: &str = "system_status";
pub const EVENT_THREAD_MESSAGE_CREATED: &str = "thread_message_created";

// Default Values
pub const DEFAULT_TOOL_NAME: &str = "tool";
pub const DEFAULT_AGENT_COLOR: &str = "#ffecb3"; // Light amber
pub const DEFAULT_AGENT_NODE_COLOR: &str = "#2ecc71"; // Green for agent identity nodes
pub const DEFAULT_AGENT_NAME: &str = "New Agent";
pub const DEFAULT_SYSTEM_INSTRUCTIONS: &str = "You are a helpful AI assistant.";
pub const DEFAULT_TASK_INSTRUCTIONS: &str = "Complete the given task.";
pub const DEFAULT_THREAD_TITLE: &str = "New Thread";

// Node dimensions
pub const DEFAULT_NODE_WIDTH: f64 = 200.0;
pub const DEFAULT_NODE_HEIGHT: f64 = 100.0;

// Status Colors (for canvas nodes)
pub const COLOR_IDLE: &str = "#ffecb3";      // Light amber
pub const COLOR_PROCESSING: &str = "#b3e5fc"; // Light blue
pub const COLOR_COMPLETE: &str = "#c8e6c9";   // Light green
pub const COLOR_ERROR: &str = "#ffcdd2";      // Light red

// Canvas colors
pub const CANVAS_BACKGROUND_COLOR: &str = "#f5f5f5";
pub const NODE_COLOR_GENERIC: &str = "#e0e0e0";
pub const SHADOW_COLOR: &str = "rgba(0, 0, 0, 0.2)";
pub const NODE_TEXT_COLOR: &str = "#333333";
pub const CONNECTION_LINE_COLOR: &str = "#666666";

// Node border colors
pub const NODE_BORDER_DEFAULT: &str = "#cccccc";
pub const NODE_BORDER_SELECTED: &str = "#2196f3";
pub const NODE_BORDER_AGENT_IDLE: &str = "#ffc107";
pub const NODE_BORDER_AGENT_PROCESSING_BASE: &str = "#2196f3";
pub const NODE_BORDER_AGENT_ERROR: &str = "#f44336";
pub const NODE_BORDER_AGENT_SCHEDULED: &str = "#9c27b0";
pub const NODE_BORDER_AGENT_PAUSED: &str = "#ff9800";

// Node fill colors
pub const NODE_FILL_AGENT_IDENTITY: &str = "#e3f2fd";

// Button Types
pub const BUTTON_TYPE_BUTTON: &str = "button";
pub const BUTTON_TYPE_SUBMIT: &str = "submit";

// HTML Attributes
pub const ATTR_TYPE: &str = "type";
pub const ATTR_DATA_TOOL_CALL_ID: &str = "data-tool-call-id";
pub const ATTR_DATA_TESTID: &str = "data-testid";
