//! Shared modal helper used by Agent Debug & Config modals.
//!
//! Keeps creation / show / hide logic in one place so feature modals don’t
//! duplicate the same boilerplate.

use web_sys::{Document, Element};

use crate::dom_utils;

/// Ensure a `<div id="{id}" class="modal">…` exists in the DOM and return it.
/// The returned element is the **backdrop** container.  A child `<div
/// class="modal-content">` is created (and returned) if missing so callers
/// can append their specific inner markup.
///
/// Returns `(backdrop, content)`.
pub fn ensure_modal(document: &Document, id: &str) -> Result<(Element, Element), wasm_bindgen::JsValue> {
    // Backdrop first ------------------------------------------------------
    let backdrop = if let Some(el) = document.get_element_by_id(id) {
        el
    } else {
        let el = document.create_element("div")?;
        el.set_id(id);
        el.set_class_name("modal");
        dom_utils::hide(&el);
        document.body().unwrap().append_child(&el)?;
        el
    };

    // Content wrapper inside backdrop ------------------------------------
    let content = if let Some(el) = backdrop.query_selector(".modal-content")? {
        el
    } else {
        let el = document.create_element("div")?;
        el.set_class_name("modal-content");
        backdrop.append_child(&el)?;
        el
    };

    Ok((backdrop, content))
}

/// Show the modal (removes `hidden`).  Caller may still want to adjust
/// `style` attributes on inner markup.
pub fn show(modal_backdrop: &Element) {
    dom_utils::show(modal_backdrop);
}

/// Hide the modal backdrop (adds `hidden`).
pub fn hide(modal_backdrop: &Element) {
    dom_utils::hide(modal_backdrop);
}
