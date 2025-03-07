use wasm_bindgen::prelude::*;
use web_sys::{
    Document, 
    HtmlCanvasElement, 
    MouseEvent,
};
use crate::state::APP_STATE;
use crate::models::NodeType;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

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
    
    // Setup animation loop for refreshing the canvas
    crate::ui::setup_animation_loop();
    
    Ok(())
}

pub fn resize_canvas(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
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
        
        // Update AppState with the new dimensions
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.canvas_width = container_width as f64;
            state.canvas_height = container_height as f64;
        });
        
        // Update viewport and redraw if necessary
        APP_STATE.with(|state| {
            let state = state.borrow();
            
            if let Some(context) = &state.context {
                // Reset transform first to avoid compounding scales
                let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
                
                // Apply the device pixel ratio scaling
                let _ = context.scale(dpr, dpr);
                
                // If auto-fit is enabled, refit the nodes
                if state.auto_fit && !state.nodes.is_empty() {
                    // We need to drop the immutable borrow to get a mutable one
                    drop(state);
                    
                    APP_STATE.with(|state| {
                        let mut state = state.borrow_mut();
                        state.fit_nodes_to_view();
                    });
                } else {
                    state.draw_nodes();
                }
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
    let _document = window.document().expect("should have a document");
    let _dpr = window.device_pixel_ratio();
    
    // Mouse down event
    let mousedown_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        // Get raw coordinates
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        // First determine what was clicked
        let (clicked_id, is_agent, offset_x, offset_y) = APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Check if we clicked on a node
            if let Some((id, offset_x, offset_y)) = state.find_node_at_position(x, y) {
                // Store the selected node ID
                state.selected_node_id = Some(id.clone());
                
                // Check if this node is an agent type
                let is_agent = if let Some(node) = state.nodes.get(&id) {
                    matches!(node.node_type, NodeType::AgentIdentity)
                } else {
                    false
                };
                
                // Return what was clicked with offsets
                (Some(id), is_agent, offset_x, offset_y)
            } else {
                // Nothing was clicked - prepare for canvas dragging
                state.selected_node_id = None;
                state.canvas_dragging = true;
                state.drag_start_x = x;
                state.drag_start_y = y;
                
                // Return empty values
                (None, false, 0.0, 0.0)
            }
        });
        
        // Check if a node was clicked
        if let Some(id) = clicked_id {
            // Check if right click
            if event.button() == 2 {
                // Right click handling (future: show context menu)
                return;
            }
            
            if is_agent {
                // Handle agent node click - get necessary data first
                let (node_text, system_instructions, history_data) = APP_STATE.with(|state| {
                    let state = state.borrow();
                    
                    if let Some(node) = state.nodes.get(&id) {
                        (
                            node.text.clone(),
                            node.system_instructions.clone().unwrap_or_default(),
                            node.history.clone().unwrap_or_default()
                        )
                    } else {
                        (String::new(), String::new(), Vec::new())
                    }
                });
                
                // Now safely manipulate the DOM without holding the borrow
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                
                // Set modal title
                if let Some(modal_title) = document.get_element_by_id("modal-title") {
                    modal_title.set_inner_html(&format!("Agent: {}", node_text));
                }
                
                // Load system instructions
                if let Some(system_elem) = document.get_element_by_id("system-instructions") {
                    if let Some(system_textarea) = system_elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                        system_textarea.set_value(&system_instructions);
                    }
                }
                
                // Load conversation history
                if let Some(history_container) = document.get_element_by_id("history-container") {
                    if history_data.is_empty() {
                        history_container.set_inner_html("<p>No history available.</p>");
                    } else {
                        // Clear existing history
                        history_container.set_inner_html("");
                        
                        // Add each message to the history container
                        for message in history_data {
                            if let Ok(message_elem) = document.create_element("div") {
                                message_elem.set_class_name(&format!("history-item {}", message.role));
                                
                                if let Ok(content) = document.create_element("p") {
                                    content.set_inner_html(&message.content);
                                    
                                    let _ = message_elem.append_child(&content);
                                    let _ = history_container.append_child(&message_elem);
                                }
                            }
                        }
                    }
                }
                
                // Show the modal
                if let Some(modal) = document.get_element_by_id("agent-modal") {
                    let _ = modal.set_attribute("style", "display: block;");
                }
            } else {
                // Handle regular node dragging
                APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.dragging = Some(id);
                    state.drag_offset_x = offset_x;
                    state.drag_offset_y = offset_y;
                    state.canvas_dragging = false; // Ensure canvas dragging is off
                });
            }
        }
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
                // Node dragging (existing behavior)
                let id = id.clone();
                let drag_offset_x = state.drag_offset_x;
                let drag_offset_y = state.drag_offset_y;
                
                // Apply viewport transformation to the mouse coordinates
                let world_x = x / state.zoom_level + state.viewport_x;
                let world_y = y / state.zoom_level + state.viewport_y;
                
                state.update_node_position(&id, world_x - drag_offset_x, world_y - drag_offset_y);
                
                // After updating the node position, save state
                let _ = state.save_if_modified();
            } else if state.canvas_dragging {
                // Canvas dragging (new behavior)
                // Calculate how far the mouse has moved since starting the drag
                let dx = (state.drag_start_x - x) / state.zoom_level;
                let dy = (state.drag_start_y - y) / state.zoom_level;
                
                // Update the viewport (moving it in the direction of the drag)
                state.viewport_x += dx;
                state.viewport_y += dy;
                
                // Enforce boundaries to prevent panning too far
                state.enforce_viewport_boundaries();
                
                // Update the drag start position for next movement
                state.drag_start_x = x;
                state.drag_start_y = y;
                
                // Redraw with the new viewport
                state.draw_nodes();
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
            state.canvas_dragging = false; // Clear canvas dragging state
            
            // Save state after completing the drag
            let _ = state.save_if_modified();
        });
    }) as Box<dyn FnMut(_)>);
    
    canvas_mouseup.add_event_listener_with_callback(
        "mouseup",
        mouseup_handler.as_ref().unchecked_ref(),
    )?;
    mouseup_handler.forget();
    
    // Add mouse wheel event for manual zooming when auto-fit is disabled
    let canvas_wheel = canvas.clone();
    // Create an additional clone for use inside the closure
    let canvas_wheel_inside = canvas_wheel.clone();
    
    let wheel_handler = Closure::wrap(Box::new(move |event: web_sys::WheelEvent| {
        // We won't prevent default since we want natural scrolling when not zooming
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Only allow manual zooming when auto-fit is disabled
            if !state.auto_fit {
                // Get canvas dimensions to use center as zoom point
                let canvas_width = canvas_wheel_inside.width() as f64;
                let canvas_height = canvas_wheel_inside.height() as f64;
                let window = web_sys::window().expect("no global window exists");
                let dpr = window.device_pixel_ratio();
                
                // Get center of canvas in screen coordinates
                let x = canvas_width / (2.0 * dpr);
                let y = canvas_height / (2.0 * dpr);
                
                // Convert to world coordinates
                let world_x = x / state.zoom_level + state.viewport_x;
                let world_y = y / state.zoom_level + state.viewport_y;
                
                // Get wheel delta
                let delta_y = event.delta_y();
                
                // Adjust zoom based on wheel direction
                let zoom_delta = if delta_y > 0.0 { 0.9 } else { 1.1 };
                state.zoom_level *= zoom_delta;
                
                // Limit zoom to reasonable values
                state.zoom_level = f64::max(0.1, f64::min(state.zoom_level, 5.0));
                
                // Adjust viewport to zoom toward/away from center
                state.viewport_x = world_x - x / state.zoom_level;
                state.viewport_y = world_y - y / state.zoom_level;
                
                state.draw_nodes();
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    // Add wheel event listener with standard method
    canvas_wheel.add_event_listener_with_callback(
        "wheel",
        wheel_handler.as_ref().unchecked_ref(),
    )?;
    wheel_handler.forget();
    
    Ok(())
}
