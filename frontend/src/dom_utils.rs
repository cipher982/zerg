//! dom_utils.rs – thin helper layer for repetitive DOM operations.
//!
//! Inspired by the checklist in `ui_robustness_task.md` this module exposes
//! small, **ergonomic** wrappers for common show / hide / activate patterns
//! without sprinkling `set_attribute("style", …)` calls across the code-base.

use wasm_bindgen::JsCast;
use web_sys::{Element, HtmlInputElement};

/// Remove the `hidden` attribute so the element becomes visible.
pub fn show(el: &Element) {
    let _ = el.remove_attribute("hidden");
}

/// Add `hidden="true"` so the element is not rendered.
pub fn hide(el: &Element) {
    // Browsers treat the *presence* of the attribute as the boolean so the
    // value does not matter, but we still write "true" for readability in
    // dev-tools.
    let _ = el.set_attribute("hidden", "true");
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
