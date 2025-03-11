use std::collections::HashMap;
use std::cell::RefCell;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, WebSocket};
use crate::models::{Node, NodeType, CanvasNode, Workflow, Edge, ApiAgent};
use crate::canvas::renderer;
use crate::storage::ActiveView;
use js_sys::Date;
use wasm_bindgen::JsValue;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use std::rc::Rc;
use crate::messages::Message;
use crate::update::update;
use crate::constants::DEFAULT_TASK_INSTRUCTIONS;

// Store global application state
pub struct AppState {
    // Original node structure (will be phased out gradually)
    pub nodes: HashMap<String, Node>,
    
    // New separated data structures
    pub agents: HashMap<u32, ApiAgent>,        // Backend agent data
    pub canvas_nodes: HashMap<String, CanvasNode>, // Visual layout 
    pub workflows: HashMap<u32, Workflow>,     // Workflows collection
    pub current_workflow_id: Option<u32>,      // Currently active workflow
    
    // Canvas and rendering related
    pub canvas: Option<HtmlCanvasElement>,
    pub context: Option<CanvasRenderingContext2d>,
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
    // Whether state has been modified since last save
    pub state_modified: bool,
    // Currently selected node ID
    pub selected_node_id: Option<String>,
    // Flag to track if we're dragging an agent
    pub is_dragging_agent: bool,
    // Track the active view (Dashboard or Canvas)
    pub active_view: ActiveView,
    // Pending network call data to avoid nested borrows
    pub pending_network_call: Option<(String, String)>,
    // Loading state flags
    pub is_loading: bool,
    pub data_loaded: bool,
    pub api_load_attempted: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            agents: HashMap::new(),
            canvas_nodes: HashMap::new(),
            workflows: HashMap::new(),
            current_workflow_id: None,
            canvas: None,
            context: None,
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
            canvas_width: 800.0, // Default width
            canvas_height: 600.0, // Default height
            viewport_x: 0.0,
            viewport_y: 0.0,
            zoom_level: 1.0,
            auto_fit: true, // Enable auto-fit by default
            latest_user_input_id: None,
            message_id_to_node_id: HashMap::new(),
            selected_model: "gpt-4o".to_string(), // Default model
            available_models: vec![
                // Default models until we fetch from the server
                ("gpt-4o".to_string(), "GPT-4o".to_string()),
                ("gpt-4-turbo".to_string(), "GPT-4 Turbo".to_string()),
                ("gpt-3.5-turbo".to_string(), "GPT-3.5 Turbo".to_string()),
            ],
            state_modified: false,
            selected_node_id: None,
            is_dragging_agent: false,
            active_view: ActiveView::Dashboard, // Default to Dashboard view
            pending_network_call: None,
            is_loading: true,
            data_loaded: false,
            api_load_attempted: false,
        }
    }

    pub fn add_node(&mut self, text: String, x: f64, y: f64, node_type: NodeType) -> String {
        let id = format!("node_{}", self.nodes.len());
        web_sys::console::log_1(&format!("Creating node: id={}, type={:?}, text={}", id, node_type, text).into());
        
        // Determine color based on node type
        let color = match node_type {
            NodeType::UserInput => "#3498db".to_string(),    // Blue
            NodeType::ResponseOutput => "#9b59b6".to_string(), // Purple
            NodeType::AgentIdentity => "#2ecc71".to_string(), // Green
            NodeType::GenericNode => "#95a5a6".to_string(),  // Gray
        };
        
        // Calculate approximate node size based on text content
        let chars_per_line = 25; // Approximate chars per line
        let lines = (text.len() as f64 / chars_per_line as f64).ceil() as usize;
        
        // Set minimum sizes but allow for growth
        let width = f64::max(200.0, chars_per_line as f64 * 8.0); // Estimate width based on chars
        let height = f64::max(80.0, lines as f64 * 20.0 + 40.0);  // Base height + lines
        
        // Initialize system instructions, history, and status based on node type
        let (system_instructions, history, status) = match node_type {
            NodeType::UserInput => {
                web_sys::console::log_1(&JsValue::from_str("Initializing user input node properties"));
                (None, None, None)
            },
            NodeType::AgentIdentity => {
                web_sys::console::log_1(&JsValue::from_str("Initializing agent node properties"));
                
                // For agent identity nodes, create empty properties
                (Some(String::new()), Some(Vec::new()), Some("idle".to_string()))
            },
            NodeType::ResponseOutput => {
                web_sys::console::log_1(&JsValue::from_str("Initializing response node properties"));
                (None, None, None)
            },
            NodeType::GenericNode => {
                web_sys::console::log_1(&JsValue::from_str("Initializing generic node properties"));
                (None, None, None)
            }
        };
        
        let node = Node {
            id: id.clone(),
            x,
            y,
            text,
            width,
            height,
            color,
            parent_id: None, // Parent ID will be set separately if needed
            node_type,
            system_instructions,
            task_instructions: None, // Initialize with None
            history,
            status,
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

    pub fn add_response_node(&mut self, parent_id: &str, response_text: String) -> String {
        let response_id = format!("resp-{}", self.generate_message_id());
        let parent = self.nodes.get(parent_id);
        
        // Default position for response node is below parent
        let (mut x, mut y) = (100.0, 100.0);
        
        if let Some(parent_node) = parent {
            x = parent_node.x;
            y = parent_node.y + parent_node.height + 30.0;
        }
        
        let node = crate::models::Node {
            id: response_id.clone(),
            x,
            y,
            text: response_text.clone(),
            width: 300.0,
            height: 100.0,
            color: "#d5f5e3".to_string(),  // Light green
            parent_id: Some(parent_id.to_string()),
            node_type: crate::models::NodeType::ResponseOutput,
            
            // Response nodes don't have these fields
            system_instructions: None,
            task_instructions: None,
            history: None,
            status: None,
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
                
                let history = parent_node.history.get_or_insert_with(Vec::new);
                history.push(message.clone());
                
                // Sync this message with the API
                crate::storage::save_agent_messages_to_api(parent_id, &[message]);
            }
        }
        
        response_id
    }
    
    pub fn draw_nodes(&self) {
        renderer::draw_nodes(self);
    }
    
    pub fn update_node_position(&mut self, node_id: &str, x: f64, y: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.x = x;
            node.y = y;
            self.state_modified = true; // Mark state as modified
            
            // Auto-fit all nodes if enabled
            if self.auto_fit {
                self.fit_nodes_to_view();
            } else {
                self.draw_nodes();
            }
        }
    }
    
    pub fn find_node_at_position(&self, x: f64, y: f64) -> Option<(String, f64, f64)> {
        // Convert canvas coordinates to world coordinates
        let world_x = x / self.zoom_level + self.viewport_x;
        let world_y = y / self.zoom_level + self.viewport_y;
        
        for (id, node) in &self.nodes {
            if world_x >= node.x && world_x <= node.x + node.width &&
               world_y >= node.y && world_y <= node.y + node.height {
                return Some((id.clone(), world_x - node.x, world_y - node.y));
            }
        }
        None
    }
    
    // Apply transform to ensure all nodes are visible
    pub fn fit_nodes_to_view(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        
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
            
            // Use the smaller ratio to ensure everything fits
            let new_zoom = f64::min(width_ratio, height_ratio);
            
            // Limit maximum zoom level to prevent excessive zooming
            let max_zoom = 1.0; // Maximum zoom level (1.0 = 100%)
            let new_zoom = f64::min(new_zoom, max_zoom);
            
            // Calculate the center of the nodes
            let center_x = min_x + (max_x - min_x) / 2.0;
            let center_y = min_y + (max_y - min_y) / 2.0;
            
            // Calculate viewport position to center the content
            let new_viewport_x = center_x - (canvas_width / (2.0 * new_zoom));
            let new_viewport_y = center_y - (canvas_height / (2.0 * new_zoom));
            
            // Update state
            self.zoom_level = new_zoom;
            self.viewport_x = new_viewport_x;
            self.viewport_y = new_viewport_y;
            
            // Now redraw
            self.draw_nodes();
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
        // Store original auto-fit setting
        let original_auto_fit = self.auto_fit;
        
        // Temporarily disable auto-fit if it's on
        if original_auto_fit {
            self.auto_fit = false;
        }
        
        // Use existing fit method to center the view - this is known to work well
        self.fit_nodes_to_view();
        
        // Restore original auto-fit setting
        self.auto_fit = original_auto_fit;
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
                
                // Redraw with new viewport
                state.draw_nodes();
                
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
    
    // Get the node ID for a message ID
    pub fn get_node_id_for_message(&self, message_id: &str) -> Option<String> {
        self.message_id_to_node_id.get(message_id).cloned()
    }
    
    // Enforce viewport boundaries to prevent panning too far from content
    pub fn enforce_viewport_boundaries(&mut self) {
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
            // Save to API as the source of truth
            let result = crate::storage::save_state(self);
            
            // Save to the API
            crate::storage::save_state_to_api(self);
            
            // Sync agent messages
            for (node_id, node) in &self.nodes {
                if let crate::models::NodeType::AgentIdentity = node.node_type {
                    if let Some(history) = &node.history {
                        if !history.is_empty() {
                            crate::storage::save_agent_messages_to_api(node_id, history);
                        }
                    }
                }
            }
            
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
        
        // First render the active view to ensure proper display of containers
        crate::views::render_active_view_by_type(&active_view, &document)?;
        
        // Check if we need to refresh canvas in a separate borrow scope
        let has_canvas = APP_STATE.with(|state| {
            let state = state.borrow();
            state.canvas.is_some() && state.context.is_some()
        });
        
        if has_canvas {
            // Refresh canvas in a separate borrow scope
            APP_STATE.with(|state| {
                let state = state.borrow_mut();
                state.draw_nodes();
            });
        }
        
        // Always refresh the dashboard to ensure it has the latest data
        crate::components::dashboard::refresh_dashboard(&document)?;
        
        Ok(())
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

    /// Gets the task instructions for an agent with a standard fallback
    pub fn get_task_instructions_with_fallback(&self, agent_id: &str) -> String {
        self.nodes.get(agent_id)
            .and_then(|node| node.task_instructions.clone())
            .unwrap_or_else(|| DEFAULT_TASK_INSTRUCTIONS.to_string())
    }

    // New dispatch method to handle messages
    pub fn dispatch(&mut self, msg: Message) -> (bool, Option<(String, String)>) {
        update(self, msg);
        
        // Check if there's a pending network call
        let pending_call = self.pending_network_call.take();
        
        // Save state if it was modified
        if self.state_modified {
            if let Err(e) = self.save_if_modified() {
                web_sys::console::warn_1(&format!("Failed to save state: {:?}", e).into());
            }
        }
        
        // Return true to indicate that UI refresh is needed, along with any pending network call
        (true, pending_call)
    }

    // Update to set the selected node ID and load messages if it's an agent
    pub fn select_node(&mut self, node_id: Option<String>) {
        self.selected_node_id = node_id.clone();
        
        // If a node was selected and it's an agent, load its messages
        if let Some(node_id) = &node_id {
            if let Some(node) = self.nodes.get(node_id) {
                if let crate::models::NodeType::AgentIdentity = node.node_type {
                    // Load messages for this agent from the API
                    crate::storage::load_agent_messages_from_api(node_id, 1); // Using default agent ID of 1 for now
                }
            }
        }
    }

    /// Creates a new canvas node linked to an optional agent
    pub fn add_canvas_node(&mut self, agent_id: Option<u32>, x: f64, y: f64, 
                          node_type: NodeType, text: String) -> String {
        // Generate a unique ID for the canvas node
        let node_id = match node_type {
            NodeType::AgentIdentity => {
                if let Some(id) = agent_id {
                    format!("canvas-agent-{}", id)
                } else {
                    format!("canvas-node-{}", Date::now() as u32)
                }
            },
            _ => format!("canvas-node-{}", Date::now() as u32)
        };
        
        // Create the new canvas node
        let canvas_node = CanvasNode {
            node_id: node_id.clone(),
            agent_id,
            x,
            y,
            width: 200.0,
            height: 100.0,
            text,
            node_type,
            color: match node_type {
                NodeType::AgentIdentity => "#ffecb3".to_string(), // Light amber
                NodeType::UserInput => "#e3f2fd".to_string(),     // Light blue
                NodeType::ResponseOutput => "#e8f5e9".to_string(), // Light green
                NodeType::GenericNode => "#f5f5f5".to_string(),    // Light gray
            },
            parent_id: None,
            is_selected: false,
            is_dragging: false,
        };
        
        // Add the node to our canvas_nodes collection
        self.canvas_nodes.insert(node_id.clone(), canvas_node);
        
        // If we have a current workflow, add this node to it
        if let Some(workflow_id) = self.current_workflow_id {
            if let Some(workflow) = self.workflows.get_mut(&workflow_id) {
                if let Some(canvas_node) = self.canvas_nodes.get(&node_id) {
                    workflow.nodes.push(canvas_node.clone());
                }
            }
        }
        
        self.state_modified = true;
        
        // Return the new node's ID
        node_id
    }
    
    /// Updates the position of a canvas node
    pub fn update_canvas_node_position(&mut self, node_id: &str, x: f64, y: f64) {
        if let Some(node) = self.canvas_nodes.get_mut(node_id) {
            node.x = x;
            node.y = y;
            self.state_modified = true;
            
            // Also update the node in the current workflow if it exists
            if let Some(workflow_id) = self.current_workflow_id {
                if let Some(workflow) = self.workflows.get_mut(&workflow_id) {
                    for workflow_node in &mut workflow.nodes {
                        if workflow_node.node_id == node_id {
                            workflow_node.x = x;
                            workflow_node.y = y;
                            break;
                        }
                    }
                }
            }
        }
    }
    
    /// Finds a canvas node at the given position
    pub fn find_canvas_node_at_position(&self, x: f64, y: f64) -> Option<(String, f64, f64)> {
        let nodes: Vec<_> = self.canvas_nodes.iter().collect();
        for (id, node) in nodes.into_iter().rev() {
            if x >= node.x && x <= node.x + node.width &&
               y >= node.y && y <= node.y + node.height {
                // Return the node's ID and the offset from the mouse to the node's top-left corner
                return Some((id.clone(), x - node.x, y - node.y));
            }
        }
        None
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
                crate::canvas::renderer::draw_nodes(&app_state);
            }
        }
    });
    
    Ok(())
}

// Helper function to update node IDs after API creation
pub fn update_node_id(old_id: &str, new_id: &str) {
    APP_STATE.with(|state_ref| {
        let mut state = state_ref.borrow_mut();
        
        // If the node exists with the old ID
        if let Some(node) = state.nodes.remove(old_id) {
            // Insert it with the new ID
            let mut updated_node = node.clone();
            updated_node.id = new_id.to_string();
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
    // Dispatch the message to update state
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let (refresh_needed, pending_network_call) = state.dispatch(msg);
        
        // Store pending network call data for processing outside of this borrow
        let network_data = pending_network_call.clone();
        
        // Drop the mutable borrow before any additional operations
        drop(state);
        
        // Process any pending network calls
        if let Some((text, message_id)) = network_data {
            crate::network::send_text_to_backend(&text, message_id);
        }
        
        // Refresh UI after state changes if needed
        if refresh_needed {
            if let Err(e) = AppState::refresh_ui_after_state_change() {
                web_sys::console::warn_1(&format!("Failed to refresh UI after action: {:?}", e).into());
            }
        }
    });
} 