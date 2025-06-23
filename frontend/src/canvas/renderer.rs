use web_sys::{CanvasRenderingContext2d};
use crate::models::{Node, NodeType, ApiAgent, NodeExecStatus};
use crate::state::AppState;
use crate::constants::*;
use super::shapes;
use js_sys::Date;
use std::collections::HashMap;

// Removed redundant import of HtmlCanvasElement, it's already imported on line 1

#[allow(dead_code)]
pub fn draw_nodes(state: &mut AppState) {
    if let (Some(canvas_el), Some(context)) = (&state.canvas, &state.context) {
        context.set_fill_style_str("rgba(10, 10, 15, 1)");
        context.fill_rect(0.0, 0.0, canvas_el.width() as f64, canvas_el.height() as f64);

        // Update and draw the particle system
        if let Some(ps) = &mut state.particle_system {
            ps.update();
            ps.draw(context);
        }

        // Animate connection lines
        state.connection_animation_offset += 0.5;
        if state.connection_animation_offset > 100.0 {
            state.connection_animation_offset = 0.0;
        }
        
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
        
        // Draw connection preview line if dragging
        draw_connection_preview(state, &context);
        
        // Draw all nodes with connectivity information
        for (_, node) in &state.nodes.clone() {
            let is_reachable = state.is_node_reachable_from_trigger(&node.node_id);
            draw_node(&context, &node, &state.agents, &state.selected_node_id, &state.connection_source_node, state.connection_mode, &state.hovered_handle, is_reachable);
        }
        
        // Restore original context
        context.restore();
    }
}

#[allow(dead_code)]
fn draw_connections(state: &AppState, context: &CanvasRenderingContext2d) {
    // Draw legacy parent-child connections
    for (_, node) in &state.nodes {
        if let Some(parent_id) = &node.parent_id {
            if let Some(parent) = state.nodes.get(parent_id) {
                draw_connection_line(context, parent, node, &state.connection_animation_offset);
            }
        }
    }
    
    // Draw modern workflow edges
    if let Some(current_workflow_id) = state.current_workflow_id {
        if let Some(workflow) = state.workflows.get(&current_workflow_id) {
            for edge in &workflow.edges {
                if let (Some(from_node), Some(to_node)) = (
                    state.nodes.get(&edge.from_node_id),
                    state.nodes.get(&edge.to_node_id)
                ) {
                    draw_connection_line(context, from_node, to_node, &state.connection_animation_offset);
                }
            }
        }
    }
}

/// Draw a connection line between two nodes with animation
/// The animation flows from from_node to to_node (source → destination)
fn draw_connection_line(context: &CanvasRenderingContext2d, from_node: &crate::models::Node, to_node: &crate::models::Node, animation_offset: &f64) {
    context.begin_path();
    
    // Start point (bottom of from node)
    let start_x = from_node.x + from_node.width / 2.0;
    let start_y = from_node.y + from_node.height;
    
    // End point (top of to node)
    let end_x = to_node.x + to_node.width / 2.0;
    let end_y = to_node.y;
    
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

    // Check if either node is running for enhanced glow effect
    let from_running = from_node.exec_status == Some(crate::models::NodeExecStatus::Running);
    let to_running = to_node.exec_status == Some(crate::models::NodeExecStatus::Running);
    let glow_active = from_running || to_running;
    
    // Backup original context state
    context.save();
    
    if glow_active {
        // Use a bright accent colour when nodes are running
        context.set_stroke_style_str("#1fb6ff"); // --primary-accent
        context.set_line_width(3.0);
        
        // Soft outer glow
        context.set_shadow_color("rgba(31, 182, 255, 0.8)");
        context.set_shadow_blur(8.0);
        
        // Animated dashed line conveys progress along the edge
        let dash_arr = js_sys::Array::new();
        dash_arr.push(&10_f64.into());
        dash_arr.push(&6_f64.into());
        let _ = context.set_line_dash(&dash_arr);
        context.set_line_dash_offset(-*animation_offset); // Negative for forward flow
    } else {
        // Default connection style with subtle animation
        context.set_stroke_style_str("#95a5a6");
        context.set_line_width(2.0);
        
        // Add subtle animated dashes to all connection lines
        let dash_arr = js_sys::Array::new();
        dash_arr.push(&8_f64.into());
        dash_arr.push(&4_f64.into());
        let _ = context.set_line_dash(&dash_arr);
        context.set_line_dash_offset(-*animation_offset * 0.3); // Negative for forward flow, slower animation for idle lines
    }
    
    context.stroke();
    
    // Restore context so arrow inherits default style
    context.restore();
    
    // Add an arrow at the end pointing to the target node
    super::shapes::draw_arrow(context, end_x, end_y, 0.0, -1.0);
}

pub fn draw_node(context: &CanvasRenderingContext2d, node: &Node, agents: &HashMap<u32, ApiAgent>, selected_node_id: &Option<String>, connection_source_node: &Option<String>, connection_mode: bool, hovered_handle: &Option<(String, String)>, is_reachable: bool) {
    // Apply visual greying for unreachable nodes
    context.save();
    if !is_reachable {
        context.set_global_alpha(0.4); // Make unconnected nodes semi-transparent
    }
    
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
            context.set_shadow_color("rgba(0, 0, 0, 0.25)");
            context.set_shadow_blur(25.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(10.0);
            
            // Fill with the new card color from the design system
            context.set_fill_style_str("#2d2d44"); // --dark-card
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
                Some("success") => (STATUS_SUCCESS_COLOR, STATUS_SUCCESS_BG, STATUS_SUCCESS_COLOR.to_string()),
                Some("scheduled") => (STATUS_SCHEDULED_COLOR, STATUS_SCHEDULED_BG, STATUS_SCHEDULED_COLOR.to_string()),
                Some("paused") => (STATUS_PAUSED_COLOR, STATUS_PAUSED_BG, STATUS_PAUSED_COLOR.to_string()),
                _ => (STATUS_IDLE_COLOR, STATUS_IDLE_BG, STATUS_IDLE_COLOR.to_string()),
            };
            
            // Draw border with status color
            context.set_line_width(2.0);
            context.set_stroke_style_str(&status_text);
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();
            
            // Draw modern status icon
            context.set_font("18px system-ui, -apple-system, sans-serif");
            context.set_text_align("center");
            context.set_text_baseline("middle");
            
            let icon = match status.as_deref() {
                Some("processing") | Some("running") => "⚡",
                Some("error") => "⚠️",
                Some("success") => "✅",
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
            context.set_fill_style_str("#ffffff"); // --text
            context.set_text_align("left");
            context.set_text_baseline("middle");
            let _ = context.fill_text(&agent_name, node.x + 50.0, node.y + 30.0);
            
            // Status text
            let status_text = status.as_deref().unwrap_or("idle");
            context.set_font("400 12px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("rgba(255, 255, 255, 0.7)"); // --text-secondary
            let _ = context.fill_text(&status_text.to_uppercase(), node.x + 50.0, node.y + 45.0);
            
            context.restore();
        },
        NodeType::Tool { tool_name, server_name, config: _, visibility: _ } => {
            // Draw a card-style tool node (rounded rectangle, similar to agent)
            context.save();

            // Shadow for depth
            context.set_shadow_color("rgba(0, 0, 0, 0.10)");
            context.set_shadow_blur(16.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(6.0);

            // Card background
            context.set_fill_style_str("#f8fafc");
            shapes::draw_rounded_rect_path(context, node);
            context.fill();

            // Remove shadow for other elements
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);

            // Accent bar (top) - color based on execution status
            let accent_color = match node.exec_status {
                Some(NodeExecStatus::Running) => "#fbbf24", // amber-400
                Some(NodeExecStatus::Success) => "#22c55e", // green-500
                Some(NodeExecStatus::Failed) => "#ef4444",  // red-500
                _ => "#38bdf8", // sky-400 (default)
            };
            
            context.begin_path();
            context.move_to(node.x, node.y + 6.0);
            context.line_to(node.x + node.width, node.y + 6.0);
            context.set_stroke_style_str(accent_color);
            context.set_line_width(4.0);
            context.stroke();

            // Main border
            context.set_line_width(1.0);
            context.set_stroke_style_str("#e2e8f0");
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();

            // Tool icon
            let icon = match server_name.as_str() {
                "github" => "🐙",
                "slack" => "💬",
                "linear" => "📋",
                "gmail" => "📧",
                "http" => "🌐",
                _ => "🔧",
            };
            context.set_font("18px system-ui, -apple-system, sans-serif");
            context.set_text_align("left");
            context.set_text_baseline("middle");
            context.set_fill_style_str("#38bdf8");
            let _ = context.fill_text(icon, node.x + 18.0, node.y + 32.0);

            // Tool name
            context.set_font("600 16px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#1e293b");
            let _ = context.fill_text(tool_name, node.x + 48.0, node.y + 30.0);

            // Server name/badge
            context.set_font("400 13px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#64748b");
            let _ = context.fill_text(server_name, node.x + 48.0, node.y + 50.0);

            // Execution status indicator (top-right)
            let status_x = node.x + node.width - 20.0;
            let status_y = node.y + 18.0;
            context.begin_path();
            let _ = context.arc(status_x, status_y, 5.0, 0.0, 2.0 * std::f64::consts::PI);
            
            let status_color = match node.exec_status {
                Some(NodeExecStatus::Running) => "#fbbf24", // amber-400
                Some(NodeExecStatus::Success) => "#22c55e", // green-500
                Some(NodeExecStatus::Failed) => "#ef4444",  // red-500
                _ => "#94a3b8", // slate-400 (idle)
            };
            
            context.set_fill_style_str(status_color);
            context.fill();
            
            // Add pulsing effect for running status
            if node.exec_status == Some(NodeExecStatus::Running) {
                let timestamp = js_sys::Date::now() as f64;
                let pulse = (timestamp / 500.0).sin() * 0.3 + 0.7;
                context.set_global_alpha(pulse);
                context.begin_path();
                let _ = context.arc(status_x, status_y, 8.0, 0.0, 2.0 * std::f64::consts::PI);
                context.set_fill_style_str("rgba(251, 191, 36, 0.4)");
                context.fill();
                context.set_global_alpha(1.0);
            }

            context.restore();
        },
        NodeType::Trigger { trigger_type, config: _ } => {
            // Draw a card-style trigger node (rounded rectangle, green accent)
            context.save();

            // Shadow for depth
            context.set_shadow_color("rgba(0, 0, 0, 0.10)");
            context.set_shadow_blur(16.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(6.0);

            // Card background
            context.set_fill_style_str("#ecfdf5");
            shapes::draw_rounded_rect_path(context, node);
            context.fill();

            // Remove shadow for other elements
            context.set_shadow_blur(0.0);
            context.set_shadow_offset_x(0.0);
            context.set_shadow_offset_y(0.0);

            // Accent bar (top)
            context.begin_path();
            context.move_to(node.x, node.y + 6.0);
            context.line_to(node.x + node.width, node.y + 6.0);
            context.set_stroke_style_str("#10b981");
            context.set_line_width(4.0);
            context.stroke();

            // Main border
            context.set_line_width(1.0);
            context.set_stroke_style_str("#d1fae5");
            shapes::draw_rounded_rect_path(context, node);
            context.stroke();

            // Trigger icon and type
            let (icon, type_text) = match trigger_type {
                crate::models::TriggerType::Webhook => ("🔗", "Webhook"),
                crate::models::TriggerType::Schedule => ("⏰", "Schedule"),
                crate::models::TriggerType::Email => ("📧", "Email"),
                crate::models::TriggerType::Manual => ("", "Manual"),
            };

            // Icon
            context.set_font("18px system-ui, -apple-system, sans-serif");
            context.set_text_align("left");
            context.set_text_baseline("middle");
            context.set_fill_style_str("#10b981");
            let _ = context.fill_text(icon, node.x + 18.0, node.y + 32.0);

            // Type text
            context.set_font("600 16px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#065f46");
            let _ = context.fill_text(type_text, node.x + 48.0, node.y + 30.0);

            // Label
            context.set_font("400 13px system-ui, -apple-system, sans-serif");
            context.set_fill_style_str("#047857");
            let _ = context.fill_text("Trigger", node.x + 48.0, node.y + 50.0);

            // Status indicator (top-right)
            let status_x = node.x + node.width - 20.0;
            let status_y = node.y + 18.0;
            context.begin_path();
            let _ = context.arc(status_x, status_y, 5.0, 0.0, 2.0 * std::f64::consts::PI);
            context.set_fill_style_str("#10b981");
            context.fill();

            context.restore();
        },
        NodeType::GenericNode => {
            // Draw a simple rectangle for generic nodes
            shapes::draw_rounded_rect(context, node);
        },
    }
    
    // Draw selection/connection mode visual feedback
    let is_selected = selected_node_id.as_ref() == Some(&node.node_id);
    let is_connection_source = connection_source_node.as_ref() == Some(&node.node_id);
    
    if is_selected || is_connection_source {
        context.save();
        
        // Draw selection border
        context.begin_path();
        shapes::draw_rounded_rect_path(context, node);
        
        if is_connection_source {
            // Source node gets a blue selection border
            context.set_stroke_style_str("#3b82f6");
            context.set_line_width(3.0);
        } else if is_selected {
            // Regular selected node gets a white border
            context.set_stroke_style_str("#ffffff");
            context.set_line_width(2.0);
        }
        
        context.stroke();
        context.restore();
    }
    
    // In connection mode, show connection hints
    if connection_mode && connection_source_node.is_some() && !is_connection_source {
        context.save();
        
        // Draw a subtle glow for potential connection targets
        context.begin_path();
        shapes::draw_rounded_rect_path(context, node);
        context.set_stroke_style_str("rgba(34, 197, 94, 0.5)");
        context.set_line_width(1.0);
        context.stroke();
        
        context.restore();
    }
    
    // Draw connection handles (small circles on node edges)
    draw_connection_handles(context, node, hovered_handle);
    
    // Draw the text content of the node
    shapes::draw_node_text(context, node);
    
    // Restore context (for alpha changes)
    context.restore();
}

/// Draw small circular connection handles on the edges of nodes
fn draw_connection_handles(context: &CanvasRenderingContext2d, node: &Node, hovered_handle: &Option<(String, String)>) {
    context.save();
    
    let handle_radius = 6.0;
    let input_handle_color = "#22c55e"; // Green for input
    let output_handle_color = "#3b82f6"; // Blue for output
    let handle_hover_color = "#f59e0b"; // Orange when hovering
    let handle_hover_radius = 8.0; // Larger when hovering
    
    // Calculate handle positions (input/output)
    let handles = [
        (node.x + node.width / 2.0, node.y, "input"), // Top = Input
        (node.x + node.width / 2.0, node.y + node.height, "output"), // Bottom = Output
    ];
    
    for (x, y, position) in handles.iter() {
        // Check if this handle is being hovered
        let is_hovered = if let Some((hovered_node_id, hovered_pos)) = hovered_handle {
            hovered_node_id == &node.node_id && hovered_pos == position
        } else {
            false
        };
        
        let current_radius = if is_hovered { handle_hover_radius } else { handle_radius };
        let base_color = if *position == "input" { input_handle_color } else { output_handle_color };
        let current_color = if is_hovered { handle_hover_color } else { base_color };
        
        // Draw handle background
        context.begin_path();
        let _ = context.arc(*x, *y, current_radius, 0.0, 2.0 * std::f64::consts::PI);
        context.set_fill_style_str("#ffffff");
        context.fill();
        
        // Draw handle border
        context.set_stroke_style_str(current_color);
        context.set_line_width(if is_hovered { 3.0 } else { 2.0 });
        context.stroke();
        
        // Add a subtle glow effect when hovered
        if is_hovered {
            context.begin_path();
            let _ = context.arc(*x, *y, current_radius + 2.0, 0.0, 2.0 * std::f64::consts::PI);
            context.set_stroke_style_str("rgba(59, 130, 246, 0.3)"); // Semi-transparent blue
            context.set_line_width(4.0);
            context.stroke();
        }
    }
    
    context.restore();
}

/// Draw preview line when dragging a connection
fn draw_connection_preview(state: &AppState, context: &CanvasRenderingContext2d) {
    if !state.connection_drag_active {
        return;
    }
    
    if let (Some((source_node_id, source_handle)), Some((current_x, current_y))) = 
        (&state.connection_drag_start, &state.connection_drag_current) {
        
        if let Some(source_node) = state.nodes.get(source_node_id) {
            // Calculate source handle position
            let (start_x, start_y) = match source_handle.as_str() {
                "top" => (source_node.x + source_node.width / 2.0, source_node.y),
                "right" => (source_node.x + source_node.width, source_node.y + source_node.height / 2.0),
                "bottom" => (source_node.x + source_node.width / 2.0, source_node.y + source_node.height),
                "left" => (source_node.x, source_node.y + source_node.height / 2.0),
                _ => (source_node.x + source_node.width / 2.0, source_node.y),
            };
            
            context.save();
            
            // Draw dashed preview line
            context.begin_path();
            let dash_array = js_sys::Array::new();
            dash_array.push(&wasm_bindgen::JsValue::from(5.0));
            dash_array.push(&wasm_bindgen::JsValue::from(5.0));
            context.set_line_dash(&dash_array).unwrap();
            context.set_stroke_style_str("#3b82f6");
            context.set_line_width(2.0);
            
            context.move_to(start_x, start_y);
            context.line_to(*current_x, *current_y);
            context.stroke();
            
            context.restore();
        }
    }
}
