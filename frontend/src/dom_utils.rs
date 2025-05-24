//! dom_utils.rs – thin helper layer for repetitive DOM operations.
//!
//! Inspired by the checklist in `ui_robustness_task.md` this module exposes
//! small, **ergonomic** wrappers for common show / hide / activate patterns
//! without sprinkling `set_attribute("style", …)` calls across the code-base.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement, HtmlElement, Document};

/// Remove the `hidden` attribute so the element becomes visible.
pub fn show(el: &Element) {
    let _ = el.class_list().remove_1("hidden");
    let _ = el.class_list().add_1("visible");
}

/// Hide the element by toggling CSS classes.
pub fn hide(el: &Element) {
    let _ = el.class_list().remove_1("visible");
    let _ = el.class_list().add_1("hidden");
}

/// Mark a tab button as the active one (adds "tab-button active" class).
pub fn set_active(btn: &Element) {
    btn.set_class_name("tab-button active");
}

/// Remove the `active` modifier from a tab button.
pub fn set_inactive(btn: &Element) {
    btn.set_class_name("tab-button");
}

/// Fetch an `<input>` element by id and cast it to `HtmlInputElement`.
///
/// Panics when the element is missing or is of a different type.  Intended
/// for *fixed* DOM fragments that should always exist.
#[allow(dead_code)] // helper for vanilla DOM experiments
pub fn html_input(id: &str) -> HtmlInputElement {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .expect(&format!("<input id='{}'> not found or wrong type", id))
}

// ---------------------------------------------------------------------------
// Focus Management Utilities
// ---------------------------------------------------------------------------

/// Focus the first interactive element within the given container.
/// Returns true if an element was focused, false otherwise.
pub fn focus_first_interactive(container: &Element) -> bool {
    let focusable_selectors = [
        "input:not([disabled])",
        "button:not([disabled])",
        "textarea:not([disabled])",
        "select:not([disabled])",
        "a[href]",
        "[tabindex]:not([tabindex='-1'])"
    ];
    
    for selector in &focusable_selectors {
        if let Ok(Some(element)) = container.query_selector(selector) {
            if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                let _ = html_element.focus();
                return true;
            }
        }
    }
    false
}

/// Get all focusable elements within a container.
/// This is a simplified implementation that finds the first element of each type.
pub fn get_focusable_elements(container: &Element) -> Vec<HtmlElement> {
    let mut focusable = Vec::new();
    let focusable_selectors = [
        "input:not([disabled])",
        "button:not([disabled])",
        "textarea:not([disabled])",
        "select:not([disabled])",
        "a[href]",
        "[tabindex]:not([tabindex='-1'])"
    ];
    
    // Since we don't have query_selector_all, we'll just find the first of each type
    for selector in &focusable_selectors {
        if let Ok(Some(element)) = container.query_selector(selector) {
            if let Ok(html_element) = element.dyn_into::<HtmlElement>() {
                focusable.push(html_element);
            }
        }
    }
    focusable
}

/// Store the currently focused element for later restoration.
pub fn store_active_element(document: &Document) -> Option<HtmlElement> {
    document.active_element()
        .and_then(|el| el.dyn_into::<HtmlElement>().ok())
}

/// Restore focus to a previously stored element.
pub fn restore_focus(element: Option<HtmlElement>) {
    if let Some(el) = element {
        let _ = el.focus();
    }
}

// ---------------------------------------------------------------------------
// Unit tests (run with `cargo test --lib` in the frontend crate)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // wasm-bindgen unit tests have to run in a wasm-bindgen test
    // environment.  Here we only test the **string** manipulation helpers so
    // we guard with `#[cfg(not(target_arch = "wasm32"))]` to skip when
    // compiled for the browser.

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn class_helpers() {
        // Simple smoke test that the helpers compile on non-wasm targets.
        // Real DOM tests run in the Playwright suite.
        fn dummy(el: &mut web_sys::Element) {
            set_active(el);
            set_inactive(el);
        }

        // The dummy function is never called but ensures the generic code
        // path type-checks on non-wasm builds.
        let _ = dummy;
    }
}
