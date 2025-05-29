//! Tiny toast / notification helper.
//! Creates a `#toast-root` container once per page and appends toast divs that
//! fade-out after a few seconds.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

#[derive(Debug, Clone, Copy)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

pub fn success(msg: &str) {
    show(msg, ToastKind::Success);
}

pub fn error(msg: &str) {
    show(msg, ToastKind::Error);
}

pub fn show(message: &str, kind: ToastKind) {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let root = ensure_root(&document);

    let toast = document.create_element("div").unwrap();
    toast.set_class_name("toast");
    match kind {
        ToastKind::Success => toast.class_list().add_1("toast-success").unwrap(),
        ToastKind::Error => toast.class_list().add_1("toast-error").unwrap(),
        ToastKind::Info => toast.class_list().add_1("toast-info").unwrap(),
    };
    toast.set_text_content(Some(message));

    // Prepend so newest appears on top.
    let _ = root.prepend_with_node_1(&toast);

    // Auto-remove after 4s.
    let toast_clone: HtmlElement = toast.unchecked_into();
    let cb = Closure::once_into_js(move || {
        let _ = toast_clone.parent_node().map(|p| p.remove_child(&toast_clone));
    });
    let _ = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 4000);

    ensure_styles(&document);
}

fn ensure_root(document: &Document) -> Element {
    if let Some(el) = document.get_element_by_id("toast-root") {
        el
    } else {
        let root = document.create_element("div").unwrap();
        root.set_id("toast-root");
        root.set_class_name("toast-root");
        document.body().unwrap().append_child(&root).unwrap();
        root
    }
}

fn ensure_styles(document: &Document) {
    if document.get_element_by_id("toast-styles").is_some() {
        return;
    }

    let css = "
.toast-root{position:fixed;top:16px;right:16px;display:flex;flex-direction:column;gap:8px;z-index:9999;font-family:Arial,Helvetica,sans-serif}
.toast{padding:10px 16px;border-radius:4px;color:#fff;box-shadow:0 2px 4px rgba(0,0,0,.1);opacity:0;animation:toast-in .2s forwards}
.toast-success{background:#16a34a}
.toast-error{background:#dc2626}
.toast-info{background:#2563eb}
/* spinner for buttons */
.spinner{display:inline-block;width:14px;height:14px;border:2px solid #fff;border-top-color:transparent;border-radius:50%;animation:spin 1s linear infinite;vertical-align:middle}
@keyframes spin{to{transform:rotate(360deg)}}
@keyframes toast-in{to{opacity:1}}
";

    let style = document.create_element("style").unwrap();
    style.set_id("toast-styles");
    style.set_text_content(Some(css));
    // Append to <head>
    if let Some(head) = document.query_selector("head").unwrap() {
        head.append_child(&style).unwrap();
    } else {
        // fallback – append to body
        document.body().unwrap().append_child(&style).unwrap();
    }
}
