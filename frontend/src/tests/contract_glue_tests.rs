use wasm_bindgen_test::*;

use serde_json::json;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn contract_glue_flattens_typed_trigger_meta() {
    let canvas = json!({
        "nodes": [
            {
                "id": "n1",
                "type": "Trigger",
                "position": { "x": 0.0, "y": 0.0 },
                "config": {
                    "trigger": {
                        "type": "Email",
                        "config": { "enabled": true, "params": {"foo": "bar"}, "filters": [] }
                    }
                }
            }
        ],
        "edges": []
    });

    let out = crate::network::contract_glue::normalize_canvas_for_api(canvas)
        .expect("normalize ok");
    let out_val = serde_json::to_value(&out).unwrap();

    let node = out_val.get("nodes").and_then(|n| n.get(0)).unwrap();
    assert_eq!(node.get("type").and_then(|v| v.as_str()), Some("trigger"));

    let cfg = node.get("config").unwrap();
    assert!(cfg.get("trigger").is_none(), "typed trigger meta removed");
    assert_eq!(cfg.get("trigger_type").and_then(|v| v.as_str()), Some("email"));
    assert_eq!(cfg.get("params").unwrap().get("foo").and_then(|v| v.as_str()), Some("bar"));
}

