use wasm_bindgen_test::*;

use crate::messages::Message;
use crate::models::ApiWorkflow;
use crate::state::AppState;

wasm_bindgen_test_configure!(run_in_browser);

fn count_triggers(state: &AppState) -> usize {
    state
        .workflow_nodes
        .values()
        .filter(|n| matches!(n.get_semantic_type(), crate::models::NodeType::Trigger { .. }))
        .count()
}

fn empty_workflow(id: u32, name: &str) -> ApiWorkflow {
    ApiWorkflow {
        id,
        owner_id: 0,
        name: name.to_string(),
        description: Some("test".to_string()),
        canvas: serde_json::json!({ "nodes": [], "edges": [] }),
        is_active: true,
        created_at: None,
        updated_at: None,
    }
}

#[wasm_bindgen_test]
fn default_trigger_added_on_workflow_load_only_once() {
    let mut state = AppState::new();

    let wf = empty_workflow(1, "wf-load");
    crate::update::update(&mut state, Message::CurrentWorkflowLoaded(wf.clone()));
    assert_eq!(count_triggers(&state), 1, "exactly one trigger after first load");

    crate::update::update(&mut state, Message::CurrentWorkflowLoaded(wf));
    assert_eq!(count_triggers(&state), 1, "still exactly one trigger after reload");
}

#[wasm_bindgen_test]
fn default_trigger_added_on_select_workflow_once() {
    let mut state = AppState::new();

    let wf = empty_workflow(2, "wf-select");
    state.workflows.insert(wf.id, wf);

    crate::update::update(&mut state, Message::SelectWorkflow { workflow_id: 2 });
    assert_eq!(count_triggers(&state), 1, "exactly one trigger after first select");

    crate::update::update(&mut state, Message::SelectWorkflow { workflow_id: 2 });
    assert_eq!(count_triggers(&state), 1, "still exactly one trigger after reselection");
}

