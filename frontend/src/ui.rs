use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlCanvasElement, HtmlInputElement, MouseEvent};
use crate::state::APP_STATE;
use crate::models::NodeType;
use crate::network::send_text_to_backend;

pub fn setup_ui(document: &Document) -> Result<(), JsValue> {
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
    canvas.set_attribute("style", "position: absolute; top: 0; left: 0; width: 100%; height: 100%;")?;
    
    canvas_container.append_child(&canvas)?;
    
    // Add everything to the document body
    let app_container = document.create_element("div")?;
    app_container.set_attribute("style", "padding: 20px;")?;
    app_container.append_child(&input_container)?;
    app_container.append_child(&canvas_container)?;
    
    document.body().unwrap().append_child(&app_container)?;
    
    // Set up event listener for the button
    setup_button_click_handler(document)?;
    
    Ok(())
}

fn setup_button_click_handler(document: &Document) -> Result<(), JsValue> {
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

pub fn setup_canvas(document: &Document) -> Result<(), JsValue> {
    let canvas = document.get_element_by_id("node-canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;
    
    // Set canvas dimensions to match container
    resize_canvas(&canvas)?;
    
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    
    // Ensure initial context is properly scaled with device pixel ratio
    let window = web_sys::window().expect("no global window exists");
    let dpr = window.device_pixel_ratio();
    let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    let _ = context.scale(dpr, dpr);
    
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.canvas = Some(canvas.clone());
        state.context = Some(context);
    });
    
    // Set up mouse events for the canvas
    setup_canvas_mouse_events(&canvas)?;
    
    // Set up resize handler
    setup_resize_handler(&canvas)?;
    
    Ok(())
}

fn resize_canvas(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    // Get the parent container dimensions
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    if let Some(container) = document.get_element_by_id("canvas-container") {
        let container_width = container.client_width();
        let container_height = container.client_height();
        
        // Get the device pixel ratio for high-DPI displays
        let dpr = window.device_pixel_ratio();
        
        // Set the canvas width and height attributes to the container size times the pixel ratio
        let scaled_width = (container_width as f64 * dpr) as u32;
        let scaled_height = (container_height as f64 * dpr) as u32;
        
        // Set the actual canvas bitmap size
        canvas.set_width(scaled_width);
        canvas.set_height(scaled_height);
        
        // Set CSS size to maintain visual dimensions
        canvas.style().set_property("width", &format!("{}px", container_width))?;
        canvas.style().set_property("height", &format!("{}px", container_height))?;
        
        // Scale the context to account for the pixel ratio
        APP_STATE.with(|state| {
            let state = state.borrow();
            if let Some(context) = &state.context {
                // Reset transform first to avoid compounding scales
                let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                // Apply the device pixel ratio scaling
                let _ = context.scale(dpr, dpr);
                state.draw_nodes();
            }
        });
    }
    
    Ok(())
}

fn setup_resize_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let canvas_clone = canvas.clone();
    let resize_callback = Closure::wrap(Box::new(move || {
        let _ = resize_canvas(&canvas_clone);
    }) as Box<dyn FnMut()>);
    
    // Add window resize event listener
    web_sys::window()
        .expect("no global window exists")
        .add_event_listener_with_callback(
            "resize",
            resize_callback.as_ref().unchecked_ref(),
        )?;
    
    // Leak the closure to keep it alive for the lifetime of the application
    resize_callback.forget();
    
    Ok(())
}

fn setup_canvas_mouse_events(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    // Get device pixel ratio once for all handlers
    let window = web_sys::window().expect("no global window exists");
    let dpr = window.device_pixel_ratio();
    
    // Mouse down event
    let mousedown_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        // Get raw coordinates
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Find which node was clicked on
            if let Some((id, offset_x, offset_y)) = state.find_node_at_position(x, y) {
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
    let canvas_mousemove = canvas.clone();
    let mousemove_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            if let Some(id) = &state.dragging {
                let id = id.clone();
                let drag_offset_x = state.drag_offset_x;
                let drag_offset_y = state.drag_offset_y;
                
                state.update_node_position(&id, x - drag_offset_x, y - drag_offset_y);
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas_mousemove.add_event_listener_with_callback(
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
    
    canvas_mouseup.add_event_listener_with_callback(
        "mouseup",
        mouseup_handler.as_ref().unchecked_ref(),
    )?;
    mouseup_handler.forget();
    
    Ok(())
} 