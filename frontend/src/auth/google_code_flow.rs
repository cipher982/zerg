//! Google Identity Services *code-client* integration – Phase C (MVP stub).
//!
//! For the first milestone we don’t perform the real OAuth flow so that the
//! Rust/WASM build keeps working in environments where the GIS script is not
//! injected.  Instead we directly dispatch `Message::GmailConnected` which
//! flips the UI flag and unblocks e-mail trigger creation for demos.
//!
//! Once backend plumbing is ready this stub will:
//! 1) Call the JS `google.accounts.oauth2.initCodeClient` helper via
//!    `wasm_bindgen` externs.
//! 2) POST the received *auth_code* to `/api/auth/google/gmail` using
//!    `ApiClient`.
//! 3) On HTTP 200 dispatch `Message::GmailConnected`.

use wasm_bindgen::prelude::*;

/// Kick off the Gmail connect flow.
///
/// The current stub simply sets the flag without network IO.  This keeps the
/// UI path testable until the OAuth integration lands.
pub fn initiate_gmail_connect() {
    web_sys::console::log_1(&"[stub] initiate_gmail_connect called".into());

    // Immediately mark connected so the UI updates.
    crate::state::dispatch_global_message(crate::messages::Message::GmailConnected);
}

// In the real implementation we’ll expose these bindings:
//
// #[wasm_bindgen(module = "@google-identity/oauth2")]
// extern "C" {
//     #[wasm_bindgen(js_name = initCodeClient)]
//     fn init_code_client(params: &JsValue) -> JsValue;
// }
