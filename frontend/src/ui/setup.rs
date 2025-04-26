//! UI setup helpers that are **not** specific to any modal component.
//! This module currently exposes a single public function `create_base_ui`
//! which inserts the static header & status-bar at the top of the document.

use wasm_bindgen::prelude::*;
use web_sys::Document;

/// Build the basic, always-visible chrome (header and status bar).
/// The function is idempotent – repeated invocations early-return once
/// the `header` element already exists in the DOM.
pub fn create_base_ui(document: &Document) -> Result<(), JsValue> {
    // Bail if a header already exists – assumes full base UI created earlier.
    if document.get_element_by_id("header").is_some() {
        return Ok(());
    }

    // Header
    let header = document.create_element("div")?;
    header.set_class_name("header");
    header.set_attribute("id", "header")?;

    let title = document.create_element("h1")?;
    title.set_inner_html("AI Agent Platform");
    header.append_child(&title)?;

    // Status bar – connection + API status
    let status_bar = document.create_element("div")?;
    status_bar.set_class_name("status-bar");

    let status = document.create_element("div")?;
    status.set_id("status");
    status.set_class_name("yellow"); // initial colour
    status.set_inner_html("Status: Connecting");

    let api_status = document.create_element("div")?;
    api_status.set_id("api-status");
    api_status.set_inner_html("API: Ready");

    status_bar.append_child(&status)?;
    status_bar.append_child(&api_status)?;

    // Inject into DOM
    let body = document.body().ok_or(JsValue::from_str("No <body> element found"))?;
    body.append_child(&header)?;
    body.append_child(&status_bar)?;

    Ok(())
}
