use std::collections::HashMap;
use std::cell::RefCell;
use web_sys::{HtmlCanvasElement, CanvasRenderingContext2d, WebSocket};
use crate::models::{Node, NodeType};
use crate::canvas::renderer;

// Store global application state
pub struct AppState {
    pub nodes: HashMap<String, Node>,
    pub canvas: Option<HtmlCanvasElement>,
    pub context: Option<CanvasRenderingContext2d>,
    pub input_text: String,
    pub dragging: Option<String>,
    pub drag_offset_x: f64,
    pub drag_offset_y: f64,
    // New fields for canvas dragging
    pub canvas_dragging: bool,
    pub canvas_drag_start_x: f64,
    pub canvas_drag_start_y: f64,
    pub websocket: Option<WebSocket>,
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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            canvas: None,
            context: None,
            input_text: String::new(),
            dragging: None,
            drag_offset_x: 0.0,
            drag_offset_y: 0.0,
            canvas_dragging: false,
            canvas_drag_start_x: 0.0,
            canvas_drag_start_y: 0.0,
            websocket: None,
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
        }
    }

    pub fn add_node(&mut self, text: String, x: f64, y: f64, node_type: NodeType) -> String {
        let id = format!("node_{}", self.nodes.len());
        
        // Determine color based on node type
        let color = match node_type {
            NodeType::UserInput => "#3498db".to_string(),    // Blue
            NodeType::AgentResponse => "#9b59b6".to_string(), // Purple
            NodeType::AgentIdentity => "#2ecc71".to_string(), // Green
        };
        
        // Calculate approximate node size based on text content
        // This is a simple heuristic - we could do more sophisticated text measurement
        let _words = text.split_whitespace().count();
        let chars_per_line = 25; // Approximate chars per line
        let lines = (text.len() as f64 / chars_per_line as f64).ceil() as usize;
        
        // Set minimum sizes but allow for growth
        let width = f64::max(200.0, chars_per_line as f64 * 8.0); // Estimate width based on chars
        let height = f64::max(80.0, lines as f64 * 20.0 + 40.0);  // Base height + lines
        
        // Clone node_type before using it in the struct
        let node_type_clone = node_type.clone();
        
        let node = Node {
            id: id.clone(),
            x,
            y,
            text,
            width,
            height,
            color,
            parent_id: None,
            node_type: node_type_clone,
        };
        self.nodes.insert(id.clone(), node);
        
        // If this is a user input node, update the latest_user_input_id
        if let NodeType::UserInput = node_type {
            self.latest_user_input_id = Some(id.clone());
        }
        
        // Auto-fit all nodes if enabled
        if self.auto_fit && self.nodes.len() > 1 {
            self.fit_nodes_to_view();
        }
        
        id
    }

    pub fn add_response_node(&mut self, parent_id: &str, response_text: String) {
        if let Some(parent) = self.nodes.get(parent_id) {
            // Position the response node below the parent with some offset
            let x = parent.x + 50.0; // Slight offset to the right
            let y = parent.y + parent.height + 30.0; // Below the parent node
            
            let response_id = self.add_node(response_text, x, y, NodeType::AgentResponse);
            
            // Update the parent_id relationship
            if let Some(response_node) = self.nodes.get_mut(&response_id) {
                response_node.parent_id = Some(parent_id.to_string());
            }
        }
    }
    
    pub fn draw_nodes(&self) {
        renderer::draw_nodes(self);
    }
    
    pub fn update_node_position(&mut self, node_id: &str, x: f64, y: f64) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.x = x;
            node.y = y;
            
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
            let required_width = max_x - min_x + 80.0; // Add padding
            let required_height = max_y - min_y + 80.0;
            
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
            
            // Center the content
            let new_viewport_x = min_x - 40.0; // Add padding
            let new_viewport_y = min_y - 40.0;
            
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
        
        // Use existing fit method to center the view
        self.fit_nodes_to_view();
        
        // Restore original auto-fit setting
        self.auto_fit = original_auto_fit;
    }

    // Generate a unique message ID
    pub fn generate_message_id(&self) -> String {
        format!("msg_{}", js_sys::Date::now())
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
        
        // Add padding around content bounds
        let padding = 500.0; // Large padding to allow reasonable movement
        min_x -= padding;
        min_y -= padding;
        max_x += padding;
        max_y += padding;
        
        // Calculate viewport constraints
        // Don't let viewport move too far from content in any direction
        let content_width = max_x - min_x;
        let content_height = max_y - min_y;
        
        // Keep the viewport x within bounds
        if self.viewport_x < min_x {
            self.viewport_x = min_x;
        } else if self.viewport_x > max_x - content_width / 4.0 {
            self.viewport_x = max_x - content_width / 4.0;
        }
        
        // Keep the viewport y within bounds
        if self.viewport_y < min_y {
            self.viewport_y = min_y;
        } else if self.viewport_y > max_y - content_height / 4.0 {
            self.viewport_y = max_y - content_height / 4.0;
        }
    }
}

// We use thread_local to store our app state
thread_local! {
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new());
} 