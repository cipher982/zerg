use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlCanvasElement, HtmlInputElement, MouseEvent};
use crate::state::APP_STATE;
use crate::models::NodeType;
use crate::network::{send_text_to_backend, fetch_available_models};
use wasm_bindgen_futures::spawn_local;

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
    
    // Create model selection dropdown
    let model_select = document.create_element("select")?;
    model_select.set_attribute("id", "model-select")?;
    model_select.set_attribute("style", "padding: 8px; margin-left: 10px;")?;
    
    // Initially populate with default models
    update_model_dropdown(document)?;
    
    // Create auto-fit toggle button
    let auto_fit_button = document.create_element("button")?;
    auto_fit_button.set_inner_html("Auto-Fit: ON");
    auto_fit_button.set_attribute("id", "auto-fit-button")?;
    auto_fit_button.set_attribute("style", "padding: 8px 16px; background-color: #3498db; color: white; border: none; cursor: pointer; margin-left: 10px;")?;
    
    // Create input container
    let input_container = document.create_element("div")?;
    input_container.set_attribute("style", "margin-bottom: 20px;")?;
    input_container.append_child(&input)?;
    input_container.append_child(&button)?;
    input_container.append_child(&model_select)?;
    input_container.append_child(&auto_fit_button)?;
    
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
    
    // Set up event handlers
    setup_button_click_handler(document)?;
    setup_input_keypress_handler(document)?;
    setup_auto_fit_toggle_handler(document)?;
    setup_model_select_handler(document)?;
    
    // Fetch available models from the backend
    fetch_models_from_backend(document)?;
    
    Ok(())
}

fn update_model_dropdown(document: &Document) -> Result<(), JsValue> {
    if let Some(select_el) = document.get_element_by_id("model-select") {
        // Clear existing options
        select_el.set_inner_html("");
        
        // Get available models from state
        let models = APP_STATE.with(|state| {
            let state = state.borrow();
            state.available_models.clone()
        });
        
        // Get selected model
        let selected_model = APP_STATE.with(|state| {
            let state = state.borrow();
            state.selected_model.clone()
        });
        
        // Add options to dropdown
        for (value, label) in models.iter() {
            let option = document.create_element("option")?;
            option.set_attribute("value", value)?;
            
            // Set selected if it matches current selection
            if value == &selected_model {
                option.set_attribute("selected", "selected")?;
            }
            
            option.set_inner_html(label);
            select_el.append_child(&option)?;
        }
    }
    
    Ok(())
}

fn fetch_models_from_backend(document: &Document) -> Result<(), JsValue> {
    let document_clone = document.clone();
    
    // Fetch models asynchronously
    spawn_local(async move {
        if let Ok(()) = fetch_available_models().await {
            // Update the dropdown with fetched models
            let _ = update_model_dropdown(&document_clone);
        }
    });
    
    Ok(())
}

fn setup_button_click_handler(document: &Document) -> Result<(), JsValue> {
    let button_el = document.get_element_by_id("send-button").unwrap();
    let input_el = document.get_element_by_id("user-input").unwrap();
    
    let button_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
        let input = input_el.dyn_ref::<HtmlInputElement>().unwrap();
        let text = input.value();
        
        if !text.is_empty() {
            send_user_input(&text);
            
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

fn setup_input_keypress_handler(document: &Document) -> Result<(), JsValue> {
    let input_el = document.get_element_by_id("user-input").unwrap();
    let input_el_clone = input_el.clone();
    
    let keypress_handler = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Enter" {
            let input = input_el_clone.dyn_ref::<HtmlInputElement>().unwrap();
            let text = input.value();
            
            if !text.is_empty() {
                send_user_input(&text);
                
                // Clear the input field
                input.set_value("");
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    input_el.add_event_listener_with_callback(
        "keypress",
        keypress_handler.as_ref().unchecked_ref(),
    )?;
    keypress_handler.forget();
    
    Ok(())
}

fn send_user_input(text: &str) {
    // Add a node for the user's input
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.input_text = text.to_string();
        
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
        
        state.add_node(text.to_string(), 50.0, y_position, NodeType::UserInput);
        state.draw_nodes();
    });
    
    // Send the text to the backend
    send_text_to_backend(text);
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
    let _dpr = window.device_pixel_ratio();
    
    // Mouse down event
    let mousedown_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        // Get raw coordinates
        let x = event.offset_x() as f64;
        let y = event.offset_y() as f64;
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Find which node was clicked on, accounting for viewport transformation
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
                
                // Apply viewport transformation to the mouse coordinates
                let world_x = x / state.zoom_level + state.viewport_x;
                let world_y = y / state.zoom_level + state.viewport_y;
                
                state.update_node_position(&id, world_x - drag_offset_x, world_y - drag_offset_y);
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
    
    // Add mouse wheel event for manual zooming when auto-fit is disabled
    let canvas_wheel = canvas.clone();
    // Create an additional clone for use inside the closure
    let canvas_wheel_inside = canvas_wheel.clone();
    
    let wheel_handler = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
        let event_obj = event.clone();
        
        // Prevent default scrolling behavior
        event.prevent_default();
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Only allow manual zooming when auto-fit is disabled
            if !state.auto_fit {
                // Get canvas dimensions to use center as zoom point (simpler approach)
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
                
                // Get wheel delta using JavaScript property access
                let delta_y = js_sys::Reflect::get(&event_obj, &"deltaY".into())
                    .unwrap_or(js_sys::Reflect::get(&event_obj, &"wheelDelta".into()).unwrap_or(0.0.into()))
                    .as_f64()
                    .unwrap_or(0.0);
                
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
        
        // Explicitly prevent default again to be extra safe
        let _ = js_sys::Reflect::set(
            &event,
            &"returnValue".into(),
            &false.into()
        );
    }) as Box<dyn FnMut(_)>);
    
    // Use both wheel and mousewheel for cross-browser compatibility
    canvas_wheel.add_event_listener_with_callback(
        "wheel",
        wheel_handler.as_ref().unchecked_ref(),
    )?;
    wheel_handler.forget();
    
    Ok(())
}

// Add this new function to handle auto-fit toggle
fn setup_auto_fit_toggle_handler(document: &Document) -> Result<(), JsValue> {
    let auto_fit_button = document.get_element_by_id("auto-fit-button").unwrap();
    
    let click_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.toggle_auto_fit();
            
            // Update button text
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("no document exists");
            if let Some(button) = document.get_element_by_id("auto-fit-button") {
                let status = if state.auto_fit { "ON" } else { "OFF" };
                button.set_inner_html(&format!("Auto-Fit: {}", status));
                
                // Update button color
                let color = if state.auto_fit { "#3498db" } else { "#95a5a6" };
                button.set_attribute("style", &format!(
                    "padding: 8px 16px; background-color: {}; color: white; border: none; cursor: pointer; margin-left: 10px;",
                    color
                )).unwrap();
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    auto_fit_button.add_event_listener_with_callback(
        "click",
        click_callback.as_ref().unchecked_ref(),
    )?;
    click_callback.forget();
    
    Ok(())
}

fn setup_model_select_handler(document: &Document) -> Result<(), JsValue> {
    let select_el = document.get_element_by_id("model-select").unwrap();
    
    let change_handler = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let select = event.target().unwrap().dyn_into::<web_sys::HtmlSelectElement>().unwrap();
        let model = select.value();
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.selected_model = model;
        });
    }) as Box<dyn FnMut(_)>);
    
    select_el.add_event_listener_with_callback(
        "change",
        change_handler.as_ref().unchecked_ref(),
    )?;
    change_handler.forget();
    
    Ok(())
} 