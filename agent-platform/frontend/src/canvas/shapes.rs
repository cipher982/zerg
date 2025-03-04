use web_sys::CanvasRenderingContext2d;
use wasm_bindgen::JsValue;
use crate::models::Node;

pub fn draw_rounded_rect(context: &CanvasRenderingContext2d, node: &Node) {
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

pub fn draw_arrow(context: &CanvasRenderingContext2d, x: f64, y: f64, dx: f64, dy: f64) {
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

pub fn draw_thought_bubble(context: &CanvasRenderingContext2d, node: &Node) {
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
        std::f64::consts::PI * 0.0
    );
    
    context.arc(
        x + width / 2.0, 
        y, 
        radius * 0.8, 
        std::f64::consts::PI, 
        std::f64::consts::PI * 0.0
    );
    
    context.arc(
        x + width - radius, 
        y + radius, 
        radius, 
        std::f64::consts::PI * 1.0, 
        std::f64::consts::PI * 1.5
    );
    
    // Right edge
    context.arc(
        x + width, 
        y + height / 2.0, 
        radius * 0.8, 
        std::f64::consts::PI * 1.5, 
        std::f64::consts::PI * 0.5
    );
    
    // Bottom edge
    context.arc(
        x + width - radius, 
        y + height - radius, 
        radius, 
        std::f64::consts::PI * 0.0, 
        std::f64::consts::PI * 0.5
    );
    
    context.arc(
        x + width / 2.0, 
        y + height, 
        radius * 0.8, 
        std::f64::consts::PI * 0.0, 
        std::f64::consts::PI * 1.0
    );
    
    context.arc(
        x + radius, 
        y + height - radius, 
        radius, 
        std::f64::consts::PI * 0.5, 
        std::f64::consts::PI * 1.0
    );
    
    // Left edge
    context.arc(
        x, 
        y + height / 2.0, 
        radius * 0.8, 
        std::f64::consts::PI * 0.5, 
        std::f64::consts::PI * 1.5
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
        context.arc(last_x, last_y, *size, 0.0, std::f64::consts::PI * 2.0);
        context.fill();
        context.stroke();
        
        // Move x slightly for next bubble
        last_x -= *size * 0.8;
    }
}

pub fn draw_node_text(context: &CanvasRenderingContext2d, node: &Node) {
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