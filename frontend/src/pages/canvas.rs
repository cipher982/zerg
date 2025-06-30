// frontend/src/pages/canvas.rs
//
// This file contains the Canvas page component, responsible for
// mounting and unmounting the canvas view with its sidebar and toolbar.
//
use crate::models::{NodeType, TriggerConfig, TriggerType};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element};

/// Mount the canvas view by creating necessary DOM elements
/// This function is called when switching to the canvas view
pub fn mount_canvas(document: &Document) -> Result<(), JsValue> {
    web_sys::console::log_1(&"CANVAS: Starting mount".into());

    // Get app container for proper layout
    let app_container = document
        .get_element_by_id("app-container")
        .ok_or(JsValue::from_str("Could not find app-container"))?;

    // Add canvas-view class for flexbox layout
    web_sys::console::log_1(&"CANVAS: Setting app-container to canvas-view class".into());
    app_container.set_class_name("canvas-view");

    // First create/ensure agent shelf exists (sidebar)
    web_sys::console::log_1(&"CANVAS: Creating/refreshing agent shelf".into());

    // Agent shelf state is managed by workflow loading - no manual sync needed

    crate::components::agent_shelf::refresh_agent_shelf(document)?;

    // Ensure agent shelf is a direct child of app_container
    let agent_shelf = document
        .get_element_by_id("agent-shelf")
        .ok_or(JsValue::from_str("Agent shelf not found after refresh"))?;

    // If agent shelf is not already a child of app_container, append it
    if agent_shelf.parent_node().map_or(true, |p| {
        p.node_name() != "DIV"
            || p.dyn_ref::<Element>()
                .map_or(true, |e| e.id() != "app-container")
    }) {
        web_sys::console::log_1(&"CANVAS: Appending agent shelf to app container".into());
        app_container.append_child(&agent_shelf)?;
    }

    // Create main content area if it doesn't exist
    web_sys::console::log_1(&"CANVAS: Creating/finding main content area".into());
    let main_content = if let Some(content) = document.get_element_by_id("main-content-area") {
        content
    } else {
        let content = document.create_element("div")?;
        content.set_id("main-content-area");
        content.set_class_name("main-content-area");
        app_container.append_child(&content)?;
        content
    };

    // Removed: input panel (toolbar) creation and mounting; controls are now in workflow bar

    // Create canvas container if needed
    web_sys::console::log_1(&"CANVAS: Creating/finding canvas container".into());
    let canvas_container = if let Some(container) = document.get_element_by_id("canvas-container") {
        container
    } else {
        let container = document.create_element("div")?;
        container.set_id("canvas-container");
        container.set_class_name("canvas-container");

        // Create canvas element
        let canvas = document.create_element("canvas")?;
        canvas.set_id("node-canvas");
        container.append_child(&canvas)?;

        // Add container to main content
        main_content.append_child(&container)?;

        // Setup canvas once it's created
        crate::components::canvas_editor::setup_canvas(document)?;

        container
    };

    // Ensure canvas container is visible
    crate::dom_utils::show(&canvas_container);

    // Initialize/refresh the workflow switcher bar for this canvas view
    web_sys::console::log_1(&"CANVAS: Initializing workflow switcher".into());
    if let Err(e) = crate::components::workflow_switcher::init(document) {
        web_sys::console::error_1(
            &format!("Failed to initialize workflow switcher: {:?}", e).into(),
        );
    }

    // Initialize the execution results panel
    web_sys::console::log_1(&"CANVAS: Initializing execution results panel".into());
    // Removed execution results panel initialization.

    // Initialize the particle system for the canvas background via message dispatch
    crate::state::dispatch_global_message(crate::messages::Message::InitializeParticleSystem {
        width: crate::state::APP_STATE.with(|state| state.borrow().canvas_width),
        height: crate::state::APP_STATE.with(|state| state.borrow().canvas_height),
    });

    web_sys::console::log_1(&"CANVAS: Setup canvas drawing (no state borrowed)".into());

    // ------------------------------------------------------------------
    // Auto-place trigger node if canvas is empty (with delay to ensure canvas setup is complete)
    // ------------------------------------------------------------------
    let should_add_trigger = crate::state::APP_STATE.with(|s| {
        let st = s.borrow();
        web_sys::console::log_1(
            &format!("CANVAS: nodes in state = {}", st.workflow_nodes.len()).into(),
        );

        // Check if there's already a trigger node
        let has_trigger = st.workflow_nodes.values().any(|node| {
            matches!(
                node.get_semantic_type(),
                crate::models::NodeType::Trigger { .. }
            )
        });

        !has_trigger
    });

    if should_add_trigger {
        web_sys::console::log_1(&"CANVAS: Scheduling trigger node creation".into());

        // Use setTimeout to ensure canvas is fully rendered before adding trigger node
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let closure = Closure::wrap(Box::new(move || {
            web_sys::console::log_1(&"CANVAS: Adding delayed trigger node".into());

            // Double-check we still need a trigger node
            let still_need_trigger = crate::state::APP_STATE.with(|s| {
                let st = s.borrow();
                !st.workflow_nodes.values().any(|node| {
                    matches!(
                        node.get_semantic_type(),
                        crate::models::NodeType::Trigger { .. }
                    )
                })
            });

            if still_need_trigger {
                // Calculate position: top 1/3, center of viewport
                let (trigger_x, trigger_y) = crate::state::APP_STATE.with(|s| {
                    let st = s.borrow();
                    let viewport_width = if st.canvas_width > 0.0 {
                        st.canvas_width
                    } else {
                        800.0
                    };
                    let viewport_height = if st.canvas_height > 0.0 {
                        st.canvas_height
                    } else {
                        600.0
                    };

                    let x = st.viewport_x + (viewport_width / st.zoom_level) / 2.0 - 100.0; // Center horizontally
                    let y = st.viewport_y + (viewport_height / st.zoom_level) / 3.0 - 40.0; // Top 1/3
                    (x, y)
                });

                // Create default manual trigger node
                
                let trigger_config = TriggerConfig {
                    params: std::collections::HashMap::new(),
                    enabled: true,
                    filters: Vec::new(),
                };

                let trigger_node_type = NodeType::Trigger {
                    trigger_type: TriggerType::Manual,
                    config: trigger_config,
                };

                crate::state::dispatch_global_message(crate::messages::Message::AddNode {
                    text: "Manual Trigger".to_string(),
                    x: trigger_x,
                    y: trigger_y,
                    node_type: trigger_node_type,
                });

                web_sys::console::log_1(&"CANVAS: Trigger node created successfully".into());
            } else {
                web_sys::console::log_1(&"CANVAS: Trigger node already exists, skipping".into());
            }
        }) as Box<dyn FnMut()>);

        // Schedule for next tick to allow canvas to stabilize
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments(
                closure.as_ref().unchecked_ref(),
                100, // 100ms delay
                &js_sys::Array::new(),
            )
            .expect("Failed to set timeout");

        closure.forget(); // Important: prevent closure from being dropped
    }
    // Set up canvas resizing and drawing (without borrowing APP_STATE)
    if let Some(canvas_elem) = document.get_element_by_id("node-canvas") {
        if let Ok(canvas) = canvas_elem.dyn_into::<web_sys::HtmlCanvasElement>() {
            // Resize canvas explicitly without borrowing the app state
            if let Err(e) = crate::components::canvas_editor::resize_canvas(
                &canvas,
                crate::components::canvas_editor::AppStateRef::None,
            ) {
                web_sys::console::error_1(&format!("Failed to resize canvas: {:?}", e).into());
            }
        }
    }

    web_sys::console::log_1(&"CANVAS: Mount complete".into());
    Ok(())
}

/// Ensure a trigger node exists in the current canvas state
/// This function should be called every time the canvas view is activated
pub fn ensure_trigger_node_exists() {
    web_sys::console::log_1(&"CANVAS: Checking for trigger node".into());

    let should_add_trigger = crate::state::APP_STATE.with(|s| {
        let st = s.borrow();
        web_sys::console::log_1(
            &format!("CANVAS: nodes in state = {}", st.workflow_nodes.len()).into(),
        );

        // Check if there's already a trigger node
        let has_trigger = st.workflow_nodes.values().any(|node| {
            matches!(
                node.get_semantic_type(),
                crate::models::NodeType::Trigger { .. }
            )
        });

        web_sys::console::log_1(&format!("CANVAS: has trigger node = {}", has_trigger).into());
        !has_trigger
    });

    if should_add_trigger {
        web_sys::console::log_1(&"CANVAS: Adding default trigger node".into());

        // Calculate position: top 1/3, center of viewport
        let (trigger_x, trigger_y) = crate::state::APP_STATE.with(|s| {
            let st = s.borrow();
            let viewport_width = if st.canvas_width > 0.0 {
                st.canvas_width
            } else {
                800.0
            };
            let viewport_height = if st.canvas_height > 0.0 {
                st.canvas_height
            } else {
                600.0
            };

            let x = st.viewport_x + (viewport_width / st.zoom_level) / 2.0 - 100.0; // Center horizontally
            let y = st.viewport_y + (viewport_height / st.zoom_level) / 3.0 - 40.0; // Top 1/3
            (x, y)
        });

        // Create default manual trigger node
        
        let trigger_config = TriggerConfig {
            params: std::collections::HashMap::new(),
            enabled: true,
            filters: Vec::new(),
        };

        let trigger_node_type = NodeType::Trigger {
            trigger_type: TriggerType::Manual,
            config: trigger_config,
        };

        crate::state::dispatch_global_message(crate::messages::Message::AddNode {
            text: "Manual Trigger".to_string(),
            x: trigger_x,
            y: trigger_y,
            node_type: trigger_node_type,
        });
    } else {
        web_sys::console::log_1(&"CANVAS: Trigger node already exists, skipping creation".into());
    }
}

/// Unmount the canvas view by removing it from the DOM
/// This function is called when switching away from the canvas view
#[allow(dead_code)]
pub fn unmount_canvas(document: &Document) -> Result<(), JsValue> {
    web_sys::console::log_1(&"CANVAS: Starting unmount".into());

    // Remove workflow bar
    if let Some(workflow_bar) = document.get_element_by_id("workflow-bar") {
        web_sys::console::log_1(&"CANVAS: Removing workflow bar".into());
        if let Some(parent) = workflow_bar.parent_node() {
            parent.remove_child(&workflow_bar)?;
        }
    }

    // Remove input panel
    if let Some(panel) = document.get_element_by_id("canvas-input-panel") {
        web_sys::console::log_1(&"CANVAS: Removing input panel".into());
        if let Some(parent) = panel.parent_node() {
            parent.remove_child(&panel)?;
        }
    }

    // Remove canvas container
    if let Some(container) = document.get_element_by_id("canvas-container") {
        web_sys::console::log_1(&"CANVAS: Removing canvas container".into());
        if let Some(parent) = container.parent_node() {
            parent.remove_child(&container)?;
        }
    }

    // Remove main content area
    if let Some(content) = document.get_element_by_id("main-content-area") {
        web_sys::console::log_1(&"CANVAS: Removing main content area".into());
        if let Some(parent) = content.parent_node() {
            parent.remove_child(&content)?;
        }
    }

    // Remove agent shelf - critical to ensure it doesn't appear in dashboard view
    if let Some(shelf) = document.get_element_by_id("agent-shelf") {
        web_sys::console::log_1(&"CANVAS: Removing agent shelf".into());
        if let Some(parent) = shelf.parent_node() {
            parent.remove_child(&shelf)?;
        }
    }

    // Reset app-container class back to default (remove canvas-view)
    if let Some(app_container) = document.get_element_by_id("app-container") {
        web_sys::console::log_1(&"CANVAS: Resetting app container class".into());
        app_container.set_class_name("");
    }

    // Clear the particle system when unmounting the canvas via message dispatch
    crate::state::dispatch_global_message(crate::messages::Message::ClearParticleSystem);

    web_sys::console::log_1(&"CANVAS: Unmount complete".into());
    Ok(())
}

/// Get a reference to the canvas container element if it exists
#[allow(dead_code)]
pub fn get_canvas_container(document: &Document) -> Option<Element> {
    document.get_element_by_id("canvas-container")
}

/// Check if the canvas is currently mounted
pub fn is_canvas_mounted(document: &Document) -> bool {
    document.get_element_by_id("canvas-container").is_some()
}
