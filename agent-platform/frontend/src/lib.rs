use wasm_bindgen::prelude::*;
use web_sys::{
    Document, Element, HtmlCanvasElement, HtmlElement, HtmlInputElement,
    MouseEvent, WebSocket, MessageEvent, CanvasRenderingContext2d,
};
use js_sys::{Function, Object};
use serde::{Serialize, Deserialize};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

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

    fn add_node(&mut self, text: String, x: f64, y: f64) -> String {
        let id = format!("node_{}", self.nodes.len());
        let node = Node {
            id: id.clone(),
            x,
            y,
            text,
            width: 200.0,
            height: 100.0,
            color: "#3498db".to_string(),
        };
        self.nodes.insert(id.clone(), node);
        id
    }

    fn add_response_node(&mut self, parent_id: &str, response_text: String) {
        if let Some(parent) = self.nodes.get(parent_id) {
            let x = parent.x + parent.width + 50.0;
            let y = parent.y;
            let response_id = self.add_node(response_text, x, y);
            
            // In a more complex app, we would store connections between nodes here
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
                // Position the node in the left side of the canvas
                state.add_node(text.clone(), 50.0, 200.0);
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
    let canvas_mousedown = canvas.clone();
    let mousedown_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Check if we clicked on a node
            for (id, node) in state.nodes.iter() {
                if x >= node.x && x <= node.x + node.width &&
                   y >= node.y && y <= node.y + node.height {
                    state.dragging = Some(id.clone());
                    state.drag_offset_x = x - node.x;
                    state.drag_offset_y = y - node.y;
                    break;
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas.add_event_listener_with_callback(
        "mousedown",
        mousedown_handler.as_ref().unchecked_ref(),
    )?;
    mousedown_handler.forget();
    
    // Mouse move event
    let canvas_mousemove = canvas.clone();
    let mousemove_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            if let Some(id) = &state.dragging {
                if let Some(node) = state.nodes.get_mut(id) {
                    node.x = x - state.drag_offset_x;
                    node.y = y - state.drag_offset_y;
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
    let canvas_mouseup = canvas.clone();
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
    let ws = WebSocket::new("ws://localhost:8000/ws")?;
    
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
    let onerror_callback = Closure::wrap(Box::new(move |_| {
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
    let request_info = Object::new();
    let headers = Object::new();
    let body = Object::new();
    
    // Set up the fetch options
    js_sys::Reflect::set(&headers, &"Content-Type".into(), &"application/json".into()).unwrap();
    js_sys::Reflect::set(&request_info, &"method".into(), &"POST".into()).unwrap();
    js_sys::Reflect::set(&request_info, &"headers".into(), &headers).unwrap();
    
    // Set up the request body
    js_sys::Reflect::set(&body, &"text".into(), &text.into()).unwrap();
    let body_string = js_sys::JSON::stringify(&body).unwrap();
    js_sys::Reflect::set(&request_info, &"body".into(), &body_string).unwrap();
    
    // Make the fetch request
    let promise = window.fetch_with_str_and_init("http://localhost:8000/api/process-text", &request_info.into());
    
    // We're not handling the response here since we'll get it via WebSocket
    let _ = promise;
}

impl AppState {
    fn draw_nodes(&self) {
        if let (Some(canvas), Some(context)) = (&self.canvas, &self.context) {
            // Clear the canvas
            context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
            
            // Draw connections between nodes (if we had stored connections)
            // This would be the place to draw lines between nodes
            
            // Draw all nodes
            for (_, node) in &self.nodes {
                // Draw node rectangle
                context.set_fill_style(&JsValue::from_str(&node.color));
                context.fill_rect(node.x, node.y, node.width, node.height);
                
                // Draw node border
                context.set_stroke_style(&JsValue::from_str("#2c3e50"));
                context.stroke_rect(node.x, node.y, node.width, node.height);
                
                // Draw node text
                context.set_fill_style(&JsValue::from_str("#ffffff"));
                context.set_font("14px Arial");
                context.set_text_align("center");
                context.set_text_baseline("middle");
                
                // Handle text wrapping
                let max_width = node.width - 20.0;
                let words = node.text.split_whitespace().collect::<Vec<&str>>();
                let mut line = String::new();
                let mut y = node.y + 30.0;
                
                for word in words {
                    let test_line = if line.is_empty() {
                        word.to_string()
                    } else {
                        format!("{} {}", line, word)
                    };
                    
                    let metrics = context.measure_text(&test_line).unwrap();
                    
                    if metrics.width() > max_width && !line.is_empty() {
                        // Draw the current line and move to the next line
                        context.fill_text(&line, node.x + node.width / 2.0, y).unwrap();
                        line = word.to_string();
                        y += 20.0;
                    } else {
                        line = test_line;
                    }
                }
                
                // Draw the last line
                if !line.is_empty() {
                    context.fill_text(&line, node.x + node.width / 2.0, y).unwrap();
                }
            }
        }
    }
} 