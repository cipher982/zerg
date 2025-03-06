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
    
    // Create toggle switch container
    let toggle_container = document.create_element("div")?;
    toggle_container.set_attribute("style", "display: inline-flex; align-items: center; margin-left: 10px;")?;
    
    // Create label for toggle with stacked text
    let toggle_label = document.create_element("div")?;
    toggle_label.set_inner_html("Auto Layout");
    toggle_label.set_attribute("style", "margin-right: 8px; font-size: 0.9em; text-align: center; line-height: 1.2;")?;
    
    // Create toggle switch track
    let toggle_track = document.create_element("label")?;
    let _ = toggle_track.set_attribute("class", "switch");
    toggle_track.set_attribute("style", "position: relative; display: inline-block; width: 50px; height: 24px;")?;
    
    // Create hidden checkbox that stores the actual state
    let toggle_checkbox = document.create_element("input")?;
    toggle_checkbox.set_attribute("type", "checkbox")?;
    toggle_checkbox.set_attribute("id", "auto-fit-checkbox")?;
    toggle_checkbox.set_attribute("checked", "")?; // On by default
    toggle_checkbox.set_attribute("style", "opacity: 0; width: 0; height: 0;")?;
    
    // Create the visual slider with faster transition
    let toggle_slider = document.create_element("span")?;
    let _ = toggle_slider.set_attribute("class", "slider");
    toggle_slider.set_attribute("style", "position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #3498db; border-radius: 24px; transition: .2s;")?;
    
    // Create the circle that moves with faster transition
    let toggle_circle = document.create_element("span")?;
    let _ = toggle_circle.set_attribute("class", "circle");
    toggle_circle.set_attribute("style", "position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; border-radius: 50%; transition: .2s; transform: translateX(26px);")?;
    
    // Assemble the toggle
    toggle_slider.append_child(&toggle_circle)?;
    toggle_track.append_child(&toggle_checkbox)?;
    toggle_track.append_child(&toggle_slider)?;
    toggle_container.append_child(&toggle_label)?;
    toggle_container.append_child(&toggle_track)?;
    
    // Create center view button
    let center_button = document.create_element("button")?;
    center_button.set_inner_html("Center View");
    center_button.set_attribute("id", "center-button")?;
    center_button.set_attribute("style", "padding: 8px 16px; background-color: #2980b9; color: white; border: none; cursor: pointer; margin-left: 10px; border-radius: 4px;")?;
    
    // Create clear all button
    let clear_button = document.create_element("button")?;
    clear_button.set_inner_html("Clear All");
    clear_button.set_attribute("id", "clear-button")?;
    clear_button.set_attribute("style", "padding: 8px 16px; background-color: #e74c3c; color: white; border: none; cursor: pointer; margin-left: 10px; border-radius: 4px;")?;
    
    // Create input container
    let input_container = document.create_element("div")?;
    input_container.set_attribute("style", "margin-bottom: 20px;")?;
    input_container.append_child(&input)?;
    input_container.append_child(&button)?;
    input_container.append_child(&model_select)?;
    input_container.append_child(&toggle_container)?;
    input_container.append_child(&center_button)?;
    input_container.append_child(&clear_button)?;
    
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
    setup_center_view_handler(document)?;
    setup_model_select_handler(document)?;
    setup_clear_button_handler(document)?;
    
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
        
        // Save state after adding the node
        if let Err(e) = state.save_if_modified() {
            web_sys::console::error_1(&format!("Failed to save state: {:?}", e).into());
        }
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
                // Node dragging (existing behavior)
                state.dragging = Some(id);
                state.drag_offset_x = offset_x;
                state.drag_offset_y = offset_y;
                state.canvas_dragging = false; // Ensure canvas dragging is off
            } else {
                // Canvas dragging - clicked on empty area
                // If in Auto Layout Mode, automatically switch to Manual Layout Mode
                if state.auto_fit {
                    state.auto_fit = false;
                    
                    // Update the toggle switch to reflect the mode change
                    let window = web_sys::window().expect("no global window exists");
                    let document = window.document().expect("no document exists");
                    
                    // Update the checkbox state
                    if let Some(checkbox) = document.get_element_by_id("auto-fit-checkbox") {
                        if let Some(input) = checkbox.dyn_ref::<web_sys::HtmlInputElement>() {
                            input.set_checked(false);
                        }
                    }
                    
                    // Update the visual appearance
                    if let Some(slider) = document.query_selector(".slider").ok().flatten() {
                        let _ = slider.set_attribute("style", 
                            "position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: #95a5a6; border-radius: 24px; transition: .2s;"
                        );
                    }
                    
                    if let Some(circle) = document.query_selector(".circle").ok().flatten() {
                        let _ = circle.set_attribute("style", 
                            "position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; border-radius: 50%; transition: .2s; transform: translateX(0);"
                        );
                    }
                }
                
                // Enable canvas dragging
                state.dragging = None;
                state.canvas_dragging = true;
                state.canvas_drag_start_x = x;
                state.canvas_drag_start_y = y;
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
                let dx = (state.canvas_drag_start_x - x) / state.zoom_level;
                let dy = (state.canvas_drag_start_y - y) / state.zoom_level;
                
                // Update the viewport (moving it in the direction of the drag)
                state.viewport_x += dx;
                state.viewport_y += dy;
                
                // Enforce boundaries to prevent panning too far
                state.enforce_viewport_boundaries();
                
                // Update the drag start position for next movement
                state.canvas_drag_start_x = x;
                state.canvas_drag_start_y = y;
                
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
    let toggle_checkbox = document.get_element_by_id("auto-fit-checkbox").unwrap();
    
    let change_handler = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let checkbox = event.target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap();
        let checked = checkbox.checked();
        
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Only toggle if the current state is different from what we want
            if state.auto_fit != checked {
                state.toggle_auto_fit();
            }
            
            // Get the toggle slider to update its appearance
            let window = web_sys::window().expect("no global window exists");
            let document = window.document().expect("no document exists");
            if let Some(slider) = document.query_selector(".slider").ok().flatten() {
                let bg_color = if checked { "#3498db" } else { "#95a5a6" };
                let _ = slider.set_attribute("style", &format!(
                    "position: absolute; cursor: pointer; top: 0; left: 0; right: 0; bottom: 0; background-color: {}; border-radius: 24px; transition: .2s;",
                    bg_color
                ));
            }
            
            if let Some(circle) = document.query_selector(".circle").ok().flatten() {
                let transform = if checked { "translateX(26px)" } else { "translateX(0)" };
                let _ = circle.set_attribute("style", &format!(
                    "position: absolute; height: 18px; width: 18px; left: 3px; bottom: 3px; background-color: white; border-radius: 50%; transition: .2s; transform: {};",
                    transform
                ));
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    toggle_checkbox.add_event_listener_with_callback(
        "change",
        change_handler.as_ref().unchecked_ref(),
    )?;
    change_handler.forget();
    
    Ok(())
}

fn setup_center_view_handler(document: &Document) -> Result<(), JsValue> {
    let center_button = document.get_element_by_id("center-button").unwrap();
    
    let click_callback = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.center_view();
        });
    }) as Box<dyn FnMut(_)>);
    
    center_button.add_event_listener_with_callback(
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
            state.state_modified = true; // Mark as modified
            let _ = state.save_if_modified();
        });
    }) as Box<dyn FnMut(_)>);
    
    select_el.add_event_listener_with_callback(
        "change",
        change_handler.as_ref().unchecked_ref(),
    )?;
    change_handler.forget();
    
    Ok(())
}

// Add function for clear button handler
fn setup_clear_button_handler(document: &Document) -> Result<(), JsValue> {
    let clear_button = document.get_element_by_id("clear-button")
        .ok_or_else(|| JsValue::from_str("Clear button not found"))?;
    
    let click_callback = Closure::wrap(Box::new(move |_e: MouseEvent| {
        // Show confirmation dialog
        let window = web_sys::window().expect("no global window exists");
        let confirm = window.confirm_with_message("Are you sure you want to clear all nodes? This cannot be undone.")
            .unwrap_or(false);
        
        if confirm {
            // Clear state and storage
            APP_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.nodes.clear();
                state.latest_user_input_id = None;
                state.message_id_to_node_id.clear();
                state.viewport_x = 0.0;
                state.viewport_y = 0.0;
                state.zoom_level = 1.0;
                state.auto_fit = true;
                state.state_modified = true;
                
                state.draw_nodes();
                
                // Clear storage
                if let Err(e) = crate::storage::clear_storage() {
                    web_sys::console::error_1(&format!("Failed to clear storage: {:?}", e).into());
                }
                
                web_sys::console::log_1(&"Canvas cleared and storage reset".into());
            });
        }
    }) as Box<dyn FnMut(_)>);
    
    clear_button.add_event_listener_with_callback("click", click_callback.as_ref().unchecked_ref())?;
    click_callback.forget();
    
    Ok(())
} 