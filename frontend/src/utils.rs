//! Utility helpers shared across the WASM frontend.

use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

/// Format a duration given in **milliseconds** into a short human-readable
/// string such as `"1 m 23 s"` or `"12 s"`.
///
/// The goal is to keep the output compact (fits into table cell) while still
/// being understandable.
pub fn format_duration_ms(ms: u64) -> String {
    let secs_total = ms / 1000;
    let minutes = secs_total / 60;
    let seconds = secs_total % 60;

    if minutes > 0 {
        format!("{} m {:02} s", minutes, seconds)
    } else {
        format!("{} s", seconds)
    }
}

/// Return the current timestamp in **milliseconds** since UNIX epoch.
///
/// We use JS Date here because it is available in browser/WASM without adding
/// heavy chrono dependencies.
pub fn now_ms() -> u64 {
    // `js_sys::Date::now()` returns f64 representing milliseconds since epoch.
    js_sys::Date::now() as u64
}

/// Helper for pretty-printing a floating USD cost value.
///
/// Rounds to 3–4 decimals depending on magnitude so small values like 0.0007
/// are not rendered as 0.00.
pub fn format_cost_usd(cost: f64) -> String {
    if cost >= 0.1 {
        format!("${:.2}", cost)
    } else if cost >= 0.01 {
        format!("${:.3}", cost)
    } else {
        format!("${:.4}", cost)
    }
}

/// Parse an ISO-8601 datetime string and return milliseconds since epoch.
/// Returns None when the string cannot be parsed.
pub fn parse_iso_ms(iso: &str) -> Option<u64> {
    let date = js_sys::Date::new(&JsValue::from_str(iso));
    let ms = date.get_time();
    if ms.is_nan() {
        None
    } else {
        Some(ms as u64)
    }
}

/// Capitalise the first letter of a &str.
pub fn capitalise_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ---------------------------------------------------------------------------
// Authentication helpers
// ---------------------------------------------------------------------------

/// Remove the persisted JWT from localStorage.
pub fn clear_stored_jwt() {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.remove_item("zerg_jwt");
        }
    }
}

/// Returns the JWT stored in localStorage, if any.
pub fn current_jwt() -> Option<String> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            return storage.get_item("zerg_jwt").ok().flatten();
        }
    }
    None
}

/// Fully logs the user out: clears JWT, closes WebSocket, shows login overlay.
#[wasm_bindgen]
pub fn logout() -> Result<(), JsValue> {
    clear_stored_jwt();

    // Update global state
    crate::state::APP_STATE.with(|st| {
        let mut s = st.borrow_mut();
        s.logged_in = false;

        // Close any active websocket connection
        let _ = s.ws_client.borrow_mut().close();
    });

    // Show overlay again
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // If overlay not present create it.
            if document.get_element_by_id("login-overlay").is_none() {
                let client_id = js_sys::Reflect::get(&window, &"GOOGLE_CLIENT_ID".into())
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();
                crate::components::auth::mount_login_overlay(&document, &client_id);
            } else {
                if let Some(el) = document.get_element_by_id("login-overlay") {
                    el.set_class_name("login-overlay");
                }
            }
        }
    }

    Ok(())
}

// wasm-bindgen tests ----------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_format_duration_ms() {
        assert_eq!(format_duration_ms(1_500), "1 s");
        assert_eq!(format_duration_ms(12_000), "12 s");
        assert_eq!(format_duration_ms(65_000), "1 m 05 s");
    }
}
