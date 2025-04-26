//! Minimal Agent Debug Modal (Phase 1).
//!
//! This is **read-only** and intentionally simple: we only show two tabs
//! – Overview (selected by default) and Raw JSON.  Later phases will extend
//! this component, so we keep the layout and IDs stable.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

use crate::state::{AgentDebugPane, DebugTab, AppState};

/// Public entry – create or refresh the modal based on current pane state.
pub fn render_agent_debug_modal(state: &AppState, document: &Document) -> Result<(), JsValue> {
    let Some(pane) = &state.agent_debug_pane else {
        return Ok(()); // nothing to render
    };

    // Ensure modal wrapper exists
    let modal = match document.get_element_by_id("agent-debug-modal") {
        Some(elem) => elem,
        None => {
            let elem = document.create_element("div")?;
            elem.set_id("agent-debug-modal");
            // Use the shared dark-theme modal styling so colours match the rest
            // of the application.  `.modal` is already defined in
            // `frontend/www/styles.css` and provides the backdrop & positioning.
            elem.set_class_name("modal");
            // The modal is shown immediately, so override the default
            // `display: none` from the CSS class.
            elem.set_attribute("style", "display: block;")?;

            // Add a content box
            let content = document.create_element("div")?;
            content.set_id("agent-debug-content");
            // Re-use the global `.modal-content` styling which already sets a
            // dark background (`#2a2a3a`), border, padding and responsive
            // width constraints.
            content.set_class_name("modal-content");

            elem.append_child(&content)?;

            // Close on background click
            let close_clone = elem.clone();
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_evt: web_sys::MouseEvent| {
                let _ = close_clone.set_attribute("style", "display:none;");
            }));
            elem.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();

            // Mount into the body
            document.body().unwrap().append_child(&elem)?;
            elem
        }
    };

    // Ensure modal visible – simply override the default `display: none`
    // applied by the `.modal` class.
    modal.set_attribute("style", "display: block;")?;

    // Stop background click from closing when clicking inside content
    if let Some(content) = document.get_element_by_id("agent-debug-content") {
        let prevent = Closure::<dyn FnMut(_)>::wrap(Box::new(|e: web_sys::MouseEvent| {
            e.stop_propagation();
        }));
        content.add_event_listener_with_callback("click", prevent.as_ref().unchecked_ref())?;
        prevent.forget();

        // Re-render inner HTML based on state
        content.set_inner_html("");

        // Header
        let header = document.create_element("h2")?;
        header.set_inner_html(&format!("Agent #{} Debug", pane.agent_id));
        content.append_child(&header)?;

        // Tabs (Overview / Raw JSON)
        let tabs_container = document.create_element("div")?;
        tabs_container.set_class_name("tab-container");

        let overview_tab = create_tab_button(document, "Overview", matches!(pane.active_tab, DebugTab::Overview))?;
        let raw_tab = create_tab_button(document, "Raw JSON", matches!(pane.active_tab, DebugTab::RawJson))?;

        tabs_container.append_child(&overview_tab)?;
        tabs_container.append_child(&raw_tab)?;
        content.append_child(&tabs_container)?;

        // Content area
        let body = document.create_element("div")?;
        body.set_class_name("tab-body");

        match pane.active_tab {
            DebugTab::Overview => {
                if pane.loading {
                    body.set_inner_html("Loading …");
                } else if let Some(details) = &pane.details {
                    let agent = &details.agent;
                    let list = document.create_element("ul")?;
                    list.set_class_name("overview-list");
                    list.set_inner_html(&format!(
                        "<li><strong>Name:</strong> {}</li>
                         <li><strong>Status:</strong> {}</li>
                         <li><strong>Model:</strong> {}</li>
                         <li><strong>Schedule:</strong> {}</li>",
                        agent.name,
                        agent.status.clone().unwrap_or_else(|| "-".into()),
                        agent.model.clone().unwrap_or_else(|| "-".into()),
                        agent.schedule.clone().unwrap_or_else(|| "-".into()),
                    ));
                    body.append_child(&list)?;
                } else {
                    body.set_inner_html("No data.");
                }
            }
            DebugTab::RawJson => {
                if pane.loading {
                    body.set_inner_html("Loading …");
                } else if let Some(details) = &pane.details {
                    let pre = document.create_element("pre")?;
                    pre.set_class_name("raw-json");
                    let json_str = serde_json::to_string_pretty(details).unwrap_or_default();
                    pre.set_text_content(Some(&json_str));
                    body.append_child(&pre)?;
                }
            }
        }

        content.append_child(&body)?;
    }

    Ok(())
}

fn create_tab_button(document: &Document, label: &str, active: bool) -> Result<Element, JsValue> {
    let btn = document.create_element("button")?;
    btn.set_inner_html(label);
    btn.set_class_name(if active { "tab-button active" } else { "tab-button" });
    Ok(btn)
}

/// Hide and remove the debug modal from the DOM.
pub fn hide_agent_debug_modal(document: &Document) -> Result<(), JsValue> {
    if let Some(elem) = document.get_element_by_id("agent-debug-modal") {
        elem.set_attribute("style", "display:none;")?;
    }
    Ok(())
}