use web_sys::CanvasRenderingContext2d;
use crate::models::{Node, NodeType};
use crate::state::AppState;
use super::shapes;

pub fn draw_nodes(state: &AppState) {
    if let (Some(canvas), Some(context)) = (&state.canvas, &state.context) {
        // Clear the canvas
        context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
        
        // Draw connections first (in the background)
        draw_connections(state, &context);
        
        // Draw all nodes
        for (_, node) in &state.nodes {
            draw_node(state, node, &context);
        }
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
                context.set_stroke_style(&wasm_bindgen::JsValue::from("#95a5a6"));
                context.set_line_width(2.0);
                context.stroke();
                
                // Optional: Add an arrow at the end pointing down
                shapes::draw_arrow(context, end_x, end_y, 0.0, -1.0);
            }
        }
    }
}

fn draw_node(_state: &AppState, node: &Node, context: &CanvasRenderingContext2d) {
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
            // For future use - draw a hexagon or other distinctive shape
            shapes::draw_rounded_rect(context, node); // Placeholder for now
        },
    }
    
    // Draw the text content of the node
    shapes::draw_node_text(context, node);
} 