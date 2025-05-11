//! Simple *TabBar* helper – central place to create the standard markup for
//! `.tab-container` + `.tab-button` elements so modal implementations don’t
//! clone this snippet over and over.
//!
//! Phase-1 scope (May 2025): *pure markup factory*.
//!   • No internal state – the caller owns the “which tab is active” logic
//!     (usually inside `AppState`).
//!   • No default click-handlers – each consumer attaches its own listener so
//!     it can dispatch the appropriate `Message` variant.
//!
//! A later phase might add an enum-based constructor that auto-generates
//! handlers when given a closure factory, but that requires higher-order
//! generics that complicate wasm-bindgen lifetimes – out of scope for now.

use wasm_bindgen::prelude::*;
use web_sys::Document;

/// Create a `<div class="tab-container">` with child `<button>`s – one per
/// entry in `tabs`.
///
/// * `tabs` – slice of `(label, active)` tuples.  The caller is responsible
///   for **unordered** uniqueness and for attaching click listeners after the
///   function returns.
///
/// Returns the container element which can be directly inserted into the DOM
/// hierarchy.
pub fn build_tab_bar(
    document: &Document,
    tabs: &[(&str, bool)],
) -> Result<web_sys::Element, JsValue> {
    let container = document.create_element("div")?;
    container.set_class_name("tab-container");

    for (label, active) in tabs {
        let btn = document.create_element("button")?;
        btn.set_inner_html(label);
        btn.set_class_name(if *active { "tab-button active" } else { "tab-button" });
        container.append_child(&btn)?;
    }

    Ok(container)
}
