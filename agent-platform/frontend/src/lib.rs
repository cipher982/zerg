use wasm_bindgen::prelude::*;
use web_sys::{
    Document, HtmlCanvasElement, HtmlInputElement,
    MouseEvent, WebSocket, MessageEvent, CanvasRenderingContext2d,
};
use js_sys;
use serde::{Serialize, Deserialize};
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
enum NodeType {
    UserInput,
    AgentResponse,
    AgentIdentity, // For future use to represent persistent agents
}

// Define a struct for nodes in our visualization
#[derive(Clone, Serialize, Deserialize)]
struct Node {
    id: String,
    x: f64,
    y: f64,
    text: String,
    width: f64,
    height: f64,
    color: String,
    parent_id: Option<String>,
    node_type: NodeType,
}

// Store global application state
struct AppState {
    nodes: HashMap<String, Node>,
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    input_text: String,
    dragging: Option<String>,
    drag_offset_x: f64,
    drag_offset_y: f64,
    websocket: Option<WebSocket>,
}

impl AppState {
    fn new() -> Self {
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

    fn add_node(&mut self, text: String, x: f64, y: f64, node_type: NodeType) -> String {
        let id = format!("node_{}", self.nodes.len());
        
        // Determine color based on node type
        let color = match node_type {
            NodeType::UserInput => "#3498db".to_string(),    // Blue
            NodeType::AgentResponse => "#9b59b6".to_string(), // Purple
            NodeType::AgentIdentity => "#2ecc71".to_string(), // Green
        };
        
        // Calculate approximate node size based on text content
        // This is a simple heuristic - we could do more sophisticated text measurement
        let words = text.split_whitespace().count();
        let chars_per_line = 25; // Approximate chars per line
        let lines = (text.len() as f64 / chars_per_line as f64).ceil() as usize;
        
        // Set minimum sizes but allow for growth
        let width = 200.0.max(chars_per_line as f64 * 8.0); // Estimate width based on chars
        let height = 80.0.max(lines as f64 * 20.0 + 40.0);  // Base height + lines
        
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

    fn add_response_node(&mut self, parent_id: &str, response_text: String) {
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
}

// We use thread_local to store our app state
thread_local! {
    static APP_STATE: RefCell<AppState> = RefCell::new(AppState::new());
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Set up the application
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    setup_ui(&document)?;
    setup_canvas(&document)?;
    setup_websocket()?;
    
    Ok(())
}

fn setup_ui(document: &Document) -> Result<(), JsValue> {
    // Create input field
    let input = document.create_element("input")?;
    input.set_attribute("type", "text")?;
    input.set_attribute("id", "user-input")?;
    input.set_attribute("placeholder", "Enter text here...")?;
    input.set_attribute("style", "width: 300px; padding: 8px; margin-right: 10px;")?;
    
    // Create send button
    let button = document.create_element("button")?;
    button.set_inner_html("Send to AI");
    button.set_attribute("id", "send-button")?;
    button.set_attribute("style", "padding: 8px 16px; background-color: #2ecc71; color: white; border: none; cursor: pointer;")?;
    
    // Create input container
    let input_container = document.create_element("div")?;
    input_container.set_attribute("style", "margin-bottom: 20px;")?;
    input_container.append_child(&input)?;
    input_container.append_child(&button)?;
    
    // Create canvas container
    let canvas_container = document.create_element("div")?;
    canvas_container.set_id("canvas-container");
    canvas_container.set_attribute("style", "border: 1px solid #ddd; width: 100%; height: 500px; position: relative;")?;
    
    // Create canvas
    let canvas = document.create_element("canvas")?;
    canvas.set_id("node-canvas");
    canvas.set_attribute("width", "800")?;
    canvas.set_attribute("height", "500")?;
    canvas.set_attribute("style", "position: absolute; top: 0; left: 0;")?;
    
    canvas_container.append_child(&canvas)?;
    
    // Add everything to the document body
    let app_container = document.create_element("div")?;
    app_container.set_attribute("style", "padding: 20px;")?;
    app_container.append_child(&input_container)?;
    app_container.append_child(&canvas_container)?;
    
    document.body().unwrap().append_child(&app_container)?;
    
    // Set up event listener for the button
    let button_el = document.get_element_by_id("send-button").unwrap();
    let input_el = document.get_element_by_id("user-input").unwrap();
    
    let button_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
        let input = input_el.dyn_ref::<HtmlInputElement>().unwrap();
        let text = input.value();
        
        if !text.is_empty() {
            // Add a node for the user's input
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.input_text = text.clone();
                // Position the node in the left side of the canvas at a consistent position
                // If it's the first node, start higher up
                let y_position = if state.nodes.is_empty() {
                    100.0 // Start at the top
                } else {
                    // Find the lowest y-position of existing nodes
                    let lowest_y = state.nodes.values()
                        .map(|n| n.y + n.height)
                        .fold(0.0, f64::max);
                    lowest_y + 50.0 // Position below the lowest node with spacing
                };
                state.add_node(text.clone(), 50.0, y_position, NodeType::UserInput);
                state.draw_nodes();
            });
            
            // Send the text to the backend
            send_text_to_backend(&text);
            
            // Clear the input field
            input.set_value("");
        }
    }) as Box<dyn FnMut(_)>);
    
    button_el.add_event_listener_with_callback(
        "click",
        button_handler.as_ref().unchecked_ref(),
    )?;
    button_handler.forget();
    
    Ok(())
}

fn setup_canvas(document: &Document) -> Result<(), JsValue> {
    let canvas = document.get_element_by_id("node-canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;
    
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<CanvasRenderingContext2d>()?;
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.canvas = Some(canvas.clone());
        state.context = Some(context);
    });
    
    // Set up mouse events for the canvas
    setup_canvas_mouse_events(&canvas)?;
    
    Ok(())
}

fn setup_canvas_mouse_events(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    // Mouse down event
    let _canvas_mousedown = canvas.clone();
    let mousedown_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Check if we clicked on a node
            let mut node_clicked = None;
            
            // First find which node was clicked on
            for (id, node) in &state.nodes {
                if x >= node.x && x <= node.x + node.width &&
                   y >= node.y && y <= node.y + node.height {
                    node_clicked = Some((id.clone(), x - node.x, y - node.y));
                    break;
                }
            }
            
            // Then update the state with the clicked node
            if let Some((id, offset_x, offset_y)) = node_clicked {
                state.dragging = Some(id);
                state.drag_offset_x = offset_x;
                state.drag_offset_y = offset_y;
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas.add_event_listener_with_callback(
        "mousedown",
        mousedown_handler.as_ref().unchecked_ref(),
    )?;
    mousedown_handler.forget();
    
    // Mouse move event
    let _canvas_mousemove = canvas.clone();
    let mousemove_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            if let Some(id) = &state.dragging {
                let id = id.clone(); // Clone the ID to avoid borrowing conflicts
                let drag_offset_x = state.drag_offset_x; // Copy these values
                let drag_offset_y = state.drag_offset_y;
                
                if let Some(node) = state.nodes.get_mut(&id) {
                    node.x = x - drag_offset_x;
                    node.y = y - drag_offset_y;
                    state.draw_nodes();
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas.add_event_listener_with_callback(
        "mousemove",
        mousemove_handler.as_ref().unchecked_ref(),
    )?;
    mousemove_handler.forget();
    
    // Mouse up event
    let _canvas_mouseup = canvas.clone();
    let mouseup_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dragging = None;
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas.add_event_listener_with_callback(
        "mouseup",
        mouseup_handler.as_ref().unchecked_ref(),
    )?;
    mouseup_handler.forget();
    
    Ok(())
}

fn setup_websocket() -> Result<(), JsValue> {
    // Create a new WebSocket connection
    let ws = WebSocket::new("ws://localhost:8001/ws")?;
    
    // Set up message event handler
    let onmessage_callback = Closure::wrap(Box::new(move |event: MessageEvent| {
        // Get the message data as string
        let response = event.data().as_string().unwrap();
        
        // Create a response node
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Get the latest node that was added (the user's input)
            if let Some(first_node_id) = state.nodes.keys().next().cloned() {
                state.add_response_node(&first_node_id, response);
                state.draw_nodes();
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
    onmessage_callback.forget();
    
    // Set up error handler
    let onerror_callback = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        web_sys::console::log_1(&"WebSocket error occurred".into());
    }) as Box<dyn FnMut(_)>);
    
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
    });
    
    Ok(())
}

fn send_text_to_backend(text: &str) {
    // Use the Fetch API to send data to the backend
    let window = web_sys::window().expect("no global window exists");
    
    // Create headers
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Content-Type", "application/json").unwrap();
    
    // Create request body
    let body_obj = js_sys::Object::new();
    js_sys::Reflect::set(&body_obj, &"text".into(), &text.into()).unwrap();
    let body_string = js_sys::JSON::stringify(&body_obj).unwrap();
    
    // Create request init object
    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");
    opts.set_headers(&headers.into());
    
    // Convert body_string to JsValue
    let body_value: JsValue = body_string.into();
    opts.set_body(&body_value);
    
    // Create request
    let request = web_sys::Request::new_with_str_and_init(
        "http://localhost:8001/api/process-text", 
        &opts
    ).unwrap();
    
    // Send the fetch request
    let _ = window.fetch_with_request(&request);
    
    // We're not handling the response here since we'll get it via WebSocket
}

impl AppState {
    fn draw_nodes(&self) {
        if let (Some(canvas), Some(context)) = (&self.canvas, &self.context) {
            // Clear the canvas
            context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            
            // Draw connections between nodes
            for (_, node) in &self.nodes {
                if let Some(parent_id) = &node.parent_id {
                    if let Some(parent) = self.nodes.get(parent_id) {
                        // Draw curved connection
                        context.begin_path();
                        
                        // Start point (bottom of parent node)
                        let start_x = parent.x + parent.width / 2.0;
                        let start_y = parent.y + parent.height;
                        
                        // End point (top of response node)
                        let end_x = node.x + node.width / 2.0;
                        let end_y = node.y;
                        
                        // Control points for the bezier curve - creates a nice "S" curve
                        let midpoint_y = start_y + (end_y - start_y) / 2.0;
                        let control_x1 = start_x;
                        let control_y1 = midpoint_y - 10.0;
                        let control_x2 = end_x;
                        let control_y2 = midpoint_y + 10.0;
                        
                        // Draw the curve
                        context.move_to(start_x, start_y);
                        context.bezier_curve_to(
                            control_x1, control_y1,
                            control_x2, control_y2,
                            end_x, end_y
                        );
                        
                        // Style and stroke the path
                        context.set_stroke_style(&JsValue::from("#95a5a6"));
                        context.set_line_width(2.0);
                        context.stroke();
                        
                        // Optional: Add an arrow at the end pointing down
                        self.draw_arrow(context, end_x, end_y, 0.0, -1.0);
                    }
                }
            }
            
            // Draw all nodes
            for (_, node) in &self.nodes {
                // Determine if this is a user input or AI response based on node_type
                match node.node_type {
                    NodeType::UserInput => {
                        // Draw a rounded rectangle for user inputs
                        self.draw_rounded_rect(context, node);
                    },
                    NodeType::AgentResponse => {
                        // Draw a thought bubble for AI responses
                        self.draw_thought_bubble(context, node);
                    },
                    NodeType::AgentIdentity => {
                        // For future use - draw a hexagon or other distinctive shape
                        self.draw_rounded_rect(context, node); // Placeholder for now
                    },
                }
                
                // Draw node text
                context.set_fill_style(&JsValue::from("#ffffff"));
                context.set_font("15px Arial");
                context.set_text_align("left");
                context.set_text_baseline("top");
                
                // Handle text wrapping with better spacing
                let padding = 15.0;
                let max_width = node.width - padding * 2.0;
                let words = node.text.split_whitespace().collect::<Vec<&str>>();
                let mut line = String::new();
                let mut y_offset = node.y + padding;
                
                for word in words {
                    let test_line = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{} {}", line, word)
                    };
                    
                    // Estimate text width
                    let estimated_width = test_line.len() as f64 * 7.0;
                    
                    if estimated_width > max_width && !line.is_empty() {
                        // Draw the current line
                        context.fill_text(&line, node.x + padding, y_offset).unwrap();
                        line = word.to_string();
                        y_offset += 22.0; // Increase line height for better readability
                    } else {
                        line = test_line;
                    }
                }
                
                // Draw the last line
                if !line.is_empty() {
                    context.fill_text(&line, node.x + padding, y_offset).unwrap();
                }
            }
        }
    }

    // Add a helper method for drawing rounded rectangles
    fn draw_rounded_rect(&self, context: &CanvasRenderingContext2d, node: &Node) {
        let radius = 15.0;
        let x = node.x;
        let y = node.y;
        let width = node.width;
        let height = node.height;
        
        // Set fill style
        context.set_fill_style(&JsValue::from(node.color.clone()));
        
        // Draw rounded rectangle
        context.begin_path();
        context.move_to(x + radius, y);
        context.line_to(x + width - radius, y);
        context.quadratic_curve_to(x + width, y, x + width, y + radius);
        context.line_to(x + width, y + height - radius);
        context.quadratic_curve_to(x + width, y + height, x + width - radius, y + height);
        context.line_to(x + radius, y + height);
        context.quadratic_curve_to(x, y + height, x, y + height - radius);
        context.line_to(x, y + radius);
        context.quadratic_curve_to(x, y, x + radius, y);
        context.close_path();
        
        // Fill and stroke
        context.fill();
        context.set_stroke_style(&JsValue::from("#2c3e50"));
        context.stroke();
    }

    // Add a helper method for drawing arrows
    fn draw_arrow(&self, context: &CanvasRenderingContext2d, x: f64, y: f64, dx: f64, dy: f64) {
        let head_len = 10.0; // length of arrow head
        let angle = f64::atan2(dy, dx);
        
        context.begin_path();
        context.move_to(x, y);
        context.line_to(
            x - head_len * f64::cos(angle - std::f64::consts::PI / 6.0),
            y - head_len * f64::sin(angle - std::f64::consts::PI / 6.0)
        );
        context.move_to(x, y);
        context.line_to(
            x - head_len * f64::cos(angle + std::f64::consts::PI / 6.0),
            y - head_len * f64::sin(angle + std::f64::consts::PI / 6.0)
        );
        context.set_stroke_style(&JsValue::from("#95a5a6"));
        context.set_line_width(2.0);
        context.stroke();
    }

    // Add a helper method for drawing thought bubbles
    fn draw_thought_bubble(&self, context: &CanvasRenderingContext2d, node: &Node) {
        let x = node.x;
        let y = node.y;
        let width = node.width;
        let height = node.height;
        
        // Set fill style
        context.set_fill_style(&JsValue::from(node.color.clone()));
        
        // Draw main bubble
        context.begin_path();
        
        // Draw a cloud-like shape
        let radius = 30.0;
        
        // Top edge with bumps
        context.move_to(x + radius, y);
        context.arc(
            x + radius, 
            y + radius, 
            radius, 
            std::f64::consts::PI * 1.5, 
            std::f64::consts::PI * 0.0, 
            false
        );
        
        context.arc(
            x + width / 2.0, 
            y, 
            radius * 0.8, 
            std::f64::consts::PI, 
            std::f64::consts::PI * 0.0, 
            true
        );
        
        context.arc(
            x + width - radius, 
            y + radius, 
            radius, 
            std::f64::consts::PI * 1.0, 
            std::f64::consts::PI * 1.5, 
            false
        );
        
        // Right edge
        context.arc(
            x + width, 
            y + height / 2.0, 
            radius * 0.8, 
            std::f64::consts::PI * 1.5, 
            std::f64::consts::PI * 0.5, 
            false
        );
        
        // Bottom edge
        context.arc(
            x + width - radius, 
            y + height - radius, 
            radius, 
            std::f64::consts::PI * 0.0, 
            std::f64::consts::PI * 0.5, 
            false
        );
        
        context.arc(
            x + width / 2.0, 
            y + height, 
            radius * 0.8, 
            std::f64::consts::PI * 0.0, 
            std::f64::consts::PI * 1.0, 
            false
        );
        
        context.arc(
            x + radius, 
            y + height - radius, 
            radius, 
            std::f64::consts::PI * 0.5, 
            std::f64::consts::PI * 1.0, 
            false
        );
        
        // Left edge
        context.arc(
            x, 
            y + height / 2.0, 
            radius * 0.8, 
            std::f64::consts::PI * 0.5, 
            std::f64::consts::PI * 1.5, 
            false
        );
        
        context.close_path();
        context.fill();
        
        // Draw the border
        context.set_stroke_style(&JsValue::from("#2c3e50"));
        context.stroke();
        
        // Add small thought bubbles connecting to main bubble
        let small_bubble_sizes = [8.0, 6.0, 4.0];
        let mut last_x = x + width / 4.0;
        let mut last_y = y + height;
        
        for size in small_bubble_sizes.iter() {
            // Position the small bubble below the previous one
            last_y += *size * 1.5;
            
            // Draw small circle
            context.begin_path();
            context.arc(last_x, last_y, *size, 0.0, std::f64::consts::PI * 2.0, false);
            context.fill();
            context.stroke();
            
            // Move x slightly for next bubble
            last_x -= *size * 0.8;
        }
    }
} 