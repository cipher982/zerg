//! Google Sign-In overlay (Stage 4).
//!
//! For the very first increment we only expose **minimal** functionality so that
//! other parts of the codebase can call [`init_google_signin`] at startup.  The
//! function injects a small JS glue-code snippet that initialises the Google
//! Identity library once it has loaded and forwards the *ID token* to the
//! Rust side where we call `ApiClient::google_auth_login()`.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, Element, Document};

use crate::network::api_client::ApiClient;
use crate::state::{APP_STATE};

/// Called by JS when `google.accounts.id.initialize()` fires the *credential*
/// callback.
#[wasm_bindgen]
pub async fn google_credential_received(id_token: String) {
    // Send id_token to backend → persist JWT → update logged_in flag.
    if let Err(e) = ApiClient::google_auth_login(&id_token).await {
        web_sys::console::error_1(&e);
        return;
    }

    // Update global state so UI can re-render.
    APP_STATE.with(|state_ref| {
        state_ref.borrow_mut().logged_in = true;
    });

    // Hide login overlay if present.
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            if let Some(el) = document.get_element_by_id("login-overlay") {
                el.set_class_name("hidden");
            }
        }
    }
}

/// Injects a simple *Sign-in with Google* button overlay.  Meant to be called
/// exactly once during application startup **if** the user is not yet logged
/// in.
pub fn mount_login_overlay(document: &Document, client_id: &str) {
    // Prevent duplicate overlays (shouldn’t really happen but stay safe).
    if document.get_element_by_id("login-overlay").is_some() {
        return;
    }

    let body = document.body().expect("<body> element missing");

    // Root overlay element -------------------------------------------------
    let overlay: HtmlElement = document
        .create_element("div")
        .expect("create overlay div")
        .dyn_into::<HtmlElement>()
        .unwrap();
    overlay.set_id("login-overlay");
    overlay.set_class_name("login-overlay");

    // Container for Google button ------------------------------------------
    let btn_holder: Element = document
        .create_element("div")
        .expect("create btn holder");
    btn_holder.set_attribute("id", "google-btn-holder").unwrap();
    overlay.append_child(&btn_holder).unwrap();

    body.append_child(&overlay).unwrap();

    // ---------------------------------------------------------------------
    // JS glue – call google.accounts.id.* API.  Doing this here keeps the
    // Rust side free from complicated `js_sys` reflection code.
    // ---------------------------------------------------------------------
    // SAFETY: The Google script is loaded in <head> with `async defer`, so by
    // the time our WASM runs it *should* be available.  We still wrap in
    // `js_sys::Function` checks but keep it terse.

    let init_js = format!(r#"
        (function() {{
            if (!window.google || !google.accounts || !google.accounts.id) {{
                console.error('Google Identity library not loaded');
                return;
            }}

            google.accounts.id.initialize({{
                client_id: '{}',
                callback: (resp) => {{
                    if (resp.credential) {{
                        // Forward to Rust → async so we can await inside WASM.
                        wasm_bindgen.google_credential_received(resp.credential);
                    }}
                }}
            }});

            google.accounts.id.renderButton(
                document.getElementById('google-btn-holder'),
                {{ theme: 'outline', size: 'large' }}
            );
        }})();
    "#, client_id);

    let _ = js_sys::eval(&init_js);
}
