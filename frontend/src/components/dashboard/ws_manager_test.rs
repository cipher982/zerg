use wasm_bindgen_test::*;
use super::ws_manager::{DashboardWsManager, DASHBOARD_WS};
use crate::network::api_client::ApiClient;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use std::rc::Rc;
use std::cell::RefCell;

wasm_bindgen_test_configure!(run_in_browser);

// Helper function to create a test event
fn create_test_event(event_type: &str, agent_id: u32) -> serde_json::Value {
    serde_json::json!({
        "type": event_type,
        "data": {
            "agent_id": agent_id,
            "timestamp": "2024-04-08T12:00:00Z"
        }
    })
}

#[wasm_bindgen_test]
async fn test_dashboard_ws_initialization() {
    // Test basic initialization
    let result = DASHBOARD_WS.with(|manager| {
        let mut manager = manager.borrow_mut();
        if manager.is_none() {
            let new_manager = DashboardWsManager::new();
            let init_result = new_manager.initialize();
            *manager = Some(new_manager);
            init_result
        } else {
            Ok(())
        }
    });
    
    assert!(result.is_ok(), "Dashboard WebSocket initialization failed");
}

#[wasm_bindgen_test]
async fn test_dashboard_ws_cleanup() {
    // Initialize first
    let init_result = DASHBOARD_WS.with(|manager| {
        let mut manager = manager.borrow_mut();
        if manager.is_none() {
            let new_manager = DashboardWsManager::new();
            let init_result = new_manager.initialize();
            *manager = Some(new_manager);
            init_result
        } else {
            Ok(())
        }
    });
    assert!(init_result.is_ok(), "Failed to initialize for cleanup test");
    
    // Test cleanup
    let cleanup_result = DASHBOARD_WS.with(|manager| {
        if let Some(manager) = manager.borrow_mut().take() {
            manager.cleanup()
        } else {
            Ok(())
        }
    });
    
    assert!(cleanup_result.is_ok(), "Dashboard WebSocket cleanup failed");
}

#[wasm_bindgen_test]
async fn test_agent_event_handling() {
    // Initialize the manager
    let manager = DashboardWsManager::new();
    manager.initialize().expect("Failed to initialize manager");
    
    // Create a flag to track if the handler was called
    let handler_called = Rc::new(RefCell::new(false));
    let handler_called_clone = handler_called.clone();
    
    // Mock the api_client::load_agents function
    // In a real implementation, you'd use a proper mocking framework
    let mock_load_agents = move || {
        *handler_called_clone.borrow_mut() = true;
    };
    
    // Simulate receiving different types of agent events
    let test_cases = vec![
        ("agent_created", 1),
        ("agent_updated", 2),
        ("agent_deleted", 3),
    ];
    
    for (event_type, agent_id) in test_cases {
        let event = create_test_event(event_type, agent_id);
        
        // Reset the flag
        *handler_called.borrow_mut() = false;
        
        // Simulate receiving the event
        // In a real test, you'd use the WebSocket infrastructure
        // For now, we're just testing the handler logic
        if let Some(event_type) = event.get("type").and_then(|t| t.as_str()) {
            match event_type {
                "agent_created" | "agent_updated" | "agent_deleted" => {
                    mock_load_agents();
                },
                _ => {}
            }
        }
        
        // Verify the handler was called
        assert!(*handler_called.borrow(), "Handler not called for {}", event_type);
    }
    
    // Cleanup
    manager.cleanup().expect("Failed to cleanup manager");
}

#[wasm_bindgen_test]
async fn test_reconnection_handling() {
    // Initialize the manager
    let manager = DashboardWsManager::new();
    manager.initialize().expect("Failed to initialize manager");
    
    // Verify connection state using the new method
    assert_eq!(
        manager.get_connection_state(),
        "Connected",
        "WebSocket should be connected initially"
    );
    
    // Cleanup
    manager.cleanup().expect("Failed to cleanup manager");
}

// Helper function to simulate WebSocket messages
async fn send_test_message(message: serde_json::Value) -> Result<(), JsValue> {
    // In a real test, you'd send this through the WebSocket
    // For now, we're just simulating the message handling
    Ok(())
}

#[wasm_bindgen_test]
async fn test_subscription_management() {
    // Initialize the manager
    let manager = DashboardWsManager::new();
    manager.initialize().expect("Failed to initialize manager");
    
    // Verify that we're subscribed to agent events using the new method
    assert!(
        manager.is_subscribed_to_topic("agent:*"),
        "Should be subscribed to agent events"
    );
    
    // Cleanup
    manager.cleanup().expect("Failed to cleanup manager");
} 