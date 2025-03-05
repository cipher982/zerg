use wasm_bindgen::prelude::*;
use web_sys::{window, Element, HtmlCanvasElement};

pub fn generate_favicon() -> Result<(), JsValue> {
    let window = window().expect("no global window exists");
    let document = window.document().expect("no document exists");
    
    // Create a canvas to draw the favicon
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    
    // Set dimensions for favicon (32x32 for better quality)
    canvas.set_width(32);
    canvas.set_height(32);
    
    // Get 2D context for drawing
    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    
    // Background - use the non-deprecated methods
    context.set_fill_style_str("#3498db"); // Blue background
    context.fill_rect(0.0, 0.0, 32.0, 32.0);
    
    // Draw a stylized Z
    context.set_fill_style_str("#ffffff"); // White Z
    context.begin_path();
    context.move_to(8.0, 8.0);
    context.line_to(24.0, 8.0);
    context.line_to(8.0, 24.0);
    context.line_to(24.0, 24.0);
    context.fill();
    
    // Add border
    context.set_stroke_style_str("#2c3e50");
    context.set_line_width(1.0);
    context.stroke_rect(1.0, 1.0, 30.0, 30.0);
    
    // Convert canvas to data URL
    let data_url = canvas.to_data_url()?;
    
    // Create a favicon link element
    let link: Element = document.create_element("link")?;
    link.set_attribute("rel", "icon")?;
    link.set_attribute("type", "image/png")?;
    link.set_attribute("href", &data_url)?;
    
    // Check for head element
    if let Ok(Some(head)) = document.query_selector("head") {
        // Try to find an existing favicon
        if let Ok(Some(old_icon)) = document.query_selector("link[rel='icon']") {
            // If found, remove it
            if let Some(parent) = old_icon.parent_node() {
                let _ = parent.remove_child(&old_icon);
            }
        }
        
        // Try to find an existing shortcut icon
        if let Ok(Some(old_shortcut)) = document.query_selector("link[rel='shortcut icon']") {
            // If found, remove it
            if let Some(parent) = old_shortcut.parent_node() {
                let _ = parent.remove_child(&old_shortcut);
            }
        }
        
        // Add the new favicon link
        head.append_child(&link)?;
    }
    
    Ok(())
} 