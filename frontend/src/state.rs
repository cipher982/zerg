use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, WebSocket};
use crate::models::{
    Node,
    NodeType,
    Workflow,
    Edge,
    ApiAgent,
    ApiThread,
    ApiThreadMessage,
    Trigger,
};
use crate::models::ApiAgentRun;

use crate::models::ApiAgentDetails;
use crate::canvas::{renderer, background::ParticleSystem};
use crate::storage::ActiveView;
use crate::network::{WsClientV2, TopicManager};
use crate::messages::{Message, Command};
use crate::constants::{
    DEFAULT_NODE_WIDTH,
    DEFAULT_NODE_HEIGHT,
    DEFAULT_AGENT_NODE_COLOR,
};
use js_sys::Date;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use crate::update;

// ---------------------------------------------------------------------------
// Viewport constraints – keep values sane so we never generate Inf/NaN or
// absurd world-space coordinates.
// Default zoom == 1.0 so we allow ±50 %.
// ---------------------------------------------------------------------------

// Zoom is currently disabled – hard-lock to 100 %.
pub const MIN_ZOOM: f64 = 1.0;
pub const MAX_ZOOM: f64 = 1.0;
// Bring legacy helper trait into scope (methods formerly on CanvasNode)

// ---------------------------------------------------------------------------
// Agent Debug Pane (read-only modal) – Phase 1
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugTab {
    Overview,
    RawJson,
}

// ---------------------------------------------------------------------------
// Agent Configuration Modal – currently three fixed tabs.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentConfigTab {
    Main,
    #[allow(dead_code)] // planned UI tab – keep placeholder
    History,
    Triggers,
    ToolsIntegrations,
}

// ---------------------------------------------------------------------------
// Dashboard filter scope – either *my* (default) or *all* (admin)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardScope {
    MyAgents,
    AllAgents,
}

impl DashboardScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            DashboardScope::MyAgents => "my",
            DashboardScope::AllAgents => "all",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "all" => DashboardScope::AllAgents,
            _ => DashboardScope::MyAgents,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentDebugPane {
    pub agent_id: u32,
    pub loading: bool,
    pub details: Option<ApiAgentDetails>,
    pub active_tab: DebugTab,
}

/// UI state for each tool call indicator (collapsed/expanded and show-full settings)
#[derive(Debug, Clone)]
pub struct ToolUiState {
    /// Whether the tool details panel is expanded
    pub expanded: bool,
    /// Whether the full tool output is shown (vs truncated)
    pub show_full: bool,
}

// ---------------------------------------------------------------------------
// MCP Integration State Structures
// ---------------------------------------------------------------------------

/// Configuration for MCP servers connected to an agent
#[derive(Debug, Clone)]
pub struct AgentMcpConfig {
    /// List of MCP servers configured for this agent
    pub servers: Vec<McpServerConfig>,
    /// Set of tool names that the agent is allowed to use
    pub allowed_tools: HashSet<String>,
}

use serde::Serialize;

/// Individual MCP server configuration
#[derive(Debug, Clone, Serialize)]
pub struct McpServerConfig {
    /// Display name for the server
    pub name: String,
    /// URL for custom MCP servers (None for presets)
    pub url: Option<String>,
    /// Preset name (e.g., "github", "linear") if using a preset
    pub preset: Option<String>,
    /// Authentication token (encrypted on backend)
    pub auth_token: Option<String>,
}

/// Information about a tool provided by an MCP server
#[derive(Debug, Clone)]
pub struct McpToolInfo {
    /// Tool name (e.g., "create_issue")
    pub name: String,
    /// Server that provides this tool
    pub server_name: String,
    /// Human-readable description
    pub description: Option<String>,
}

/// Connection status for an MCP server
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    /// Server is healthy and responding
    Healthy,
    /// Server is responding but slowly (with response time in ms)
    Slow(u64),
    /// Server connection failed with error message
    Failed(String),
    /// Currently checking connection status
    Checking,
}

    // Store global application state
pub struct AppState {
    /// If true, global keyboard shortcuts (power mode) are enabled
    pub power_mode: bool,
    // Particle system for animated background
    pub particle_system: Option<ParticleSystem>,
    // Agent domain data (business logic)
    pub agents: HashMap<u32, ApiAgent>,        // Backend agent data
    pub agents_on_canvas: HashSet<u32>,        // Track which agents are already placed on canvas
    
    // Canvas visualization data
    pub nodes: HashMap<String, Node>,          // Visual layout nodes
    pub workflows: HashMap<u32, Workflow>,     // Workflows collection
    pub current_workflow_id: Option<u32>,      // Currently active workflow
    
    // Canvas and rendering related
    pub canvas: Option<HtmlCanvasElement>,
    pub context: Option<CanvasRenderingContext2d>,
    pub connection_animation_offset: f64,
    pub input_text: String,
    pub dragging: Option<String>,
    pub drag_offset_x: f64,
    pub drag_offset_y: f64,
    // New fields for canvas dragging
    pub canvas_dragging: bool,
    pub drag_start_x: f64,
    pub drag_start_y: f64,
    pub drag_last_x: f64,
    pub drag_last_y: f64,
    #[allow(dead_code)]
    pub websocket: Option<WebSocket>,
    // Canvas dimensions
    pub canvas_width: f64,
    pub canvas_height: f64,
    // Viewport tracking for zoom-to-fit functionality
    pub viewport_x: f64,
    pub viewport_y: f64,
    pub zoom_level: f64,
    pub auto_fit: bool,
    // Track the latest user input node ID
    pub latest_user_input_id: Option<String>,
    // Track message IDs and their corresponding node IDs
    pub message_id_to_node_id: HashMap<String, String>,
    // Selected AI model
    pub selected_model: String,
    // Available AI models
    pub available_models: Vec<(String, String)>,
    // Default AI model ID
    pub default_model_id: String,
    // Whether state has been modified since last save
    pub state_modified: bool,

    /// Timestamp (ms) of the **last modification** that set `state_modified`
    /// to true.  Used by the debounced persistence logic inside the
    /// AnimationTick handler.
    pub last_modified_ms: u64,
    // Currently selected node ID
    pub selected_node_id: Option<String>,
    // Track node ID on mousedown to detect clicks on mouseup
    pub clicked_node_id: Option<String>,
    // Flag to track if we're dragging an agent
    pub is_dragging_agent: bool,
    // Track the active view (Dashboard, Canvas, or ChatView)
    pub active_view: ActiveView,

    /// Current filter applied to the dashboard (persisted in localStorage).
    pub dashboard_scope: DashboardScope,


    /// Current sort settings for the dashboard table
    pub dashboard_sort: DashboardSort,
    // Pending network call data to avoid nested borrows
    pub pending_network_call: Option<(String, String)>,
    // Loading state flags
    pub is_loading: bool,
    pub data_loaded: bool,
    pub api_load_attempted: bool,
    // Chat/Thread related state
    pub current_thread_id: Option<u32>,
    pub threads: HashMap<u32, ApiThread>,
    pub thread_messages: HashMap<u32, Vec<ApiThreadMessage>>,
    pub is_chat_loading: bool,
    // New field for handling streaming responses
    /// Tracks the **current assistant message** id for every thread that is
    /// actively streaming.  `None` means we have not yet received the
    /// `assistant_id` frame (token-stream mode) or the first
    /// `assistant_message` chunk (non token mode).  Once the id is known we
    /// replace the entry with `Some(id)` so tool_output bubbles can link to
    /// their parent.
    pub active_streams: HashMap<u32, Option<u32>>,
    // --- WebSocket v2 and Topic Manager --- 
    pub ws_client: Rc<RefCell<WsClientV2>>,
    pub topic_manager: Rc<RefCell<TopicManager>>,
    pub streaming_threads: HashSet<u32>,

    /// Threads for which the server is sending *token level* chunks.  This is
    /// detected lazily when the first `assistant_token` chunk is observed so
    /// we can adapt placeholder/bubble logic accordingly.
    pub token_mode_threads: HashSet<u32>,
    pub current_agent_id: Option<u32>,

    /// Currently selected tab inside the *Agent Configuration* modal.  Kept
    /// here so business logic can switch tabs without directly touching the
    /// DOM (single-source-of-truth).  Defaults to `Main` when the modal is
    /// first opened.
    pub agent_modal_tab: AgentConfigTab,

    // Track which agent rows are expanded in the dashboard UI so we can
    // preserve open/closed state across re‑renders.
    pub expanded_agent_rows: HashSet<u32>,

    // Map `agent_id -> recent runs (ordered newest-first, max 20)`
    pub agent_runs: HashMap<u32, Vec<ApiAgentRun>>, 

    // ---------------------------------------------------------------
    // Trigger management (NEW – Phase A)
    // ---------------------------------------------------------------
    /// All triggers grouped by their owning agent.  Populated lazily when a
    /// user opens the *Triggers* tab in the agent modal.
    pub triggers: HashMap<u32, Vec<Trigger>>, // agent_id → triggers

    // Agent Debug modal (None when hidden)
    pub agent_debug_pane: Option<AgentDebugPane>,
    // UI state for collapsible tool call indicators
    pub tool_ui_states: HashMap<String, ToolUiState>,

    // -------------------------------------------------------------------
    // Runtime configuration flags fetched from `/api/system/info`
    // -------------------------------------------------------------------

    /// Google OAuth client ID returned by the backend.  *None* until the
    /// initial system-info request completes.  Stored so that subsequent
    /// calls (e.g. after a manual logout) can recreate the login overlay
    /// without requiring another network round-trip.
    pub google_client_id: Option<String>,

    // Track which agents have their full run history expanded (>5 rows)
    pub run_history_expanded: HashSet<u32>,

    // -------------------------------------------------------------------
    // Gmail integration status (Phase C)
    // -------------------------------------------------------------------

    /// True once the user connected Gmail via OAuth.  Controls whether the
    /// “Email (Gmail)” option in the *Add Trigger* wizard is clickable.
    pub gmail_connected: bool,

    // ---------------------------------------------------------------
    // Debug overlay (only compiled in debug builds)
    // ---------------------------------------------------------------
    #[cfg(debug_assertions)]
    pub debug_ring: VecDeque<String>,

    /// Runs currently in `running` status for which the dashboard should show
    /// a ticking duration value.  Stores *run_id* (primary key) – agent_id can
    /// be looked up via `agent_runs` map if required.
    pub running_runs: HashSet<u32>,

    // -------------------------------------------------------------------
    // NEW: Explicit mapping from backend `agent_id` → canvas `node_id`.
    // -------------------------------------------------------------------
    /// Once an AgentIdentity node is placed on the canvas we store a *single*
    /// mapping entry so the UI can instantly look up the visual node given an
    /// `agent_id` without resorting to brittle string parsing or HashMap
    /// scans.  The invariant we uphold is **at most one** AgentIdentity node
    /// per agent per-workflow.  When a node is deleted (feature TBD) the
    /// entry must be removed as well.
    pub agent_id_to_node_id: HashMap<u32, String>,

    // -------------------------------------------------------------------
    // Rendering: marks whether a repaint is required on the next
    // requestAnimationFrame.  Keep this separate from `state_modified`
    // which is about *persistence*, not visual output.
    pub dirty: bool,

    // -------------------------------------------------------------------
    // Authentication
    // -------------------------------------------------------------------
    /// Whether the current browser session is authenticated.  Determined at
    /// startup from `localStorage["zerg_jwt"]` and updated once the Google
    /// login flow succeeds.
    pub logged_in: bool,

    /// Authenticated user profile loaded from `/api/users/me`.  `None` until
    /// the profile fetch succeeds (or the session is unauthenticated).
    pub current_user: Option<crate::models::CurrentUser>,

    /// Track whether agents have been loaded from the API at least once
    pub agents_loaded: bool,

    // -------------------------------------------------------------------
    // MCP Integration State
    // -------------------------------------------------------------------
    /// MCP server configurations per agent
    pub agent_mcp_configs: HashMap<u32, AgentMcpConfig>,
    
    /// Available MCP tools per agent (populated when tools are fetched)
    pub available_mcp_tools: HashMap<u32, Vec<McpToolInfo>>,
    
    /// Connection status for each MCP server (key: "agent_id:server_name")
    pub mcp_connection_status: HashMap<String, ConnectionStatus>,
    
}

// ---------------------------------------------------------------------------
// Dashboard sort state – column + ascending flag
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardSortKey {
    Name,
    Status,
    LastRun,
    NextRun,
    SuccessRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardSort {
    pub key: DashboardSortKey,
    pub ascending: bool,
}

impl AppState {
    pub fn new() -> Self {
        // --- Initialize WsClientV2 and TopicManager ---
        // Create the WebSocket client first
        let ws_client_rc = Rc::new(RefCell::new(WsClientV2::new_default()));
        // Create the TopicManager, giving it a reference to the client
        let topic_manager_rc = Rc::new(RefCell::new(TopicManager::new(ws_client_rc.clone())));

        Self {
            agents: HashMap::new(),
            agents_on_canvas: HashSet::new(),
            nodes: HashMap::new(),
            workflows: HashMap::new(),
            current_workflow_id: None,
            canvas: None,
            context: None,
            connection_animation_offset: 0.0,
            input_text: String::new(),
            dragging: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            canvas_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            drag_last_x: 0.0,
            drag_last_y: 0.0,
            websocket: None,
            canvas_width: 800.0,
            canvas_height: 600.0,
            viewport_x: 0.0,
            viewport_y: 0.0,
            zoom_level: 1.0,
            // Auto-fit disabled by default – users can still trigger a one-off
            // "Find everything" action via the toolbar button.
            auto_fit: false,
            latest_user_input_id: None,
            message_id_to_node_id: HashMap::new(),
            selected_model: String::new(),
            available_models: Vec::new(),
            default_model_id: String::new(),
            state_modified: false,

            last_modified_ms: 0,
            selected_node_id: None,
            clicked_node_id: None,
            is_dragging_agent: false,
            active_view: ActiveView::ChatView,

            dashboard_scope: {
                // Read persisted dashboard scope from localStorage (if any)
                let window = web_sys::window();
                if let Some(w) = window {
                    if let Ok(Some(storage)) = w.local_storage() {
                        if let Ok(Some(scope_str)) = storage.get_item("dashboard_scope") {
                            DashboardScope::from_str(&scope_str)
                        } else {
                            DashboardScope::MyAgents
                        }
                    } else {
                        DashboardScope::MyAgents
                    }
                } else {
                    DashboardScope::MyAgents
                }
            },

            // Default sort: Name ascending


            dashboard_sort: {
                // Try restoring from localStorage
                let window = web_sys::window();
                if let Some(w) = window {
                    if let Ok(Some(storage)) = w.local_storage() {
                        if let Ok(Some(key)) = storage.get_item("dashboard_sort_key") {
                            let key_enum = match key.as_str() {
                                "status" => DashboardSortKey::Status,
                                "last_run" => DashboardSortKey::LastRun,
                                "next_run" => DashboardSortKey::NextRun,
                                "success" => DashboardSortKey::SuccessRate,
                                _ => DashboardSortKey::Name,
                            };
                            let asc = storage.get_item("dashboard_sort_asc").ok().flatten().map(|v| v != "0").unwrap_or(true);
                            DashboardSort { key: key_enum, ascending: asc }
                        } else {
                            DashboardSort { key: DashboardSortKey::Name, ascending: true }
                        }
                    } else {
                        DashboardSort { key: DashboardSortKey::Name, ascending: true }
                    }
                } else {
                    DashboardSort { key: DashboardSortKey::Name, ascending: true }
                }
            },
            pending_network_call: None,
            is_loading: true,
            data_loaded: false,
            api_load_attempted: false,
            current_thread_id: None,
            threads: HashMap::new(),
            thread_messages: HashMap::new(),
            is_chat_loading: false,
            active_streams: HashMap::new(),
            ws_client: ws_client_rc,
            topic_manager: topic_manager_rc,
            streaming_threads: HashSet::new(),

            token_mode_threads: HashSet::new(),
            current_agent_id: None,

            agent_modal_tab: AgentConfigTab::Main,

            expanded_agent_rows: HashSet::new(),

            agent_runs: HashMap::new(),

            // Trigger map starts empty – filled on demand.
            triggers: HashMap::new(),

            agent_debug_pane: None,
            // Initialize UI state for tool call indicators
            tool_ui_states: HashMap::new(),

            run_history_expanded: HashSet::new(),

            // Gmail yet to be connected.
            gmail_connected: false,

            running_runs: HashSet::new(),

            agent_id_to_node_id: HashMap::new(),

            // The very first frame must draw the freshly created canvas.
            dirty: true,

            #[cfg(debug_assertions)]
            debug_ring: VecDeque::new(),

            // -------------------------------------------------------------------
            // Authentication state – look for a persisted JWT.
            // -------------------------------------------------------------------
            logged_in: {
                let window = web_sys::window();
                if let Some(w) = window {
                    if let Ok(Some(storage)) = w.local_storage() {
                        storage.get_item("zerg_jwt").ok().flatten().is_some()
                    } else {
                        false
                    }
                } else {
                    false
                }
            },

            // Initially no profile is loaded.  Will be set once the frontend
            // successfully calls `/api/users/me` after a login or page
            // refresh.
            current_user: None,

            google_client_id: None,
            
            // Start with agents not loaded
            agents_loaded: false,
            
            // MCP Integration state
            agent_mcp_configs: HashMap::new(),
            available_mcp_tools: HashMap::new(),
            mcp_connection_status: HashMap::new(),
            power_mode: false,
            particle_system: None,
        }
    }

    // -------------------------------------------------------------------
    // Rendering helpers
    // -------------------------------------------------------------------

    /// Mark the canvas as needing a repaint on the next animation frame.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Return the assistant message id that is **currently being streamed**
    /// for the given `thread_id`, if known.
    pub fn current_assistant_id(&self, thread_id: u32) -> Option<u32> {
        self.active_streams.get(&thread_id).and_then(|opt| *opt)
    }

    pub fn add_node(&mut self, text: String, x: f64, y: f64, node_type: NodeType) -> String {
        let id = format!("node_{}", self.nodes.len());
        web_sys::console::log_1(&format!("Creating node: id={}, type={:?}, text={}", id, node_type, text).into());
        
        // Determine color based on node type
        let color = match &node_type {
            NodeType::UserInput => "#3498db".to_string(),    // Blue
            NodeType::ResponseOutput => "#9b59b6".to_string(), // Purple
            NodeType::AgentIdentity => "#2ecc71".to_string(), // Green
            NodeType::GenericNode => "#95a5a6".to_string(),  // Gray
            NodeType::Tool { .. } => "#f59e0b".to_string(),  // Orange
            NodeType::Trigger { .. } => "#10b981".to_string(), // Green
        };
        
        // Calculate approximate node size based on text content
        let chars_per_line = 25; // Approximate chars per line
        let lines = (text.len() as f64 / chars_per_line as f64).ceil() as usize;
        
        // Set minimum sizes but allow for growth
        let width = f64::max(200.0, chars_per_line as f64 * 8.0); // Estimate width based on chars
        let height = f64::max(80.0, lines as f64 * 20.0 + 40.0);  // Base height + lines
        
        let node = Node {
            node_id: id.clone(),
            agent_id: None,
            x,
            y,
            text,
            width,
            height,
            color,
            parent_id: None, // Parent ID will be set separately if needed
            node_type: node_type.clone(),
            is_selected: false,
            is_dragging: false,
            exec_status: None,
        };
        
        web_sys::console::log_1(&format!("Node created with dimensions: {}x{} at position ({}, {})", 
            node.width, node.height, node.x, node.y).into());
        
        self.nodes.insert(id.clone(), node);
        self.state_modified = true; // Mark state as modified
        
        // If this is a user input node, update the latest_user_input_id
        if let NodeType::UserInput = &node_type {
            self.latest_user_input_id = Some(id.clone());
        }
        
        // Auto-fit all nodes if enabled
        if self.auto_fit && self.nodes.len() > 1 {
            web_sys::console::log_1(&JsValue::from_str("Auto-fitting nodes to view"));
            self.fit_nodes_to_view();
        }
        
        web_sys::console::log_1(&format!("Successfully added node {}", id).into());
        id
    }

    // -------------------------------------------------------------------
    // NODE CREATION HELPERS – AGENT-AWARE
    // -------------------------------------------------------------------
    /// Add a `NodeType::AgentIdentity` node that visualises the given
    /// `agent_id`.  The function ensures the `agent_id_to_node_id` mapping is
    /// updated so other parts of the frontend can perform O(1) look-ups.
    pub fn add_agent_node(&mut self, agent_id: u32, text: String, x: f64, y: f64) -> String {
        // Delegates to the generic `add_node()` but afterwards injects the
        // agent-specific metadata and mapping.

        let node_id = self.add_node(text, x, y, NodeType::AgentIdentity);

        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.agent_id = Some(agent_id);
        }

        self.agent_id_to_node_id.insert(agent_id, node_id.clone());

        node_id
    }

    /// Remove a node from the canvas and clean up any reverse mapping if the
    /// node belonged to an agent.
    #[allow(dead_code)]
    pub fn remove_node(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.remove(node_id) {
            if let Some(agent_id) = node.agent_id {
                self.agent_id_to_node_id.remove(&agent_id);
                self.agents_on_canvas.remove(&agent_id);
            }
            self.state_modified = true;
            self.mark_dirty();
        }
    }

    pub fn add_response_node(&mut self, parent_id: &str, response_text: String) -> String {
        let response_id = format!("resp-{}", self.generate_message_id());
        let parent = self.nodes.get(parent_id);
        
        // Default position for response node is below parent
        let (mut x, mut y) = (100.0, 100.0);
        
        if let Some(parent_node) = parent {
            x = parent_node.x;
            y = parent_node.y + parent_node.height + 30.0;
        }
        
        let node = Node {
            node_id: response_id.clone(),
            agent_id: None,
            x,
            y,
            width: 300.0,
            height: 100.0,
            color: "#d5f5e3".to_string(),  // Light green
            text: response_text.clone(),
            node_type: NodeType::ResponseOutput,
            parent_id: Some(parent_id.to_string()),
            exec_status: None,
            is_selected: false,
            is_dragging: false,
        };
        
        self.nodes.insert(response_id.clone(), node);
        self.state_modified = true;
        
        // If the parent is an agent node, add this message to its history
        if let Some(parent_node) = self.nodes.get_mut(parent_id) {
            if let crate::models::NodeType::AgentIdentity = parent_node.node_type {
                let message = crate::models::Message {
                    role: "assistant".to_string(),
                    content: response_text,
                    timestamp: js_sys::Date::now() as u64,
                };
                
                // Instead of directly accessing history, use agent_id to add the message
                if let Some(agent_id) = parent_node.agent_id {
                    crate::state::APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        if let Some(_agent) = state.agents.get_mut(&agent_id) {
                            // Store the message with the agent (actual implementation would depend on your API structure)
                        }
                    });
                }
                
                // Still save the message to API
                crate::storage::save_agent_messages_to_api(parent_id, &[message]);
            }
        }
        
        response_id
    }
    
    pub fn draw_nodes(&self) {
        if let (Some(context), Some(canvas)) = (&self.context, &self.canvas) {
            // Clear the canvas with our themed background color
            context.set_fill_style_str(crate::constants::CANVAS_BACKGROUND_COLOR);
            context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            
            // Apply transformations for viewport
            context.save();
            context.translate(-self.viewport_x, -self.viewport_y).unwrap();
            context.scale(self.zoom_level, self.zoom_level).unwrap();
            
            // Draw canvas nodes (new structure)
            for (_, node) in &self.nodes {
                renderer::draw_node(&context, node, &self.agents);
            }
            
            // Draw legacy nodes (for backward compatibility - will be removed later)
            for (_, node) in &self.nodes {
                renderer::draw_node(&context, node, &self.agents);
            }
            
            // Restore the canvas context to its original state
            context.restore();
        }
    }
    
    pub fn update_node_position(&mut self, node_id: &str, x: f64, y: f64) {
        // Track if any updates were made
        let mut updated = false;
        
        // First, try to update in nodes (new structure)
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.x = x;
            node.y = y;
            updated = true;
        }
        
        // Only proceed if an update was made
        if updated {
            self.state_modified = true; // Mark state as modified
            
            // Auto-fit all nodes if enabled
            if self.auto_fit {
                self.fit_nodes_to_view();
            } else {
                self.mark_dirty();
            }
        }
    }
    
    pub fn find_node_at_position(&self, x: f64, y: f64) -> Option<(String, f64, f64)> {
        // Apply viewport transformation to the coordinates
        let adjusted_x = x / self.zoom_level + self.viewport_x;
        let adjusted_y = y / self.zoom_level + self.viewport_y;
        
        // First, check in nodes (new structure)
        for (id, node) in &self.nodes {
            if adjusted_x >= node.x && 
               adjusted_x <= node.x + node.width &&
               adjusted_y >= node.y && 
               adjusted_y <= node.y + node.height {
                return Some((id.clone(), adjusted_x - node.x, adjusted_y - node.y));
            }
        }
        
        None
    }
    
    // Apply transform to ensure all nodes are visible
    pub fn fit_nodes_to_view(&mut self) {
        // If there are no nodes at all, nothing to fit
        if self.nodes.is_empty() {
            return;
        }
        
        // Find bounding box of all nodes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        
        // Check nodes (new structure)
        for (_, node) in &self.nodes {
            min_x = f64::min(min_x, node.x);
            min_y = f64::min(min_y, node.y);
            max_x = f64::max(max_x, node.x + node.width);
            max_y = f64::max(max_y, node.y + node.height);
        }
        
        // Calculate the bounding box dimensions
        let box_width = max_x - min_x;
        let box_height = max_y - min_y;
        
        // Ensure canvas exists and has valid dimensions
        let (canvas_width, canvas_height) = match &self.canvas {
            Some(canvas) => {
                let dpr = web_sys::window().unwrap().device_pixel_ratio();
                let w = canvas.width() as f64 / dpr;
                let h = canvas.height() as f64 / dpr;
                if w < 1.0 || h < 1.0 {
                    web_sys::console::error_1(&"fit_nodes_to_view: canvas not yet sized".into());
                    return;
                }
                (w, h)
            }
            None => {
                web_sys::console::error_1(&"fit_nodes_to_view: no canvas available".into());
                return;
            }
        };
        
        // Calculate zoom level to fit all nodes with padding
        let padding = 50.0; // Padding around the bounding box
        let _zoom_x = canvas_width / (box_width + padding * 2.0);
        let _zoom_y = canvas_height / (box_height + padding * 2.0);
        
        // Take the smaller of the two to ensure all nodes fit
        if let Some(canvas) = &self.canvas {
            // Find bounding box of all nodes
            let mut min_x = f64::MAX;
            let mut min_y = f64::MAX;
            let mut max_x = f64::MIN;
            let mut max_y = f64::MIN;
            
            for (_, node) in &self.nodes {
                min_x = f64::min(min_x, node.x);
                min_y = f64::min(min_y, node.y);
                max_x = f64::max(max_x, node.x + node.width);
                max_y = f64::max(max_y, node.y + node.height);
            }
            
            // Get canvas dimensions
            let canvas_width = canvas.width() as f64;
            let canvas_height = canvas.height() as f64;
            
            // Get the device pixel ratio to adjust calculations
            let window = web_sys::window().expect("no global window exists");
            let dpr = window.device_pixel_ratio();
            
            // Adjust canvas dimensions by DPR
            let canvas_width = canvas_width / dpr;
            let canvas_height = canvas_height / dpr;
            
            // Calculate required width and height with padding
            let padding = 80.0;
            let required_width = max_x - min_x + padding; 
            let required_height = max_y - min_y + padding;
            
            // Set minimum view area to prevent excessive zooming on small node counts
            // This ensures we don't zoom in too far when there are only a few nodes
            let min_view_width = 800.0;  // Minimum width to display
            let min_view_height = 600.0; // Minimum height to display
            
            // Use the larger of required size or minimum size
            let effective_width = f64::max(required_width, min_view_width);
            let effective_height = f64::max(required_height, min_view_height);
            
            // Calculate zoom level needed
            let width_ratio = canvas_width / effective_width;
            let height_ratio = canvas_height / effective_height;
            
            // Use the smaller ratio to ensure everything fits, but clamp to a
            // sensible range so we don't zoom to nearly-zero which would make
            // subsequent coordinate conversions explode.
            let mut new_zoom = f64::min(width_ratio, height_ratio);
            if new_zoom > MAX_ZOOM {
                new_zoom = MAX_ZOOM;
            } else if new_zoom < MIN_ZOOM {
                new_zoom = MIN_ZOOM;
            }
            
            // Calculate the center of the nodes
            let center_x = min_x + (max_x - min_x) / 2.0;
            let center_y = min_y + (max_y - min_y) / 2.0;
            
            // Calculate viewport position to center the content
            let new_viewport_x = center_x - (canvas_width / (2.0 * new_zoom));
            let new_viewport_y = center_y - (canvas_height / (2.0 * new_zoom));
            
            // Update state
            self.zoom_level = new_zoom;
            self.clamp_zoom();
            self.viewport_x = new_viewport_x;
            self.viewport_y = new_viewport_y;
            self.clamp_viewport();
            
            // Schedule a redraw via RAF
            self.mark_dirty();
        }
    }
    
    // Toggle auto-fit functionality
    pub fn toggle_auto_fit(&mut self) {
        self.auto_fit = !self.auto_fit;
        if self.auto_fit {
            self.fit_nodes_to_view();
        }
    }

    // Center the viewport on all nodes without changing auto-fit setting
    pub fn center_view(&mut self) {
        // Calculate bounding box of all nodes first. If there are none, bail.
        if self.nodes.is_empty() {
            return;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for (_, node) in &self.nodes {
            min_x = f64::min(min_x, node.x);
            min_y = f64::min(min_y, node.y);
            max_x = f64::max(max_x, node.x + node.width);
            max_y = f64::max(max_y, node.y + node.height);
        }

        // Canvas dimensions must be ready; otherwise bail loudly.
        let (canvas_w, canvas_h) = match &self.canvas {
            Some(canvas) => {
                let dpr = web_sys::window().unwrap().device_pixel_ratio();
                let w = canvas.width() as f64 / dpr;
                let h = canvas.height() as f64 / dpr;
                if w < 1.0 || h < 1.0 {
                    web_sys::console::error_1(&"center_view: canvas not yet sized".into());
                    return;
                }
                (w, h)
            }
            None => {
                web_sys::console::error_1(&"center_view: no canvas available".into());
                return;
            }
        };

        let padding = 80.0;
        let required_w = (max_x - min_x) + padding;
        let required_h = (max_y - min_y) + padding;

        let width_ratio = canvas_w / required_w;
        let height_ratio = canvas_h / required_h;
        let mut target_zoom = f64::min(width_ratio, height_ratio);

        // Clamp zoom to a sensible range so that extremely small bounding
        // boxes (or an un-initialised <canvas>) cannot drive the zoom towards
        // zero which then explodes world-space coordinates on the next drag.
        if target_zoom > MAX_ZOOM {
            target_zoom = MAX_ZOOM;
        } else if target_zoom < MIN_ZOOM {
            target_zoom = MIN_ZOOM;
        }

        // Desired centre point of nodes
        let centre_x = min_x + (max_x - min_x) / 2.0;
        let centre_y = min_y + (max_y - min_y) / 2.0;

        let target_viewport_x = centre_x - (canvas_w / (2.0 * target_zoom));
        let target_viewport_y = centre_y - (canvas_h / (2.0 * target_zoom));

        // ------------------------------------------------------------------
        // Apply viewport change immediately.
        // ------------------------------------------------------------------
        // The earlier animated version left `zoom_level` unchanged until the
        // next RAF callback, which meant any user interaction that happened
        // in that single frame (e.g. dropping a new node) still used the old
        // (possibly tiny) zoom value and generated astronomically large
        // coordinates.  We switch to an immediate update – the user barely
        // notices the 250 ms animation anyway, but the correctness win is
        // massive.

        self.zoom_level  = target_zoom;
        self.clamp_zoom();
        self.viewport_x  = target_viewport_x;
        self.viewport_y  = target_viewport_y;

        self.mark_dirty();
    }

    /// Reset viewport to origin & 100% zoom with animation
    pub fn reset_view(&mut self) {
        // Immediate reset – avoid stale zoom during animation window.
        self.zoom_level = 1.0;
        self.clamp_zoom();
        self.viewport_x = 0.0;
        self.viewport_y = 0.0;
        self.mark_dirty();
    }
    
    #[allow(dead_code)]
    fn animate_viewport(
        &mut self,
        start_x: f64, start_y: f64, start_zoom: f64,
        target_x: f64, target_y: f64, target_zoom: f64
    ) {
        let window = web_sys::window().expect("no global window exists");
        
        // Animation parameters
        let duration = 250.0; // Animation duration in ms (fast but visible)
        let start_time = js_sys::Date::now();
        
        // Define the type for our self-referential closure
        type AnimationClosure = Closure<dyn FnMut(f64)>;
        
        // Need to use this approach for self-referential closure
        let f = Rc::new(RefCell::new(None::<AnimationClosure>));
        let g = f.clone();
        
        // Clone window for use outside the closure
        let window_for_start = window.clone();
        
        // Create a function to perform the animation
        *g.borrow_mut() = Some(Closure::new(Box::new(move |time: f64| {
            // Clone window reference for use in the closure
            let window_ref = web_sys::window().expect("no global window exists");
            
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let elapsed = time - start_time;
                let progress = (elapsed / duration).min(1.0);
                
                // Ease function (smooth start and end)
                let ease = |t: f64| -> f64 { 
                    if t < 0.5 {
                        4.0 * t * t * t
                    } else {
                        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                    }
                };
                
                let eased_progress = ease(progress);
                
                // Interpolate viewport position and zoom
                state.viewport_x = start_x + (target_x - start_x) * eased_progress;
                state.viewport_y = start_y + (target_y - start_y) * eased_progress;
                state.zoom_level = start_zoom + (target_zoom - start_zoom) * eased_progress;
                
                // Flag a repaint for this frame
                state.mark_dirty();
                
                // Continue animation if not finished
                if progress < 1.0 {
                    let _ = window_ref.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref());
                }
            });
        })));
        
        // Start the animation
        let _ = window_for_start.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref());
    }

    // Generate a unique message ID
    pub fn generate_message_id(&self) -> String {
        format!("msg_{}", Date::now())
    }
    
    // Track a message ID and its corresponding node ID
    pub fn track_message(&mut self, message_id: String, node_id: String) {
        self.message_id_to_node_id.insert(message_id, node_id);
    }
    
    // Function to get node ID for a message ID
    pub fn _get_node_id_for_message(&self, message_id: &str) -> Option<String> {
        self.message_id_to_node_id.get(message_id).cloned()
    }
    
    // Enforce boundaries for viewport to prevent going out of range
    pub fn _enforce_viewport_boundaries(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        
        // Find bounding box of all nodes
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        
        for (_, node) in &self.nodes {
            min_x = f64::min(min_x, node.x);
            min_y = f64::min(min_y, node.y);
            max_x = f64::max(max_x, node.x + node.width);
            max_y = f64::max(max_y, node.y + node.height);
        }
        
        // Calculate content dimensions
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        
        // Get canvas dimensions if available
        let (canvas_width, canvas_height) = if let Some(canvas) = &self.canvas {
            let window = web_sys::window().expect("no global window exists");
            let dpr = window.device_pixel_ratio();
            let canvas_width = canvas.width() as f64 / dpr;
            let canvas_height = canvas.height() as f64 / dpr;
            (canvas_width, canvas_height)
        } else {
            (800.0, 600.0) // Fallback values if canvas not available
        };
        
        // Calculate the viewport's visible width and height in world coordinates
        let viewport_width = canvas_width / self.zoom_level;
        let viewport_height = canvas_height / self.zoom_level;
        
        // Calculate expanded content bounds with generous padding
        // Allow the center of content to be positioned anywhere in the viewport
        let padding = f64::max(content_width, content_height); // Use content size as padding
        let expanded_min_x = min_x - padding;
        let expanded_min_y = min_y - padding;
        let expanded_max_x = max_x + padding;
        let expanded_max_y = max_y + padding;
        
        // Limit viewport to expanded bounds
        // This ensures nodes can be centered in the viewport and won't disappear
        self.viewport_x = self.viewport_x.clamp(
            expanded_min_x - viewport_width / 2.0, 
            expanded_max_x - viewport_width / 2.0
        );
        
        self.viewport_y = self.viewport_y.clamp(
            expanded_min_y - viewport_height / 2.0, 
            expanded_max_y - viewport_height / 2.0
        );
    }

    // Save state if modified
    pub fn save_if_modified(&mut self) -> Result<(), JsValue> {
        if self.state_modified {
            // Persist to localStorage and schedule the *single* API PATCH
            // call via ``save_state``.  We removed the second direct call to
            // `save_state_to_api` to avoid duplicate network requests for the
            // same state update.
            let result = crate::storage::save_state(self);
            
            // Sync agent messages – removed as part of node/agent decoupling
            
            self.state_modified = false;
            result
        } else {
            Ok(())
        }
    }

    // Separate method to refresh UI after state changes
    pub fn refresh_ui_after_state_change() -> Result<(), JsValue> {
        // Refresh both canvas and dashboard views to ensure all UI elements are in sync
        let window = web_sys::window().ok_or(JsValue::from_str("No window"))?;
        let document = window.document().ok_or(JsValue::from_str("No document"))?;
        
        // Get the active view once to avoid multiple borrows
        let active_view = APP_STATE.with(|state| {
            let state = state.borrow();
            state.active_view.clone() // Clone to avoid borrowing issues
        });
        
        web_sys::console::log_1(&format!("Refreshing UI for active view: {:?}", active_view).into());
        
        // First render the active view to ensure proper display of containers
        crate::views::render_active_view_by_type(&active_view, &document)?;
        
        // Only refresh components relevant to the current view
        match active_view {
            crate::storage::ActiveView::Dashboard => {
                // For Dashboard view, only refresh the dashboard component
                web_sys::console::log_1(&"Refreshing dashboard components".into());
                crate::components::dashboard::refresh_dashboard(&document)?;
            },
            crate::storage::ActiveView::Canvas => {
                // For Canvas view, refresh the canvas and agent shelf
                web_sys::console::log_1(&"Refreshing canvas components".into());
                
                // Check if we need to refresh canvas in a separate borrow scope
                let has_canvas = APP_STATE.with(|state| {
                    let state = state.borrow();
                    state.canvas.is_some() && state.context.is_some()
                });
                
                if has_canvas {
                    // Refresh canvas in a separate borrow scope
                    APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.mark_dirty();
                    });
                }
                
                // Only refresh the agent shelf for Canvas view
                let _ = crate::components::agent_shelf::refresh_agent_shelf(&document);
            },
            crate::storage::ActiveView::ChatView => {
                // For Chat view, refresh chat components
                web_sys::console::log_1(&"Refreshing chat components".into());
                // Chat view refreshes are handled by its own code
            },
            crate::storage::ActiveView::Profile => {
                // Profile page currently doesn't need dynamic refresh logic
                web_sys::console::log_1(&"Refreshing profile components".into());
            }
        }
        
        Ok(())
    }

    // -------------------------------------------------------------------
    // SANITY HELPERS – must be called after *any* change to zoom/viewport.
    // -------------------------------------------------------------------

    /// Clamp `zoom_level` to the hard limits.
    pub fn clamp_zoom(&mut self) {
        self.zoom_level = self.zoom_level.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Clamp `viewport_x` / `viewport_y` so that the centre of the viewport
    /// cannot move farther than half a canvas from the origin.  This keeps
    /// panning within ±50 % of the default view.
    pub fn clamp_viewport(&mut self) {
        // Determine canvas dimensions (world units depend on zoom).
        let (canvas_w, canvas_h) = if let Some(canvas) = &self.canvas {
            let window = web_sys::window().unwrap();
            let dpr = window.device_pixel_ratio();
            (canvas.width() as f64 / dpr, canvas.height() as f64 / dpr)
        } else {
            (self.canvas_width.max(1.0), self.canvas_height.max(1.0))
        };

        // Half of the viewport size in world units.
        let half_w = (canvas_w / self.zoom_level) * 0.5;
        let half_h = (canvas_h / self.zoom_level) * 0.5;

        self.viewport_x = self.viewport_x.clamp(-half_w, half_w);
        self.viewport_y = self.viewport_y.clamp(-half_h, half_h);
    }

    pub fn resize_node_for_content(&mut self, node_id: &str) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            // Calculate approximate node size based on text content
            let chars_per_line = 25; // Approximate chars per line
            let lines = (node.text.len() as f64 / chars_per_line as f64).ceil() as usize;
            
            // Set minimum sizes but allow for growth
            node.width = f64::max(200.0, chars_per_line as f64 * 8.0); // Estimate width based on chars
            node.height = f64::max(80.0, lines as f64 * 20.0 + 40.0);  // Base height + lines
            
            // Mark state as modified
            self.state_modified = true;
        }
    }

    /// Returns the saved task-instructions for the given node / agent.
    ///
    /// * `Ok(String)` – instructions found.
    /// * `Err(&str)`  – agent resolved but has no instructions, or could not
    ///                  resolve an agent for the provided `node_id`.
    pub fn get_task_instructions(&self, node_id: &str) -> Result<String, &'static str> {
        let agent_id_opt = self.nodes.get(node_id).and_then(|n| n.agent_id);

        let aid = agent_id_opt.ok_or("Could not resolve agent_id from node_id")?;

        let agent = self.agents.get(&aid).ok_or("Agent not found in state")?;

        agent.task_instructions.clone().ok_or("Agent has no task instructions set")
    }

    // New dispatch method to handle messages
    pub fn dispatch(&mut self, msg: Message) -> Vec<Command> {
        // Call the update function and return its commands
        update::update(self, msg)
    }

    // Update to set the selected node ID and load messages if it's an agent
    pub fn _select_node(&mut self, node_id: Option<String>) {
        // First unselect any currently selected node
        if let Some(current_id) = &self.selected_node_id {
            if let Some(node) = self.nodes.get_mut(current_id) {
                node.is_selected = false;
            }
        }
        
        // Set the new selected node
        self.selected_node_id = node_id.clone();
        
        // Mark the new node as selected
        if let Some(id) = &node_id {
            if let Some(node) = self.nodes.get_mut(id) {
                node.is_selected = true;
            }
        }
        
        // Flag for redraw
        self.mark_dirty();
    }

    /// Creates a new node linked to an optional agent
    pub fn add_node_with_agent(&mut self, agent_id: Option<u32>, x: f64, y: f64, 
                    node_type: NodeType, text: String) -> String {
        // Generate a unique ID for the node
        if node_type == NodeType::AgentIdentity {
            // Delegate to the dedicated helper so we keep the mapping in sync.
            if let Some(aid) = agent_id {
                return self.add_agent_node(aid, text, x, y);
            }
        }

        // Fallback – generic node (or agent node without backend id yet).
        let node_id = format!("node-{}", js_sys::Date::now() as u32);

        let node = Node {
            node_id: node_id.clone(),
            agent_id,
            x,
            y,
            width: DEFAULT_NODE_WIDTH,
            height: DEFAULT_NODE_HEIGHT,
            color: match node_type {
                NodeType::UserInput => "#3498db".to_string(),    // Blue
                NodeType::ResponseOutput => "#9b59b6".to_string(), // Purple
                NodeType::AgentIdentity => DEFAULT_AGENT_NODE_COLOR.to_string(),
                NodeType::GenericNode => "#95a5a6".to_string(),  // Gray
                NodeType::Tool { .. } => "#f59e0b".to_string(),  // Orange
                NodeType::Trigger { .. } => "#10b981".to_string(), // Green
            },
            text,
            node_type,
            parent_id: None,
            is_selected: false,
            is_dragging: false,
            exec_status: None,
        };

        self.nodes.insert(node_id.clone(), node);
        self.state_modified = true;

        node_id
    }
    
    /// Creates a new workflow
    pub fn create_workflow(&mut self, name: String) -> u32 {
        // Generate a new workflow ID (simply use the current timestamp for now)
        let workflow_id = (Date::now() / 1000.0) as u32;
        
        // Create the new workflow
        let workflow = Workflow {
            id: workflow_id,
            name,
            nodes: Vec::new(),
            edges: Vec::new(),
        };
        
        // Add the workflow to our collection
        self.workflows.insert(workflow_id, workflow);
        
        // Set this as the current workflow
        self.current_workflow_id = Some(workflow_id);
        
        self.state_modified = true;
        
        // Return the new workflow's ID
        workflow_id
    }
    
    /// Creates an edge between two canvas nodes
    pub fn add_edge(&mut self, from_node_id: String, to_node_id: String, label: Option<String>) -> String {
        // Generate a unique ID for the edge
        let edge_id = format!("edge-{}", Date::now() as u32);
        
        // Create the new edge
        let edge = Edge {
            id: edge_id.clone(),
            from_node_id,
            to_node_id,
            label,
        };
        
        // If we have a current workflow, add this edge to it
        if let Some(workflow_id) = self.current_workflow_id {
            if let Some(workflow) = self.workflows.get_mut(&workflow_id) {
                workflow.edges.push(edge);
            }
        }
        
        self.state_modified = true;
        
        // Return the new edge's ID
        edge_id
    }
}

// We use thread_local to store our app state
thread_local! {
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new());
}

// Add a public function to update the app state with data from the API
#[allow(dead_code)]
pub fn update_app_state_from_api(nodes: HashMap<String, Node>) -> Result<(), JsValue> {
    // Get access to the global APP_STATE
    APP_STATE.with(|app_state_ref| {
        let mut app_state = app_state_ref.borrow_mut();
        
        // Update the nodes with those loaded from the API
        for (node_id, node) in nodes {
            app_state.nodes.insert(node_id, node);
        }
        
        // Flag that the state has been modified
        app_state.state_modified = true;
        
        // Refresh the UI
        if let Some(_canvas) = &app_state.canvas {
            if let Some(_context) = &app_state.context {
                // Use the correct draw_nodes function instead of render_canvas
                crate::canvas::renderer::draw_nodes(&mut *app_state);
            }
        }
    });
    
    Ok(())
}

// Helper function to update node IDs after API creation
#[allow(dead_code)]
pub fn update_node_id(old_id: &str, new_id: &str) {
    APP_STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        
        // If the node exists with the old ID
        if let Some(node) = state.nodes.remove(old_id) {
            // Insert it with the new ID
            let mut updated_node = node.clone();
            // Directly change the `node_id` field – no helper needed.
            updated_node.node_id = new_id.to_string();
            state.nodes.insert(new_id.to_string(), updated_node);
            
            web_sys::console::log_1(&format!("Updated node ID from {} to {}", old_id, new_id).into());
            
            // Also update any relationships like parent IDs
            for (_, child_node) in state.nodes.iter_mut() {
                if let Some(parent_id) = &child_node.parent_id {
                    if parent_id == old_id {
                        child_node.parent_id = Some(new_id.to_string());
                    }
                }
            }
            
            // Update selected node if necessary
            if let Some(selected_id) = &state.selected_node_id {
                if selected_id == old_id {
                    state.selected_node_id = Some(new_id.to_string());
                }
            }
            
            // Mark state as modified to ensure it gets saved
            state.state_modified = true;
            
            // Update the UI to reflect the changes
            if let Err(e) = AppState::refresh_ui_after_state_change() {
                web_sys::console::error_1(&format!("Error refreshing UI after node ID update: {:?}", e).into());
            }
        }
    });
}

// Global helper function for dispatching messages with proper UI refresh handling
pub fn dispatch_global_message(msg: crate::messages::Message) {
    // 1. Perform state updates and collect commands
    let (commands, network_data) = APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let commands = state.dispatch(msg);
        
        // Take ownership of pending network data
        // We'll gradually migrate these to commands
        let network = state.pending_network_call.take();
        
        (commands, network)
    });
    
    // 2. Execute commands after state borrow is dropped
    for cmd in commands {
        match cmd {
            Command::SendMessage(msg) => dispatch_global_message(msg),
            // UI update closures – comment out verbose log that fired on
            // every incremental render.
            Command::UpdateUI(ui_fn) => {
                ui_fn();
            },
            
            // Group commands by type and delegate to appropriate executor
            cmd @ Command::FetchThreads(_) |
            cmd @ Command::FetchThreadMessages(_) |
            cmd @ Command::LoadAgentInfo(_) |
            cmd @ Command::FetchAgents |
            cmd @ Command::FetchAgentRuns(_) |
            cmd @ Command::FetchAgentDetails(_) => crate::command_executors::execute_fetch_command(cmd),
            cmd @ Command::FetchWorkflows => crate::command_executors::execute_fetch_command(cmd),
            cmd @ Command::CreateWorkflowApi { .. } |
            cmd @ Command::DeleteWorkflowApi { .. } |
            cmd @ Command::RenameWorkflowApi { .. } => crate::command_executors::execute_fetch_command(cmd),
            cmd @ Command::FetchTriggers(_) |
            cmd @ Command::CreateTrigger { .. } |
            cmd @ Command::DeleteTrigger(_) => crate::command_executors::execute_fetch_command(cmd),
            
            cmd @ Command::CreateThread { .. } |
            cmd @ Command::SendThreadMessage { .. } |
            cmd @ Command::UpdateThreadTitle { .. } |
            cmd @ Command::RunThread(_) => crate::command_executors::execute_thread_command(cmd),
            
            // Group network calls together with consistent cmd binding
            cmd @ Command::NetworkCall { .. } |
            cmd @ Command::UpdateAgent { .. } |
            cmd @ Command::DeleteAgentApi { .. } => crate::command_executors::execute_network_command(cmd),
            
            cmd @ Command::WebSocketAction { .. } => crate::command_executors::execute_websocket_command(cmd),

            // Persist debounced state saves
            Command::SaveState => crate::command_executors::execute_save_command(),
            
            Command::NoOp => {},
        }
    }
    
    // 3. Process legacy network calls (these will be migrated to commands)
    if let Some((text, message_id)) = network_data {
        // This only handles thread messages now
        if let Ok(thread_id) = message_id.parse::<u32>() {
            // If message_id is a thread ID, it's a thread message
            let command = Command::SendThreadMessage { 
                thread_id, 
                content: text, 
                client_id: None 
            };
            crate::command_executors::execute_thread_command(command);
        } else {
            web_sys::console::warn_1(&format!("Unhandled pending network call: {}", message_id).into());
        }
    }
}
