use web_sys::CanvasRenderingContext2d;
use crate::models::{Node, NodeType};
use crate::state::AppState;
use super::shapes;
use js_sys::Date;

pub fn draw_nodes(state: &AppState) {
    if let (Some(canvas), Some(context)) = (&state.canvas, &state.context) {
        // Clear the canvas
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
        
        // Get the device pixel ratio
        let window = web_sys::window().expect("no global window exists");
        let dpr = window.device_pixel_ratio();
        
        // Apply transformation for viewport
        context.save();
        
        // Reset any previous transforms
        let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        
        // Scale by device pixel ratio first
        let _ = context.scale(dpr, dpr);
        
        // Apply viewport transform (scale and translate)
        let _ = context.scale(state.zoom_level, state.zoom_level);
        let _ = context.translate(-state.viewport_x, -state.viewport_y);
        
        // Draw connections first (in the background)
        draw_connections(state, &context);
        
        // Draw all nodes
        for (_, node) in &state.nodes {
            draw_node(state, node, &context);
        }
        
        // Restore original context
        context.restore();
    }
}

fn draw_connections(state: &AppState, context: &CanvasRenderingContext2d) {
    for (_, node) in &state.nodes {
        if let Some(parent_id) = &node.parent_id {
            if let Some(parent) = state.nodes.get(parent_id) {
                // Draw curved connection
                context.begin_path();
                
                // Start point (bottom of parent node)
                let start_x = parent.x + parent.width / 2.0;
                let start_y = parent.y + parent.height;
                
                // End point (top of response node)
                let end_x = node.x + node.width / 2.0;
                let end_y = node.y;
                
                // Control points for the bezier curve - creates a nice "S" curve
                let midpoint_y = start_y + (end_y - start_y) / 2.0;
                let control_x1 = start_x;
                let control_y1 = midpoint_y - 10.0;
                let control_x2 = end_x;
                let control_y2 = midpoint_y + 10.0;
                
                // Draw the curve
                context.move_to(start_x, start_y);
                context.bezier_curve_to(
                    control_x1, control_y1,
                    control_x2, control_y2,
                    end_x, end_y
                );
                
                // Style and stroke the path
                context.set_stroke_style_str("#95a5a6");
                context.set_line_width(2.0);
                context.stroke();
                
                // Optional: Add an arrow at the end pointing down
                shapes::draw_arrow(context, end_x, end_y, 0.0, -1.0);
            }
        }
    }
}

pub fn draw_node(_state: &AppState, node: &Node, context: &CanvasRenderingContext2d) {
    // Draw the appropriate node shape based on type
    match node.node_type {
        NodeType::UserInput => {
            // Draw a rounded rectangle for user inputs
            shapes::draw_rounded_rect(context, node);
        },
        NodeType::AgentResponse => {
            // Draw a thought bubble for AI responses
            shapes::draw_thought_bubble(context, node);
        },
        NodeType::AgentIdentity => {
            // For agent nodes, draw a special rectangle with double border
            context.save();
            
            // Draw outer rectangle
            context.begin_path();
            
            // Set different border styles based on agent status
            if let Some(status) = &node.status {
                match status.as_str() {
                    "processing" => {
                        // Pulsing animation using time
                        let timestamp = Date::now() as f64;
                        let pulse = (timestamp / 500.0).sin() * 0.5 + 0.5; // 0 to 1 pulsing
                        let pulse_color = format!("rgba(255, 255, 255, {})", 0.5 + pulse * 0.5);
                        context.set_stroke_style_str(&pulse_color);
                    },
                    "error" => {
                        context.set_stroke_style_str("rgba(255, 0, 0, 0.8)");
                    },
                    _ => { // idle or other
                        context.set_stroke_style_str("white");
                    }
                }
            } else {
                context.set_stroke_style_str("white");
            }
            
            context.set_line_width(2.0);
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();
            
            // Fill with semi-transparent background
            context.set_fill_style_str("rgba(255, 255, 255, 0.1)");
            context.fill();
            
            context.restore();
        },
    }
    
    // Draw the text content of the node
    shapes::draw_node_text(context, node);
} 