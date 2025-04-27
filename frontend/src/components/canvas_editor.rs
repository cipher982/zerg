use wasm_bindgen::prelude::*;
use web_sys::{
    Document, 
    HtmlCanvasElement, 
    MouseEvent,
    AddEventListenerOptions,
};
use crate::state::{APP_STATE, AppState};
use crate::models::NodeType;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use crate::messages::Command;
use crate::state::dispatch_global_message;

// New type to represent either a mutable or immutable reference to AppState
pub enum AppStateRef<'a> {
    #[allow(dead_code)]
    Mutable(&'a mut AppState),
    Immutable(&'a AppState),
    None
}

pub fn setup_canvas(document: &Document) -> Result<(), JsValue> {
    let canvas = document.get_element_by_id("node-canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;
    
    // Set canvas dimensions to match container
    resize_canvas(&canvas, AppStateRef::None)?;
    
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

pub fn resize_canvas(canvas: &HtmlCanvasElement, mut app_state_ref: AppStateRef) -> Result<(), JsValue> {
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
        
        // Update dimensions and check auto_fit setting based on the reference type
        let (auto_fit, has_nodes, context) = match app_state_ref {
            AppStateRef::Mutable(ref mut state) => {
                // Can update state
                state.canvas_width = container_width as f64;
                state.canvas_height = container_height as f64;
                (state.auto_fit, !state.nodes.is_empty(), state.context.as_ref().cloned())
            },
            AppStateRef::Immutable(ref state) => {
                // Read only - these dimensions won't be saved in state
                // but that's okay for rendering purposes
                (state.auto_fit, !state.nodes.is_empty(), state.context.as_ref().cloned())
            },
            AppStateRef::None => APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.canvas_width = container_width as f64;
                state.canvas_height = container_height as f64;
                (state.auto_fit, !state.nodes.is_empty(), state.context.as_ref().cloned())
            })
        };
        
        // Apply the canvas transformations if we have a context
        if let Some(context) = context {
            // Reset transform first to avoid compounding scales
            let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            
            // Apply the device pixel ratio scaling
            let _ = context.scale(dpr, dpr);
        }
        
        // Handle auto-fit if needed based on the reference type
        if auto_fit && has_nodes {
            match app_state_ref {
                AppStateRef::Mutable(ref mut state) => {
                    state.fit_nodes_to_view();
                },
                AppStateRef::Immutable(ref state) => {
                    // Can't fit nodes in read-only mode,
                    // but we can still draw them as they are
                    state.draw_nodes();
                },
                AppStateRef::None => APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.fit_nodes_to_view();
                })
            }
        } else {
            // Just redraw without auto-fit
            match app_state_ref {
                AppStateRef::Mutable(ref state) => {
                    state.draw_nodes();
                },
                AppStateRef::Immutable(ref state) => {
                    state.draw_nodes();
                },
                AppStateRef::None => APP_STATE.with(|state| {
                    let state = state.borrow();
                    state.draw_nodes();
                })
            }
        }
    }
    
    Ok(())
}

fn setup_resize_handler(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let _window = web_sys::window().expect("no global window exists");
    
    // Set up resize handler
    let canvas_clone = canvas.clone();
    let resize_callback = Closure::wrap(Box::new(move || {
        let _ = resize_canvas(&canvas_clone, AppStateRef::None);
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
        let clicked_result = {
            APP_STATE.with(|state| {
                let state = state.borrow();
                state.find_node_at_position(x, y)
            })
        };
        
        // Based on the result, dispatch the appropriate message
        if let Some((node_id, offset_x, offset_y)) = clicked_result {
            // Check if this node is an agent type
            let is_agent = APP_STATE.with(|state| {
                let state = state.borrow();
                if let Some(node) = state.nodes.get(&node_id) {
                    matches!(node.node_type, NodeType::AgentIdentity)
                } else {
                    false
                }
            });
            
            // Check if right click
            if event.button() == 2 {
                // Right click handling (future: show context menu)
                return;
            }
            
            // Check if auto-fit is enabled
            let auto_fit_enabled = APP_STATE.with(|state| {
                let state = state.borrow();
                state.auto_fit
            });
            
            // If auto-fit is enabled, toggle it off
            if auto_fit_enabled {
                let commands = APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.dispatch(crate::messages::Message::ToggleAutoFit)
                });
                
                // Execute commands
                for cmd in commands {
                    match cmd {
                        Command::SendMessage(msg) => dispatch_global_message(msg),
                        Command::NoOp => {},
                        _ => {},  // Ignore other commands in this context
                    }
                }

                // Update the auto-fit toggle button
                let auto_fit_toggle = window.document()
                    .expect("should have a document")
                    .get_element_by_id("auto-fit-toggle");
                
                if let Some(toggle) = auto_fit_toggle {
                    // Cast to HTMLInputElement to set checked property directly
                    if let Some(toggle_input) = toggle.dyn_ref::<web_sys::HtmlInputElement>() {
                        toggle_input.set_checked(false);
                    }
                }
            }
            
            // Dispatch StartDragging message
            let commands = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let commands = state.dispatch(crate::messages::Message::StartDragging {
                    node_id: node_id.clone(),
                    offset_x,
                    offset_y,
                });
                
                // If this is an agent node, store additional information for the mouseup handler
                if is_agent {
                    state.is_dragging_agent = true;
                    state.drag_start_x = x; // Store start position to determine if it was a click or drag
                    state.drag_start_y = y;
                }
                
                commands
            });
            
            // Execute commands
            for cmd in commands {
                match cmd {
                    Command::SendMessage(msg) => dispatch_global_message(msg),
                    Command::NoOp => {},
                    _ => {},  // Ignore other commands in this context
                }
            }
        } else {
            // Nothing was clicked - prepare for canvas dragging
            // Dispatch StartCanvasDrag message
            let commands = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(crate::messages::Message::StartCanvasDrag {
                    start_x: x,
                    start_y: y,
                })
            });
            
            // Execute commands
            for cmd in commands {
                match cmd {
                    Command::SendMessage(msg) => dispatch_global_message(msg),
                    Command::NoOp => {},
                    _ => {},  // Ignore other commands in this context
                }
            }
            
            // After dispatching StartCanvasDrag, check if we need to toggle auto-fit
            let auto_fit_enabled = APP_STATE.with(|state| {
                let state = state.borrow();
                state.auto_fit
            });
            
            // If in Auto Layout Mode, automatically switch to Manual Layout Mode
            if auto_fit_enabled {
                let commands = APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.dispatch(crate::messages::Message::ToggleAutoFit)
                });
                
                // Execute commands
                for cmd in commands {
                    match cmd {
                        Command::SendMessage(msg) => dispatch_global_message(msg),
                        Command::NoOp => {},
                        _ => {},  // Ignore other commands in this context
                    }
                }
                
                // Update the auto-fit toggle button
                let auto_fit_toggle = window.document()
                    .expect("should have a document")
                    .get_element_by_id("auto-fit-toggle");
                
                if let Some(toggle) = auto_fit_toggle {
                    // Cast to HTMLInputElement to set checked property directly
                    if let Some(toggle_input) = toggle.dyn_ref::<web_sys::HtmlInputElement>() {
                        toggle_input.set_checked(false);
                    }
                }
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
        
        // Check if we're dragging a node or the canvas
        let dragging_type = APP_STATE.with(|state| {
            let state = state.borrow();
            if state.dragging.is_some() {
                "node"
            } else if state.canvas_dragging {
                "canvas"
            } else {
                "none"
            }
        });
        
        match dragging_type {
            "node" => {
                // Get the node id and offset from state
                let (node_id, drag_offset_x, drag_offset_y, zoom_level, viewport_x, viewport_y) = APP_STATE.with(|state| {
                    let state = state.borrow();
                    let node_id = state.dragging.clone().unwrap();
                    (
                        node_id,
                        state.drag_offset_x,
                        state.drag_offset_y,
                        state.zoom_level,
                        state.viewport_x,
                        state.viewport_y
                    )
                });
                
                // Apply viewport transformation to get world coordinates
                let world_x = x / zoom_level + viewport_x;
                let world_y = y / zoom_level + viewport_y;
                
                // Dispatch UpdateNodePosition message
                let commands = APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.dispatch(crate::messages::Message::UpdateNodePosition {
                        node_id: node_id,
                        x: world_x - drag_offset_x,
                        y: world_y - drag_offset_y,
                    })
                });
                
                // Execute commands
                for cmd in commands {
                    match cmd {
                        Command::SendMessage(msg) => dispatch_global_message(msg),
                        Command::NoOp => {},
                        _ => {},  // Ignore other commands in this context
                    }
                }
            },
            "canvas" => {
                // Dispatch UpdateCanvasDrag message
                let commands = APP_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.dispatch(crate::messages::Message::UpdateCanvasDrag {
                        current_x: x,
                        current_y: y,
                    })
                });
                
                // Execute commands
                for cmd in commands {
                    match cmd {
                        Command::SendMessage(msg) => dispatch_global_message(msg),
                        Command::NoOp => {},
                        _ => {},  // Ignore other commands in this context
                    }
                }
            },
            _ => {
                // Not dragging anything
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    canvas_mousemove.add_event_listener_with_callback(
        "mousemove",
        mousemove_handler.as_ref().unchecked_ref(),
    )?;
    mousemove_handler.forget();
    
    // Mouse up event
    let canvas_mouseup = canvas.clone();
    let mouseup_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        // Check if we were dragging an agent node and if it was a click (not much movement)
        let (was_dragging_agent, selected_node_id, was_click) = APP_STATE.with(|state| {
            let state = state.borrow();
            
            let was_dragging_agent = state.is_dragging_agent;
            let selected_node_id = state.selected_node_id.clone();
            
            // Determine if this was a click or drag by checking distance
            let was_click = if was_dragging_agent {
                let dx = x - state.drag_start_x;
                let dy = y - state.drag_start_y;
                let distance_squared = dx * dx + dy * dy;
                
                // If the mouse didn't move much, consider it a click (using a small threshold)
                distance_squared < 25.0 // 5px threshold
            } else {
                false
            };
            
            (was_dragging_agent, selected_node_id, was_click)
        });
        
        // Stop any dragging operation
        if APP_STATE.with(|state| { state.borrow().dragging.is_some() }) {
            // We were dragging a node, stop dragging
            let commands = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(crate::messages::Message::StopDragging)
            });
            
            // Execute commands
            for cmd in commands {
                match cmd {
                    Command::SendMessage(msg) => dispatch_global_message(msg),
                    Command::NoOp => {},
                    _ => {},  // Ignore other commands in this context
                }
            }
        } else if APP_STATE.with(|state| { state.borrow().canvas_dragging }) {
            // We were dragging the canvas, stop canvas drag
            let commands = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(crate::messages::Message::StopCanvasDrag)
            });
            
            // Execute commands
            for cmd in commands {
                match cmd {
                    Command::SendMessage(msg) => dispatch_global_message(msg),
                    Command::NoOp => {},
                    _ => {},  // Ignore other commands in this context
                }
            }
        }
        
        // Clear is_dragging_agent flag (this isn't part of any message yet)
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.is_dragging_agent = false;
        });
        
        // Refresh the dashboard
        let _ = refresh_dashboard_after_change();
        
        // If this was a click on an agent node (not a drag), open the modal
        if was_dragging_agent && was_click {
            if let Some(node_id) = selected_node_id {
                // Open the agent modal
                let window = web_sys::window().expect("no global window exists");
                let document = window.document().expect("should have a document");
                
                let _ = crate::components::agent_config_modal::AgentConfigModal::open(&document, &node_id);
            }
        }
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
        // Check if auto-fit is enabled before doing anything
        let auto_fit_enabled = APP_STATE.with(|state| {
            let state = state.borrow();
            state.auto_fit
        });
        
        // Only process the event when auto-fit is disabled, never call preventDefault()
        if !auto_fit_enabled {
            // Rest of the zoom handling code
            let (canvas_width, canvas_height, zoom_level, viewport_x, viewport_y) = APP_STATE.with(|state| {
                let state = state.borrow();
                (
                    canvas_wheel_inside.width() as f64,
                    canvas_wheel_inside.height() as f64,
                    state.zoom_level,
                    state.viewport_x,
                    state.viewport_y
                )
            });
            
            let window = web_sys::window().expect("no global window exists");
            let dpr = window.device_pixel_ratio();
            
            // Get center of canvas in screen coordinates
            let x = canvas_width / (2.0 * dpr);
            let y = canvas_height / (2.0 * dpr);
            
            // Convert to world coordinates
            let world_x = x / zoom_level + viewport_x;
            let world_y = y / zoom_level + viewport_y;
            
            // Get wheel delta
            let delta_y = event.delta_y();
            
            // Calculate new zoom level
            let zoom_delta = if delta_y > 0.0 { 0.9 } else { 1.1 };
            let new_zoom = zoom_level * zoom_delta;
            
            // Limit zoom to reasonable values
            let new_zoom = f64::max(0.1, f64::min(new_zoom, 5.0));
            
            // Calculate new viewport based on the zoom
            let new_viewport_x = world_x - x / new_zoom;
            let new_viewport_y = world_y - y / new_zoom;
            
            // Dispatch ZoomCanvas message
            let commands = APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.dispatch(crate::messages::Message::ZoomCanvas {
                    new_zoom,
                    viewport_x: new_viewport_x,
                    viewport_y: new_viewport_y,
                })
            });
            
            // Execute commands
            for cmd in commands {
                match cmd {
                    Command::SendMessage(msg) => dispatch_global_message(msg),
                    Command::NoOp => {},
                    _ => {},  // Ignore other commands in this context
                }
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    // Create options with passive: true
    let options = AddEventListenerOptions::new();
    
    // Add wheel event listener with passive option
    canvas_wheel.add_event_listener_with_callback_and_add_event_listener_options(
        "wheel",
        wheel_handler.as_ref().unchecked_ref(),
        &options,
    )?;
    wheel_handler.forget();
    
    Ok(())
}

// Helper function to refresh dashboard after node modifications
fn refresh_dashboard_after_change() -> Result<(), JsValue> {
    // First save the state in its own borrow scope
    {
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            let _ = state.save_if_modified();
        });
    }
    
    // Then refresh UI after state changes (in a completely separate borrow)
    crate::state::AppState::refresh_ui_after_state_change()
}
