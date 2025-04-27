//! A vertical "Agent Shelf" component that lists all agents and allows users to quickly
//! place an agent onto the canvas.  For the first milestone we implement it as a simple
//! clickable list (no true drag-and-drop yet). Clicking an agent pill dispatches a
//! `Message::AddAgentNode` with x = 0, y = 0 so the node is centred by existing logic.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::state::{APP_STATE, dispatch_global_message};
use crate::messages::Message;
use crate::models::NodeType;

/// Public helper that (re-)creates the Agent Shelf DOM element and appends it to the
/// `<div id="app-container">` root if it isn't present yet.
pub fn refresh_agent_shelf(document: &Document) -> Result<(), JsValue> {
    if let Some(existing) = document.get_element_by_id("agent-shelf") {
        // Shelf already exists – just repopulate the list.
        populate_agent_shelf(document, &existing)?;
    } else {
        // First render – create the root element and append to the main container.
        let shelf_el = create_root_element(document)?;
        populate_agent_shelf(document, &shelf_el)?;

        // Try to append next to the canvas inside `app-container` for now.
        // If the container is missing (unlikely) fall back to <body>.
        if let Some(parent) = document.get_element_by_id("app-container") {
            // Insert as the first child so it sits on the left side in flex layouts.
            let _ = parent.insert_before(&shelf_el, parent.first_child().as_ref());
        } else if let Some(body) = document.body() {
            body.append_child(&shelf_el)?;
        }
    }

    Ok(())
}

/// Internal – create the outer `<div id="agent-shelf">` container.
fn create_root_element(document: &Document) -> Result<Element, JsValue> {
    let el = document.create_element("div")?;
    el.set_id("agent-shelf");
    // No inline styles needed - using CSS now
    Ok(el)
}

/// Internal – clears and repopulates the shelf with the current set of agents.
fn populate_agent_shelf(document: &Document, shelf_el: &Element) -> Result<(), JsValue> {
    // Clear current children.
    shelf_el.set_inner_html("");

    // Borrow state to iterate agents.
    APP_STATE.with(|state| {
        let state = state.borrow();
        for agent in state.agents.values() {
            if let Some(agent_id) = agent.id {
                // Create a simple pill element.
                let pill: HtmlElement = document.create_element("div").unwrap().dyn_into().unwrap();
                pill.set_class_name("agent-pill");
                pill.set_inner_text(&agent.name);
                pill.set_attribute("data-agent-id", &agent_id.to_string()).unwrap();
                // No inline styles needed - using CSS now
                
                // Clone data for the closure.
                let name_cloned = agent.name.clone();
                let closure = Closure::<dyn FnMut(_)>::new(move |_: web_sys::MouseEvent| {
                    // Dispatch message to add a node at (0,0) which the existing logic
                    // will reinterpret as "centre on viewport" for AgentIdentity nodes.
                    dispatch_global_message(Message::AddAgentNode {
                        agent_id: Some(agent_id),
                        x: 0.0,
                        y: 0.0,
                        node_type: NodeType::AgentIdentity,
                        text: name_cloned.clone(),
                    });
                });
                pill.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
                closure.forget(); // Avoid dropping the closure – intentional memory leak.

                // Append to shelf.
                shelf_el.append_child(&pill).unwrap();
            }
        }
    });

    Ok(())
}
