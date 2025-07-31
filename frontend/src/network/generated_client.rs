use wasm_bindgen_futures::spawn_local;
use serde::{Serialize, Deserialize};

/// Contract-enforced models matching backend Pydantic schemas exactly
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkflowEdgeContract {
    pub from_node_id: String,  // ✅ Enforces consistent naming from Phase 1
    pub to_node_id: String,    // ✅ Enforces consistent naming from Phase 1
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkflowNodeContract {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub position: PositionContract,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PositionContract {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WorkflowDataContract {
    pub edges: Vec<WorkflowEdgeContract>,
    pub nodes: Vec<WorkflowNodeContract>,
}

/// Type-safe API client with contract validation
pub struct TypeSafeApiClient;

impl TypeSafeApiClient {
    /// Validates canvas data against contract before API call
    pub async fn update_workflow_canvas_data(canvas_data: serde_json::Value) -> Result<(), String> {
        // Validate structure matches contract
        let workflow_data: WorkflowDataContract = serde_json::from_value(canvas_data.clone())
            .map_err(|e| format!("Canvas data doesn't match contract: {}", e))?;

        // Validate edge fields use consistent naming
        for edge in &workflow_data.edges {
            if edge.from_node_id.is_empty() || edge.to_node_id.is_empty() {
                return Err("Edge missing required from_node_id or to_node_id fields".to_string());
            }
        }

        // Use existing WASM-compatible API client with validated data
        let payload = serde_json::json!({
            "canvas": canvas_data
        });
        let payload_str = payload.to_string();

        crate::network::ApiClient::patch_workflow_canvas_data(&payload_str).await
            .map(|_| ())
            .map_err(|e| format!("API call failed: {:?}", e))
    }
}

/// Convenience function for storage layer
pub fn save_canvas_data_typed(canvas_data: serde_json::Value) {
    web_sys::console::log_1(&"🚀 Sending canvas data via type-safe generated client".into());

    spawn_local(async move {
        match TypeSafeApiClient::update_workflow_canvas_data(canvas_data).await {
            Ok(_) => {
                web_sys::console::log_1(&"✅ Canvas data saved via type-safe client".into());
            }
            Err(e) => {
                web_sys::console::error_1(
                    &format!("❌ Type-safe API call failed: {}", e).into(),
                );
            }
        }
    });
}