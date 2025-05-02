//! Utility helpers shared across the WASM frontend.

use wasm_bindgen::JsValue;

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
