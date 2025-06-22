//! Generate (or refresh) the Pact contract file that the backend provider
//! verification test consumes.  This binary is **not** used by the browser
//! build – it runs on the host via `cargo run -p agent-platform-frontend --bin contract_capture`.

use std::fs;
use std::path::PathBuf;

use serde_json::json;

fn main() {
    // ------------------------------------------------------------------
    // 1. Build Pact JSON instance in-memory --------------------------------
    // ------------------------------------------------------------------
    let pact_json = json!({
        "consumer": { "name": "agent-frontend" },
        "provider": { "name": "zerg-backend" },
        "interactions": [
            {
                "description": "subscribe_thread → receive thread_history",
                "request": {
                    "type": "websocket",
                    "subtype": "text",
                    "body": {
                        "type": "subscribe_thread",
                        "thread_id": 1,
                        "message_id": "test-1"
                    }
                },
                "response": {
                    "body": {
                        "type": "thread_history",
                        "thread_id": 1,
                        "messages": []
                    }
                }
            },
            {
                "description": "subscribe to finished workflow_execution → receive execution_finished snapshot",
                "request": {
                    "type": "websocket",
                    "subtype": "text",
                    "body": {
                        "type": "subscribe",
                        "topics": ["workflow_execution:123"],
                        "message_id": "test-workflow-1"
                    }
                },
                "response": {
                    "body": {
                        "v": 1,
                        "type": "execution_finished",
                        "topic": "workflow_execution:123",
                        "req_id": "test-workflow-1",
                        "ts": 1234567890,
                        "data": {
                            "execution_id": 123,
                            "status": "success",
                            "error": null,
                            "duration_ms": 1500
                        }
                    }
                }
            }
        ],
        "metadata": {
            "pactSpecification": { "version": "4.0" }
        }
    });

    // ------------------------------------------------------------------
    // 2. Resolve output path ------------------------------------------------
    // ------------------------------------------------------------------
    // The binary lives in <repo>/frontend/.  We want to write to
    // <repo>/contracts/frontend-v1.json.  We therefore join the crate
    // directory with "../contracts".

    let mut out_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    out_path.pop(); // remove "frontend"
    out_path.push("contracts");
    fs::create_dir_all(&out_path).expect("create contracts dir");
    out_path.push("frontend-v1.json");

    // ------------------------------------------------------------------
    // 3. Write (pretty-printed) JSON --------------------------------------
    // ------------------------------------------------------------------
    fs::write(&out_path, serde_json::to_string_pretty(&pact_json).unwrap())
        .unwrap_or_else(|e| panic!("cannot write pact file {:?}: {e}", out_path));

    println!("✅ Pact contract written to {}", out_path.display());
}
