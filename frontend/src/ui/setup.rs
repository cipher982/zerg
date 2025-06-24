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
    title.set_attribute("id", "header-title")?;
    title.set_inner_html("AI Agent Platform");
    header.append_child(&title)?;

    // Status bar – connection + API status
    let status_bar = document.create_element("div")?;
    status_bar.set_class_name("status-bar");

    let status = document.create_element("div")?;
    status.set_id("global-status");
    status.set_class_name("yellow"); // initial colour
    status.set_inner_html("Status: Connecting");

    status_bar.append_child(&status)?;

    // -------------------------------------------------------------------
    // WebSocket connection badge (small coloured circle)
    // -------------------------------------------------------------------
    // A minimal visual indicator that changes colour based on the current
    // WebSocket connection state – green (connected), yellow (connecting),
    // red (disconnected / error).  Inline styles are used so the badge works
    // even without the full CSS bundle during local-dev or unit tests.

    let ws_badge = document.create_element("span")?;
    ws_badge.set_id("ws-badge");
    ws_badge.set_attribute(
        "style",
        "display:inline-block;width:10px;height:10px;border-radius:50%;\
         background:#f1c40f;margin-right:6px;vertical-align:middle;",
    )?;

    // Prepend badge before the textual status so they appear side-by-side.
    status_bar.insert_before(&ws_badge, Some(&status))?;

    // API packet counter (hidden LED / counter that flashes on WS packets).
    let api_status = document.create_element("div")?;
    api_status.set_id("global-api-status");
    api_status.set_class_name("packet-counter");
    api_status.set_inner_html("PKT: 00000000");
    status_bar.append_child(&api_status)?;

    // Add an (initially empty) layout status span that will be aligned to the
    // right by the flex layout (`justify-content: space-between`).  We *do
    // not* insert any text here so the bar looks identical to previous builds
    // until an error/warning is displayed.

    let layout_status = document.create_element("div")?;
    layout_status.set_id("global-layout-status");
    layout_status.set_class_name("");
    layout_status.set_inner_html("");

    status_bar.append_child(&layout_status)?;

    // Inject into DOM
    let body = document.body().ok_or(JsValue::from_str("No <body> element found"))?;
    body.append_child(&header)?;
    body.append_child(&status_bar)?;

    // -------------------------------------------------------------------
    // Reliability fix – ensure any *pending* canvas layout changes are
    // flushed to the backend when the user navigates away (tab close,
    // refresh, browser back gesture, etc.).  We hook into the `pagehide`
    // event which fires reliably across modern browsers whenever the page
    // is being unloaded *or* moved into the back/forward cache.  Using
    // `visibilitychange` alone is insufficient because pages kept alive in
    // bfcache do not trigger a regular unload.
    // -------------------------------------------------------------------

    if let Some(window) = web_sys::window() {
        // Closure is intentionally leaked (`forget`) because it must live
        // for the entire page lifetime – there is no corresponding remove
        // listener during normal operation.
        let on_pagehide = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            crate::state::APP_STATE.with(|s| {
                // Acquire a *unique* mutable borrow – panic if another borrow
                // is still active so the issue is caught during development.
                let mut st = s.borrow_mut();

                // Force-clear dragging flags so persistence logic emits the
                // final PATCH even if the user closes the tab mid-drag.
                st.canvas_dragging = false;

                // Force a persistence attempt even if `state_modified` was
                // already reset by the most recent animation tick.
                st.state_modified = true;

                let _ = st.save_if_modified();
            });
        });

        // SAFETY: `.unchecked_ref()` is safe because the closure type
        // matches the required `&Function` signature for addEventListener.
        let _ = window.add_event_listener_with_callback("pagehide", on_pagehide.as_ref().unchecked_ref());

        // Leak – see comment above.
        on_pagehide.forget();
    }

    Ok(())
}
