use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, HtmlElement, MouseEvent};

// Global state for drag tracking
struct DragState {
    dragging: bool,
    offset_x: i32,
    offset_y: i32,
}

// We use thread_local because WASM runs in a single thread
thread_local! {
    static DRAG_STATE: std::cell::RefCell<DragState> = std::cell::RefCell::new(DragState {
        dragging: false,
        offset_x: 0,
        offset_y: 0,
    });
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    // Get the document
    let window = web_sys::window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    // Create our box
    create_box(&document)?;
    
    Ok(())
}

fn create_box(document: &Document) -> Result<(), JsValue> {
    // Create a div element for our box
    let box_element = document.create_element("div")?;
    box_element.set_id("draggable-box");
    
    // Style the box
    let box_style = box_element.dyn_ref::<HtmlElement>().unwrap().style();
    box_style.set_property("width", "100px")?;
    box_style.set_property("height", "100px")?;
    box_style.set_property("background-color", "blue")?;
    box_style.set_property("position", "absolute")?;
    box_style.set_property("top", "100px")?;
    box_style.set_property("left", "100px")?;
    box_style.set_property("cursor", "move")?;
    box_style.set_property("user-select", "none")?;
    
    // Set up mouse down event
    let mouse_down_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        let target = event.target().unwrap();
        let element = target.dyn_ref::<HtmlElement>().unwrap();
        
        // Get the element's position
        let x = element.offset_left();
        let y = element.offset_top();
        
        // Calculate the offset within the element
        let offset_x = event.client_x() - x;
        let offset_y = event.client_y() - y;
        
        DRAG_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dragging = true;
            state.offset_x = offset_x;
            state.offset_y = offset_y;
        });
    }) as Box<dyn FnMut(_)>);
    
    box_element.add_event_listener_with_callback(
        "mousedown",
        mouse_down_handler.as_ref().unchecked_ref(),
    )?;
    mouse_down_handler.forget(); // Prevent closure from being dropped
    
    // Set up mouse move event on document
    let mouse_move_handler = Closure::wrap(Box::new(move |event: MouseEvent| {
        DRAG_STATE.with(|state| {
            let state = state.borrow();
            if state.dragging {
                let document = web_sys::window().unwrap().document().unwrap();
                if let Some(box_element) = document.get_element_by_id("draggable-box") {
                    let box_element = box_element.dyn_ref::<HtmlElement>().unwrap();
                    
                    let x = event.client_x() - state.offset_x;
                    let y = event.client_y() - state.offset_y;
                    
                    box_element.style().set_property("left", &format!("{}px", x)).unwrap();
                    box_element.style().set_property("top", &format!("{}px", y)).unwrap();
                }
            }
        });
    }) as Box<dyn FnMut(_)>);
    
    document.add_event_listener_with_callback(
        "mousemove",
        mouse_move_handler.as_ref().unchecked_ref(),
    )?;
    mouse_move_handler.forget(); // Prevent closure from being dropped
    
    // Set up mouse up event on document
    let mouse_up_handler = Closure::wrap(Box::new(move |_event: MouseEvent| {
        DRAG_STATE.with(|state| {
            let mut state = state.borrow_mut();
            state.dragging = false;
        });
    }) as Box<dyn FnMut(_)>);
    
    document.add_event_listener_with_callback(
        "mouseup",
        mouse_up_handler.as_ref().unchecked_ref(),
    )?;
    mouse_up_handler.forget(); // Prevent closure from being dropped
    
    // Add box to the document body
    let body = document.body().expect("document should have a body");
    body.append_child(&box_element)?;
    
    Ok(())
} 