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
    assert_eq!(count_triggers(&state), 0, "no triggers added by SelectWorkflow (moved to CurrentWorkflowLoaded)");

    crate::update::update(&mut state, Message::SelectWorkflow { workflow_id: 2 });
    assert_eq!(count_triggers(&state), 0, "still no triggers after reselection");
}

#[wasm_bindgen_test]
fn no_race_condition_between_select_and_load() {
    let mut state = AppState::new();

    // Simulate the race condition: SelectWorkflow followed by CurrentWorkflowLoaded
    let wf = empty_workflow(3, "wf-race");
    
    // First: SelectWorkflow processes empty workflow (should NOT create trigger anymore)
    state.workflows.insert(wf.id, wf.clone());
    crate::update::update(&mut state, Message::SelectWorkflow { workflow_id: 3 });
    assert_eq!(count_triggers(&state), 0, "SelectWorkflow should not create triggers");
    
    // Then: CurrentWorkflowLoaded processes same empty workflow from backend
    crate::update::update(&mut state, Message::CurrentWorkflowLoaded(wf.clone()));
    assert_eq!(count_triggers(&state), 1, "CurrentWorkflowLoaded should create exactly one trigger");
    
    // Rapid sequence should still result in only one trigger
    crate::update::update(&mut state, Message::SelectWorkflow { workflow_id: 3 });
    crate::update::update(&mut state, Message::CurrentWorkflowLoaded(wf));
    assert_eq!(count_triggers(&state), 1, "race condition should not create duplicate triggers");
}

