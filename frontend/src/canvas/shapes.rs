use web_sys::CanvasRenderingContext2d;
use crate::models::Node;
use crate::constants::*;

pub fn draw_rounded_rect(context: &CanvasRenderingContext2d, node: &Node) {
    context.save();
    
    // Shadow
    context.set_shadow_color(SHADOW_COLOR);
    context.set_shadow_blur(10.0);
    context.set_shadow_offset_x(3.0);
    context.set_shadow_offset_y(3.0);
    
    // Background
    context.set_fill_style_str(&node.color);
    
    draw_rounded_rect_path(context, node);
    
    context.fill();
    
    // Border
    context.set_shadow_color("rgba(0, 0, 0, 0)"); // Remove shadow for border
    context.set_line_width(1.0);
    context.set_stroke_style_str(NODE_BORDER_DEFAULT);
    
    if node.is_selected {
        // Add highlights for selected node
        context.set_stroke_style_str(NODE_BORDER_SELECTED);
        context.set_line_width(2.0);
    }
    
    context.stroke();
    
    context.restore();
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
    context.set_stroke_style_str(CONNECTION_LINE_COLOR);
    context.set_line_width(2.0);
    context.stroke();
}

pub fn draw_thought_bubble(context: &CanvasRenderingContext2d, node: &Node) {
    // Main bubble
    context.save();
    
    // Shadow
    context.set_shadow_color("rgba(0, 0, 0, 0.1)");
    context.set_shadow_blur(8.0);
    context.set_shadow_offset_x(2.0);
    context.set_shadow_offset_y(2.0);
    
    // Background
    context.set_fill_style_str(&node.color);
    
    // Draw bubble path
    let radius = 15.0;
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;
    
    context.begin_path();
    context.move_to(x + radius, y);
    context.line_to(x + width - radius, y);
    context.quadratic_curve_to(x + width, y, x + width, y + radius);
    context.line_to(x + width, y + height - radius);
    context.quadratic_curve_to(x + width, y + height, x + width - radius, y + height);
    context.line_to(x + radius + 40.0, y + height); // Leave space for tail
    
    // Add tail
    context.line_to(x + 20.0, y + height + 20.0);
    context.line_to(x + radius + 20.0, y + height);
    
    context.line_to(x + radius, y + height);
    context.quadratic_curve_to(x, y + height, x, y + height - radius);
    context.line_to(x, y + radius);
    context.quadratic_curve_to(x, y, x + radius, y);
    context.close_path();
    
    context.fill();
    
    // Border
    context.set_shadow_color("rgba(0, 0, 0, 0)"); // Remove shadow for border
    context.set_line_width(1.0);
    context.set_stroke_style_str("#000000");
    
    if node.is_selected {
        // Add highlights for selected node
        context.set_stroke_style_str("#3498db"); // Blue highlight 
        context.set_line_width(2.0);
    }
    
    context.stroke();
    
    context.restore();
}

pub fn draw_node_text(context: &CanvasRenderingContext2d, node: &Node) {
    let text = &node.text;
    
    context.save();
    
    // Text configuration
    context.set_font("14px Arial");
    context.set_fill_style_str(NODE_TEXT_COLOR);
    context.set_text_align("left");
    context.set_text_baseline("top");
    
    // Basic word wrap
    let max_width = node.width - 20.0; // Padding on both sides
    let x = node.x + 10.0; // Left padding
    let mut y = node.y + 10.0; // Top padding
    let line_height = 18.0;
    
    // Split by words
    let words = text.split_whitespace().collect::<Vec<&str>>();
    let mut current_line = String::new();
    
    for word in words {
        let test_line = if current_line.is_empty() {
            word.to_string()
        } else {
            format!("{} {}", current_line, word)
        };
        
        let test_metrics = context.measure_text(&test_line).unwrap();
        let test_width = test_metrics.width();
        
        if test_width > max_width && !current_line.is_empty() {
            // Draw the current line and start a new one
            context.fill_text(&current_line, x, y).unwrap();
            current_line = word.to_string();
            y += line_height;
        } else {
            current_line = test_line;
        }
    }
    
    // Draw the last line
    if !current_line.is_empty() {
        context.fill_text(&current_line, x, y).unwrap();
    }
    
    context.restore();
}

// Creates a rounded rectangle path without filling or stroking
pub fn draw_rounded_rect_path(context: &CanvasRenderingContext2d, node: &Node) {
    let radius = 15.0;
    let x = node.x;
    let y = node.y;
    let width = node.width;
    let height = node.height;
    
    // Draw rounded rectangle path
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
} 