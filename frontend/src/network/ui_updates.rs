use std::cell::RefCell;
use js_sys::Array;
use wasm_bindgen::prelude::*;

// Track packet counter for activity indicator
thread_local! {
    static PACKET_COUNTER: RefCell<u32> = RefCell::new(0);
}

pub fn update_connection_status(status: &str, color: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(status_element) = document.get_element_by_id("status") {
                status_element.set_class_name(color);
                status_element.set_inner_html(&format!("Status: {}", status));
            }
        }
    }
}

pub fn flash_activity() {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(status_element) = document.get_element_by_id("api-status") {
                // Update packet counter
                PACKET_COUNTER.with(|counter| {
                    let count = *counter.borrow();
                    *counter.borrow_mut() = count.wrapping_add(1);
                    status_element.set_inner_html(&format!("PKT: {:08X}", count));
                });

                // Flash the LED
                status_element.set_class_name("flash");
                
                // Remove flash after 50ms
                let status_clone = status_element.clone();
                let clear_callback = Closure::wrap(Box::new(move || {
                    status_clone.set_class_name("");
                }) as Box<dyn FnMut()>);
                
                window.set_timeout_with_callback_and_timeout_and_arguments(
                    clear_callback.as_ref().unchecked_ref(),
                    50, // Very quick flash
                    &Array::new(),
                ).expect("Failed to set timeout");
                
                clear_callback.forget();
            }
        }
    }
} 