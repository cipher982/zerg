//! Shared modal helper used by Agent Debug & Config modals.
//!
//! Keeps creation / show / hide logic in one place so feature modals don't
//! duplicate the same boilerplate.

use web_sys::{Document, Element, HtmlElement};
use wasm_bindgen::JsCast;

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
        // Add ARIA attributes for accessibility
        el.set_attribute("role", "dialog")?;
        el.set_attribute("aria-modal", "true")?;
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

/// Show the modal with focus management.
/// - Stores the currently focused element for later restoration
/// - Shows the modal
/// - Focuses the first interactive element within the modal
pub fn show(modal_backdrop: &Element) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        // Store the currently focused element
        let _previous_focus = dom_utils::store_active_element(&document);
        let _ = modal_backdrop.set_attribute("data-previous-focus", "stored");
        
        // Show the modal
        dom_utils::show(modal_backdrop);
        
        // Focus first interactive element
        let _ = dom_utils::focus_first_interactive(modal_backdrop);
        
        // Store previous focus element reference (we'll need a different approach for this)
        // For now, we'll rely on the browser's natural focus restoration
    }
}

/// Hide the modal with focus restoration.
/// - Hides the modal
/// - Restores focus to the previously focused element
pub fn hide(modal_backdrop: &Element) {
    dom_utils::hide(modal_backdrop);
    
    // For now, let the browser handle focus restoration naturally
    // In a more complete implementation, we would store and restore the previous focus
}

/// Show the modal with explicit focus management.
/// This version allows the caller to specify what element should receive focus.
pub fn show_with_focus(modal_backdrop: &Element, focus_selector: Option<&str>) {
    if let Some(document) = web_sys::window().and_then(|w| w.document()) {
        // Store the currently focused element
        let _previous_focus = dom_utils::store_active_element(&document);
        
        // Show the modal
        dom_utils::show(modal_backdrop);
        
        // Focus specific element or first interactive element
        if let Some(selector) = focus_selector {
            if let Ok(Some(element)) = modal_backdrop.query_selector(selector) {
                if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                    let _ = html_element.focus();
                    return;
                }
            }
        }
        
        // Fallback to first interactive element
        let _ = dom_utils::focus_first_interactive(modal_backdrop);
    }
}
