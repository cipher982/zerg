use web_sys::{Document, Element, HtmlElement, MouseEvent, DragEvent};
use wasm_bindgen::{JsCast, JsValue};
use crate::models::{NodeType, TriggerType, TriggerConfig, ToolConfig, ToolVisibility, InputMapping};
use crate::state::{APP_STATE, AppState};
use std::collections::HashMap;

/// Node palette component for dragging tools and triggers onto the canvas
pub struct NodePalette {
    pub is_open: bool,
    pub search_query: String,
    pub selected_category: Option<String>,
}

/// Represents a draggable node type in the palette
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaletteNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: String,
    pub node_type: NodeType,
}

impl NodePalette {
    pub fn new() -> Self {
        Self {
            is_open: false,
            search_query: String::new(),
            selected_category: None,
        }
    }

    /// Render the node palette UI
    pub fn render(&self, document: &Document) -> Result<(), JsValue> {
        // Create or get the palette container
        let palette_container = if let Some(existing) = document.get_element_by_id("node-palette") {
            existing
        } else {
            let container = document.create_element("div")?;
            container.set_id("node-palette");
            container.set_class_name("node-palette");
            
            // Add to the body or canvas container
            if let Some(canvas_container) = document.get_element_by_id("canvas-container") {
                canvas_container.append_child(&container)?;
            } else if let Some(body) = document.body() {
                body.append_child(&container)?;
            }
            
            container
        };

        // Set visibility based on is_open
        let style = if self.is_open {
            "display: block;"
        } else {
            "display: none;"
        };

        palette_container.set_attribute("style", &format!("
            position: fixed;
            left: 20px;
            top: 100px;
            width: 280px;
            height: 500px;
            background: white;
            border: 1px solid #e2e8f0;
            border-radius: 8px;
            box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
            z-index: 1000;
            padding: 16px;
            overflow-y: auto;
            font-family: system-ui, -apple-system, sans-serif;
            {}
        ", style))?;

        // Clear existing content
        palette_container.set_inner_html("");

        // Add header
        let header = document.create_element("div")?;
        header.set_class_name("palette-header");
        header.set_inner_html("
            <h3 style='margin: 0 0 16px 0; font-size: 16px; font-weight: 600; color: #1e293b;'>
                Node Palette
            </h3>
        ");
        palette_container.append_child(&header)?;

        // Add search box
        let search_container = document.create_element("div")?;
        search_container.set_inner_html(&format!("
            <input 
                type='text' 
                placeholder='Search nodes...' 
                value='{}' 
                style='
                    width: 100%; 
                    padding: 8px 12px; 
                    border: 1px solid #d1d5db; 
                    border-radius: 6px; 
                    margin-bottom: 16px;
                    font-size: 14px;
                    outline: none;
                '
                id='palette-search'
            />
        ", self.search_query));
        palette_container.append_child(&search_container)?;

        // Get available palette nodes
        let nodes = self.get_palette_nodes();

        // Group nodes by category
        let mut categories: HashMap<String, Vec<PaletteNode>> = HashMap::new();
        for node in nodes {
            categories.entry(node.category.clone())
                .or_insert_with(Vec::new)
                .push(node);
        }

        // Render categories
        for (category_name, category_nodes) in categories.iter() {
            self.render_category(document, &palette_container, category_name, category_nodes)?;
        }

        Ok(())
    }

    /// Render a category section
    fn render_category(
        &self,
        document: &Document,
        container: &Element,
        category_name: &str,
        nodes: &[PaletteNode],
    ) -> Result<(), JsValue> {
        // Category header
        let category_header = document.create_element("div")?;
        category_header.set_class_name("category-header");
        category_header.set_inner_html(&format!("
            <h4 style='
                margin: 0 0 8px 0; 
                font-size: 14px; 
                font-weight: 600; 
                color: #64748b;
                text-transform: uppercase;
                letter-spacing: 0.5px;
            '>
                {}
            </h4>
        ", category_name));
        container.append_child(&category_header)?;

        // Render nodes in this category
        for node in nodes {
            self.render_palette_node(document, container, node)?;
        }

        // Add spacing between categories
        let spacer = document.create_element("div")?;
        spacer.set_attribute("style", "height: 16px;")?;
        container.append_child(&spacer)?;

        Ok(())
    }

    /// Render an individual palette node
    fn render_palette_node(
        &self,
        document: &Document,
        container: &Element,
        node: &PaletteNode,
    ) -> Result<(), JsValue> {
        let node_element = document.create_element("div")?;
        node_element.set_class_name("palette-node");
        node_element.set_attribute("draggable", "true")?;
        node_element.set_attribute("data-node-type", &node.id)?;

        // Style the node
        node_element.set_attribute("style", "
            display: flex;
            align-items: center;
            padding: 8px 12px;
            margin-bottom: 4px;
            border: 1px solid #e2e8f0;
            border-radius: 6px;
            cursor: grab;
            transition: all 0.2s ease;
            background: #f8fafc;
        ")?;

        // Add hover effects with event listeners
        let node_clone = node_element.clone();
        let onmouseenter = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: MouseEvent| {
            let _ = node_clone.set_attribute("style", "
                display: flex;
                align-items: center;
                padding: 8px 12px;
                margin-bottom: 4px;
                border: 1px solid #3b82f6;
                border-radius: 6px;
                cursor: grab;
                transition: all 0.2s ease;
                background: #eff6ff;
                transform: translateY(-1px);
                box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
            ");
        }) as Box<dyn FnMut(_)>);

        let node_clone2 = node_element.clone();
        let onmouseleave = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: MouseEvent| {
            let _ = node_clone2.set_attribute("style", "
                display: flex;
                align-items: center;
                padding: 8px 12px;
                margin-bottom: 4px;
                border: 1px solid #e2e8f0;
                border-radius: 6px;
                cursor: grab;
                transition: all 0.2s ease;
                background: #f8fafc;
            ");
        }) as Box<dyn FnMut(_)>);

        node_element.add_event_listener_with_callback("mouseenter", onmouseenter.as_ref().unchecked_ref())?;
        node_element.add_event_listener_with_callback("mouseleave", onmouseleave.as_ref().unchecked_ref())?;

        // Store closures to prevent them from being dropped
        onmouseenter.forget();
        onmouseleave.forget();

        // Node content
        node_element.set_inner_html(&format!("
            <span style='font-size: 16px; margin-right: 8px;'>{}</span>
            <div>
                <div style='font-size: 14px; font-weight: 500; color: #1e293b;'>{}</div>
                <div style='font-size: 12px; color: #64748b; margin-top: 2px;'>{}</div>
            </div>
        ", node.icon, node.name, node.description));

        // Add drag event listeners
        self.add_drag_listeners(&node_element, node)?;

        container.append_child(&node_element)?;
        Ok(())
    }

    /// Add drag event listeners to a palette node
    fn add_drag_listeners(&self, element: &Element, node: &PaletteNode) -> Result<(), JsValue> {
        let node_data = node.clone();
        
        // Drag start event
        let ondragstart = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: DragEvent| {
            if let Some(data_transfer) = event.data_transfer() {
                // Store the node type information
                let node_json = serde_json::to_string(&node_data).unwrap_or_default();
                let _ = data_transfer.set_data("application/json", &node_json);
                let _ = data_transfer.set_data("text/plain", &node_data.id);
                
                // Set drag effect
                data_transfer.set_effect_allowed("copy");
            }
        }) as Box<dyn FnMut(_)>);

        element.add_event_listener_with_callback("dragstart", ondragstart.as_ref().unchecked_ref())?;
        ondragstart.forget();

        Ok(())
    }

    /// Get available palette nodes based on current state
    fn get_palette_nodes(&self) -> Vec<PaletteNode> {
        let mut nodes = Vec::new();

        // Trigger nodes
        nodes.push(PaletteNode {
            id: "trigger_webhook".to_string(),
            name: "Webhook Trigger".to_string(),
            description: "Trigger from HTTP webhook".to_string(),
            icon: "🔗".to_string(),
            category: "Triggers".to_string(),
            node_type: NodeType::Trigger {
                trigger_type: TriggerType::Webhook,
                config: TriggerConfig {
                    params: HashMap::new(),
                    enabled: true,
                    filters: Vec::new(),
                },
            },
        });

        nodes.push(PaletteNode {
            id: "trigger_schedule".to_string(),
            name: "Schedule Trigger".to_string(),
            description: "Trigger on schedule".to_string(),
            icon: "⏰".to_string(),
            category: "Triggers".to_string(),
            node_type: NodeType::Trigger {
                trigger_type: TriggerType::Schedule,
                config: TriggerConfig {
                    params: HashMap::new(),
                    enabled: true,
                    filters: Vec::new(),
                },
            },
        });

        nodes.push(PaletteNode {
            id: "trigger_email".to_string(),
            name: "Email Trigger".to_string(),
            description: "Trigger from email".to_string(),
            icon: "📧".to_string(),
            category: "Triggers".to_string(),
            node_type: NodeType::Trigger {
                trigger_type: TriggerType::Email,
                config: TriggerConfig {
                    params: HashMap::new(),
                    enabled: true,
                    filters: Vec::new(),
                },
            },
        });

        // Tool nodes - these would come from MCP registry in a real implementation
        nodes.push(PaletteNode {
            id: "tool_http_request".to_string(),
            name: "HTTP Request".to_string(),
            description: "Make HTTP API calls".to_string(),
            icon: "🌐".to_string(),
            category: "I/O Tools".to_string(),
            node_type: NodeType::Tool {
                tool_name: "http_request".to_string(),
                server_name: "http".to_string(),
                config: ToolConfig {
                    static_params: HashMap::new(),
                    input_mappings: HashMap::new(),
                    auto_execute: true,
                },
                visibility: ToolVisibility::AlwaysExternal,
            },
        });

        nodes.push(PaletteNode {
            id: "tool_github_create_issue".to_string(),
            name: "Create GitHub Issue".to_string(),
            description: "Create issues in GitHub".to_string(),
            icon: "🐙".to_string(),
            category: "I/O Tools".to_string(),
            node_type: NodeType::Tool {
                tool_name: "create_issue".to_string(),
                server_name: "github".to_string(),
                config: ToolConfig {
                    static_params: HashMap::new(),
                    input_mappings: HashMap::new(),
                    auto_execute: true,
                },
                visibility: ToolVisibility::AlwaysExternal,
            },
        });

        nodes.push(PaletteNode {
            id: "tool_slack_message".to_string(),
            name: "Send Slack Message".to_string(),
            description: "Send messages to Slack".to_string(),
            icon: "💬".to_string(),
            category: "I/O Tools".to_string(),
            node_type: NodeType::Tool {
                tool_name: "send_message".to_string(),
                server_name: "slack".to_string(),
                config: ToolConfig {
                    static_params: HashMap::new(),
                    input_mappings: HashMap::new(),
                    auto_execute: true,
                },
                visibility: ToolVisibility::AlwaysExternal,
            },
        });

        // Optional external tools (can be promoted from internal)
        nodes.push(PaletteNode {
            id: "tool_web_search".to_string(),
            name: "Web Search".to_string(),
            description: "Search the web (optional)".to_string(),
            icon: "🔍".to_string(),
            category: "Optional Tools".to_string(),
            node_type: NodeType::Tool {
                tool_name: "web_search".to_string(),
                server_name: "web".to_string(),
                config: ToolConfig {
                    static_params: HashMap::new(),
                    input_mappings: HashMap::new(),
                    auto_execute: true,
                },
                visibility: ToolVisibility::OptionalExternal,
            },
        });

        nodes
    }

    /// Toggle palette visibility
    pub fn toggle(&mut self) {
        self.is_open = !self.is_open;
    }

    /// Show the palette
    pub fn show(&mut self) {
        self.is_open = true;
    }

    /// Hide the palette
    pub fn hide(&mut self) {
        self.is_open = false;
    }
}

/// Initialize the node palette and add it to the global state
pub fn init_node_palette(document: &Document) -> Result<(), JsValue> {
    // Add palette toggle button to the canvas UI
    add_palette_toggle_button(document)?;

    // Initialize drag and drop handling for the canvas
    init_canvas_drop_handling(document)?;

    Ok(())
}

/// Add a toggle button to show/hide the node palette
fn add_palette_toggle_button(document: &Document) -> Result<(), JsValue> {
    // Check if button already exists
    if document.get_element_by_id("palette-toggle-btn").is_some() {
        return Ok(());
    }

    let button = document.create_element("button")?;
    button.set_id("palette-toggle-btn");
    button.set_inner_html("🎨 Nodes");
    button.set_attribute("style", "
        position: fixed;
        left: 20px;
        top: 20px;
        padding: 10px 16px;
        background: #3b82f6;
        color: white;
        border: none;
        border-radius: 6px;
        font-size: 14px;
        font-weight: 500;
        cursor: pointer;
        z-index: 1001;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
        transition: all 0.2s ease;
    ")?;

    // Add click handler
    let onclick = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: MouseEvent| {
        let document = web_sys::window().unwrap().document().unwrap();
        let palette = NodePalette::new();
        
        // Toggle palette visibility
        APP_STATE.with(|state| {
            let mut state = state.borrow_mut();
            
            // Create a simple toggle by checking if palette is visible
            if let Some(palette_elem) = document.get_element_by_id("node-palette") {
                let is_visible = palette_elem.get_attribute("style")
                    .map(|s| s.contains("display: block"))
                    .unwrap_or(false);
                
                let mut palette = NodePalette::new();
                palette.is_open = !is_visible;
                
                if let Err(e) = palette.render(&document) {
                    web_sys::console::error_1(&format!("Error rendering palette: {:?}", e).into());
                }
            } else {
                let mut palette = NodePalette::new();
                palette.is_open = true;
                
                if let Err(e) = palette.render(&document) {
                    web_sys::console::error_1(&format!("Error rendering palette: {:?}", e).into());
                }
            }
        });
    }) as Box<dyn FnMut(_)>);

    button.add_event_listener_with_callback("click", onclick.as_ref().unchecked_ref())?;
    onclick.forget();

    // Add to canvas container or body
    if let Some(canvas_container) = document.get_element_by_id("canvas-container") {
        canvas_container.append_child(&button)?;
    } else if let Some(body) = document.body() {
        body.append_child(&button)?;
    }

    Ok(())
}

/// Initialize drag and drop handling for the canvas
fn init_canvas_drop_handling(document: &Document) -> Result<(), JsValue> {
    if let Some(canvas) = document.get_element_by_id("canvas") {
        // Prevent default drag behavior
        let ondragover = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();
        }) as Box<dyn FnMut(_)>);

        let ondrop = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: DragEvent| {
            event.prevent_default();
            
            if let Some(data_transfer) = event.data_transfer() {
                if let Ok(node_data) = data_transfer.get_data("application/json") {
                    // Parse the node data
                    if let Ok(palette_node) = serde_json::from_str::<PaletteNode>(&node_data) {
                        // Get drop position
                        let rect = event.current_target().unwrap().dyn_into::<Element>().unwrap().get_bounding_client_rect();
                        let x = event.client_x() as f64 - rect.left();
                        let y = event.client_y() as f64 - rect.top();
                        
                        // Create node on canvas
                        APP_STATE.with(|state| {
                            let mut state = state.borrow_mut();
                            create_node_from_palette(&mut state, &palette_node, x, y);
                        });
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        canvas.add_event_listener_with_callback("dragover", ondragover.as_ref().unchecked_ref())?;
        canvas.add_event_listener_with_callback("drop", ondrop.as_ref().unchecked_ref())?;
        
        ondragover.forget();
        ondrop.forget();
    }

    Ok(())
}

/// Create a node on the canvas from a palette node
fn create_node_from_palette(state: &mut AppState, palette_node: &PaletteNode, x: f64, y: f64) {
    // Transform canvas coordinates to world coordinates
    let world_x = x / state.zoom_level + state.viewport_x;
    let world_y = y / state.zoom_level + state.viewport_y;
    
    // Create the node
    let node_id = state.add_node_with_agent(
        None, // No agent initially
        world_x,
        world_y,
        palette_node.node_type.clone(),
        palette_node.name.clone(),
    );
    
    web_sys::console::log_1(&format!("Created node {} at ({}, {})", node_id, world_x, world_y).into());
    
    // Mark state as dirty for re-render
    state.mark_dirty();
}
