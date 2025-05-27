use web_sys::{CanvasRenderingContext2d};
use crate::models::{Node, NodeType, ApiAgent};
use crate::state::AppState;
use crate::constants::*;
use super::shapes;
use js_sys::Date;
use std::collections::HashMap;

// Removed redundant import of HtmlCanvasElement, it's already imported on line 1

#[allow(dead_code)]
pub fn draw_nodes(state: &AppState) {
    if let (Some(canvas_el), Some(context)) = (&state.canvas, &state.context) {
        // Ensure canvas element itself has the background color set via style attribute
        // This is a fallback/override if CSS isn't applying as expected.
        // canvas_el is already &HtmlCanvasElement from the outer if let
        let _ = canvas_el.style().set_property("background-color", CANVAS_BACKGROUND_COLOR);

        // Fill canvas rendering context with background color first
        context.save();
        context.set_fill_style_str(CANVAS_BACKGROUND_COLOR);
        context.fill_rect(0.0, 0.0, canvas_el.width() as f64, canvas_el.height() as f64);
        context.restore();
        
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
            draw_node(&context, node, &state.agents);
        }
        
        // Restore original context
        context.restore();
    }
}

#[allow(dead_code)]
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

pub fn draw_node(context: &CanvasRenderingContext2d, node: &Node, agents: &HashMap<u32, ApiAgent>) {
    // Draw the appropriate node shape based on type
    match node.node_type {
        NodeType::UserInput => {
            // Draw a rounded rectangle for user inputs
            shapes::draw_rounded_rect(context, node);
        },
        NodeType::ResponseOutput => {
            // Draw a thought bubble for AI responses
            shapes::draw_thought_bubble(context, node);
        },
        NodeType::AgentIdentity => {
            // For agent nodes, draw a special styled card
            context.save();
            
            // Shadow for depth
            context.set_shadow_color(SHADOW_COLOR);
            context.set_shadow_blur(10.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(3.0);
            
            // Fill with white background first
            context.set_fill_style_str(NODE_FILL_AGENT_IDENTITY);
            shapes::draw_rounded_rect_path(context, node);
            context.fill();
            
            // Remove shadow for border
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);
            
            // Get agent status and name
            let (status, agent_name) = node.agent_id.and_then(|agent_id| {
                agents.get(&agent_id).map(|agent| {
                    (agent.status.clone(), agent.name.clone())
                })
            }).unwrap_or((None, "Unknown Agent".to_string()));
            
            // Draw status indicator bar at top
            let status_color = match status.as_deref() {
                Some("processing") | Some("running") => {
                    // Animated processing state
                    let timestamp = Date::now() as f64;
                    let pulse = (timestamp / 500.0).sin() * 0.5 + 0.5;
                    format!("rgba({}, {})", NODE_BORDER_AGENT_PROCESSING_BASE, 0.7 + pulse * 0.3)
                },
                Some("error") => NODE_BORDER_AGENT_ERROR.to_string(),
                Some("scheduled") => NODE_BORDER_AGENT_SCHEDULED.to_string(),
                Some("paused") => NODE_BORDER_AGENT_PAUSED.to_string(),
                _ => NODE_BORDER_AGENT_IDLE.to_string(),
            };
            
            // Draw status bar at top
            context.begin_path();
            context.move_to(node.x + 15.0, node.y);
            context.line_to(node.x + node.width - 15.0, node.y);
            context.line_to(node.x + node.width - 15.0, node.y + 4.0);
            context.line_to(node.x + 15.0, node.y + 4.0);
            context.close_path();
            context.set_fill_style_str(&status_color);
            context.fill();
            
            // Draw border
            context.set_line_width(1.5);
            context.set_stroke_style_str(&status_color);
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();
            
            // Add icon based on status
            context.set_font("16px Arial");
            context.set_text_align("center");
            context.set_text_baseline("middle");
            
            let icon = match status.as_deref() {
                Some("processing") | Some("running") => "⚡",
                Some("error") => "⚠️",
                Some("scheduled") => "⏰",
                Some("paused") => "⏸️",
                _ => "🤖",
            };
            
            context.set_fill_style_str(&status_color);
            let _ = context.fill_text(icon, node.x + 20.0, node.y + 25.0);
            
            // Draw agent name with better styling
            context.set_font("bold 14px Arial");
            context.set_fill_style_str(NODE_TEXT_COLOR);
            context.set_text_align("left");
            let _ = context.fill_text(&agent_name, node.x + 40.0, node.y + 25.0);
            
            context.restore();
        },
        NodeType::GenericNode => {
            // Draw a simple rectangle for generic nodes
            shapes::draw_rounded_rect(context, node);
        },
    }
    
    // Draw the text content of the node
    shapes::draw_node_text(context, node);
}
