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
    match &node.node_type {
        NodeType::UserInput => {
            // Draw a rounded rectangle for user inputs
            shapes::draw_rounded_rect(context, node);
        },
        NodeType::ResponseOutput => {
            // Draw a thought bubble for AI responses
            shapes::draw_thought_bubble(context, node);
        },
        NodeType::AgentIdentity => {
            // Draw a modern, beautiful agent card
            context.save();
            
            // Enhanced shadow for depth and luxury feel
            context.set_shadow_color("rgba(0, 0, 0, 0.15)");
            context.set_shadow_blur(20.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(8.0);
            
            // Fill with clean white background
            context.set_fill_style_str(NODE_FILL_AGENT_IDENTITY);
            shapes::draw_rounded_rect_path(context, node);
            context.fill();
            
            // Remove shadow for other elements
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);
            
            // Get agent status and name
            let (status, agent_name) = node.agent_id.and_then(|agent_id| {
                agents.get(&agent_id).map(|agent| {
                    (agent.status.clone(), agent.name.clone())
                })
            }).unwrap_or((None, "Unknown Agent".to_string()));
            
            // Status-specific colors
            let (status_color, status_bg, status_text) = match status.as_deref() {
                Some("processing") | Some("running") => {
                    let timestamp = Date::now() as f64;
                    let pulse = (timestamp / 1000.0).sin() * 0.3 + 0.7;
                    (STATUS_PROCESSING_COLOR, STATUS_PROCESSING_BG, format!("rgba(59, 130, 246, {})", pulse))
                },
                Some("error") => (STATUS_ERROR_COLOR, STATUS_ERROR_BG, STATUS_ERROR_COLOR.to_string()),
                Some("scheduled") => (STATUS_SCHEDULED_COLOR, STATUS_SCHEDULED_BG, STATUS_SCHEDULED_COLOR.to_string()),
                Some("paused") => (STATUS_PAUSED_COLOR, STATUS_PAUSED_BG, STATUS_PAUSED_COLOR.to_string()),
                _ => (STATUS_IDLE_COLOR, STATUS_IDLE_BG, STATUS_IDLE_COLOR.to_string()),
            };
            
            // Draw status indicator pill at top
            let pill_height = 6.0;
            let pill_margin = 12.0;
            context.begin_path();
            context.move_to(node.x + pill_margin + 3.0, node.y + 8.0);
            context.line_to(node.x + node.width - pill_margin - 3.0, node.y + 8.0);
            context.quadratic_curve_to(node.x + node.width - pill_margin, node.y + 8.0, node.x + node.width - pill_margin, node.y + 8.0 + 3.0);
            context.line_to(node.x + node.width - pill_margin, node.y + 8.0 + pill_height - 3.0);
            context.quadratic_curve_to(node.x + node.width - pill_margin, node.y + 8.0 + pill_height, node.x + node.width - pill_margin - 3.0, node.y + 8.0 + pill_height);
            context.line_to(node.x + pill_margin + 3.0, node.y + 8.0 + pill_height);
            context.quadratic_curve_to(node.x + pill_margin, node.y + 8.0 + pill_height, node.x + pill_margin, node.y + 8.0 + pill_height - 3.0);
            context.line_to(node.x + pill_margin, node.y + 8.0 + 3.0);
            context.quadratic_curve_to(node.x + pill_margin, node.y + 8.0, node.x + pill_margin + 3.0, node.y + 8.0);
            context.close_path();
            context.set_fill_style_str(&status_text);
            context.fill();
            
            // Draw subtle border with status color
            context.set_line_width(1.0);
            context.set_stroke_style_str(AGENT_BORDER_SUBTLE);
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();
            
            // Add status accent on left side
            context.begin_path();
            context.move_to(node.x + 3.0, node.y + 20.0);
            context.line_to(node.x + 3.0, node.y + node.height - 20.0);
            context.set_stroke_style_str(status_color);
            context.set_line_width(3.0);
            context.stroke();
            
            // Draw modern status icon
            context.set_font("18px system-ui, -apple-system, sans-serif");
            context.set_text_align("center");
            context.set_text_baseline("middle");
            
            let icon = match status.as_deref() {
                Some("processing") | Some("running") => "⚡",
                Some("error") => "⚠️",
                Some("scheduled") => "⏰",
                Some("paused") => "⏸️",
                _ => "🤖",
            };
            
            // Icon background circle
            context.begin_path();
            let _ = context.arc(node.x + 28.0, node.y + 35.0, 14.0, 0.0, 2.0 * std::f64::consts::PI);
            context.set_fill_style_str(status_bg);
            context.fill();
            
            // Icon
            context.set_fill_style_str(status_color);
            let _ = context.fill_text(icon, node.x + 28.0, node.y + 35.0);
            
            // Agent name with modern typography
            context.set_font("600 16px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str(AGENT_TEXT_PRIMARY);
            context.set_text_align("left");
            context.set_text_baseline("middle");
            let _ = context.fill_text(&agent_name, node.x + 50.0, node.y + 30.0);
            
            // Status text
            let status_text = status.as_deref().unwrap_or("idle");
            context.set_font("400 12px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str(AGENT_TEXT_SECONDARY);
            let _ = context.fill_text(&status_text.to_uppercase(), node.x + 50.0, node.y + 45.0);
            
            // Add subtle inner highlight
            context.begin_path();
            context.move_to(node.x + 15.0, node.y + 1.0);
            context.line_to(node.x + node.width - 15.0, node.y + 1.0);
            context.set_stroke_style_str("rgba(255, 255, 255, 0.8)");
            context.set_line_width(1.0);
            context.stroke();
            
            context.restore();
        },
        NodeType::Tool { tool_name, server_name, config: _, visibility: _ } => {
            // Draw a distinctive tool node with sharp corners and service branding
            context.save();
            
            // Shadow for depth
            context.set_shadow_color("rgba(0, 0, 0, 0.1)");
            context.set_shadow_blur(8.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(4.0);
            
            // Tool nodes have sharp rectangles (no rounded corners)
            context.begin_path();
            context.rect(node.x, node.y, node.width, node.height);
            
            // Service-specific colors
            let (bg_color, accent_color, icon) = match server_name.as_str() {
                "github" => ("#f6f8fa", "#24292f", "🐙"),
                "slack" => ("#f8f9fa", "#4a154b", "💬"),
                "linear" => ("#f6f7f9", "#5e6ad2", "📋"),
                "gmail" => ("#fef7f0", "#ea4335", "📧"),
                "http" => ("#f0f9ff", "#0ea5e9", "🌐"),
                _ => ("#f8fafc", "#64748b", "🔧"),
            };
            
            context.set_fill_style_str(bg_color);
            context.fill();
            
            // Remove shadow for other elements
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);
            
            // Accent border on top
            context.begin_path();
            context.move_to(node.x, node.y);
            context.line_to(node.x + node.width, node.y);
            context.set_stroke_style_str(accent_color);
            context.set_line_width(4.0);
            context.stroke();
            
            // Main border
            context.begin_path();
            context.rect(node.x, node.y, node.width, node.height);
            context.set_stroke_style_str("#e2e8f0");
            context.set_line_width(1.0);
            context.stroke();
            
            // Tool icon
            context.set_font("16px system-ui, -apple-system, sans-serif");
            context.set_text_align("left");
            context.set_text_baseline("middle");
            context.set_fill_style_str(accent_color);
            let _ = context.fill_text(icon, node.x + 12.0, node.y + 20.0);
            
            // Tool name
            context.set_font("600 14px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#1e293b");
            let _ = context.fill_text(tool_name, node.x + 35.0, node.y + 20.0);
            
            // Server name/badge
            context.set_font("400 12px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#64748b");
            let _ = context.fill_text(server_name, node.x + 12.0, node.y + 40.0);
            
            // Connection status indicator (top-right)
            let status_x = node.x + node.width - 20.0;
            let status_y = node.y + 12.0;
            context.begin_path();
            let _ = context.arc(status_x, status_y, 4.0, 0.0, 2.0 * std::f64::consts::PI);
            context.set_fill_style_str("#10b981"); // Green for connected - this would be dynamic
            context.fill();
            
            context.restore();
        },
        NodeType::Trigger { trigger_type, config: _ } => {
            // Draw a diamond-shaped trigger node
            context.save();
            
            // Shadow for depth
            context.set_shadow_color("rgba(0, 0, 0, 0.12)");
            context.set_shadow_blur(12.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(6.0);
            
            // Diamond shape
            let center_x = node.x + node.width / 2.0;
            let center_y = node.y + node.height / 2.0;
            
            context.begin_path();
            context.move_to(center_x, node.y); // Top
            context.line_to(node.x + node.width, center_y); // Right
            context.line_to(center_x, node.y + node.height); // Bottom
            context.line_to(node.x, center_y); // Left
            context.close_path();
            
            // Gradient background - use solid color for now since create_linear_gradient might not be available
            context.set_fill_style_str("#dcfce7");
            context.fill();
            
            // Remove shadow for other elements
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);
            
            // Border
            context.begin_path();
            context.move_to(center_x, node.y);
            context.line_to(node.x + node.width, center_y);
            context.line_to(center_x, node.y + node.height);
            context.line_to(node.x, center_y);
            context.close_path();
            context.set_stroke_style_str("#16a34a");
            context.set_line_width(2.0);
            context.stroke();
            
            // Trigger icon and type
            let (icon, type_text) = match trigger_type {
                crate::models::TriggerType::Webhook => ("🔗", "Webhook"),
                crate::models::TriggerType::Schedule => ("⏰", "Schedule"),
                crate::models::TriggerType::Email => ("📧", "Email"),
                crate::models::TriggerType::Manual => ("👆", "Manual"),
            };
            
            // Icon
            context.set_font("18px system-ui, -apple-system, sans-serif");
            context.set_text_align("center");
            context.set_text_baseline("middle");
            context.set_fill_style_str("#059669");
            let _ = context.fill_text(icon, center_x, center_y - 8.0);
            
            // Type text
            context.set_font("600 12px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#065f46");
            let _ = context.fill_text(type_text, center_x, center_y + 12.0);
            
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
