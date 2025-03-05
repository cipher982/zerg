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
    pub websocket: Option<WebSocket>,
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
            websocket: None,
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
        
        let node = Node {
            id: id.clone(),
            x,
            y,
            text,
            width,
            height,
            color,
            parent_id: None,
            node_type,
        };
        self.nodes.insert(id.clone(), node);
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
            self.draw_nodes();
        }
    }
    
    pub fn find_node_at_position(&self, x: f64, y: f64) -> Option<(String, f64, f64)> {
        for (id, node) in &self.nodes {
            if x >= node.x && x <= node.x + node.width &&
               y >= node.y && y <= node.y + node.height {
                return Some((id.clone(), x - node.x, y - node.y));
            }
        }
        None
    }
}

// We use thread_local to store our app state
thread_local! {
    pub static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new());
} 