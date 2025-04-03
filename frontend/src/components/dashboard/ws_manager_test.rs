use wasm_bindgen_test::*;
use super::ws_manager::{DashboardWsManager, DASHBOARD_WS};
use crate::network::TopicManager;
use wasm_bindgen::JsValue;
use std::rc::Rc;
use std::cell::RefCell;
use crate::network::ws_client_v2::{ConnectionState, WsClientV2, IWsClient};
use crate::network::messages::WsMessage;
use serde_json::Value;

wasm_bindgen_test_configure!(run_in_browser);

// Mock WebSocket Client
#[derive(Clone)]
struct MockWsClient {
    state: Rc<RefCell<ConnectionState>>,
    sent_messages: Rc<RefCell<Vec<String>>>,
    on_connect: Option<Rc<RefCell<Box<dyn FnMut() + 'static>>>>,
    on_message: Option<Rc<RefCell<Box<dyn FnMut(serde_json::Value) + 'static>>>>,
    on_disconnect: Option<Rc<RefCell<Box<dyn FnMut() + 'static>>>>,
}

impl IWsClient for MockWsClient {
    fn connect(&mut self) -> Result<(), JsValue> {
        *self.state.borrow_mut() = ConnectionState::Connected;
        if let Some(callback_rc) = &self.on_connect {
            if let Ok(mut cb) = callback_rc.try_borrow_mut() {
                (*cb)();
            }
        }
        Ok(())
    }

    fn send_serialized_message(&self, message_json: &str) -> Result<(), JsValue> {
        if *self.state.borrow() != ConnectionState::Connected {
            return Err(JsValue::from_str("WebSocket is not connected"));
        }
        self.sent_messages.borrow_mut().push(message_json.to_string());
        Ok(())
    }

    fn connection_state(&self) -> ConnectionState {
        self.state.borrow().clone()
    }

    fn close(&mut self) -> Result<(), JsValue> {
        *self.state.borrow_mut() = ConnectionState::Disconnected;
        if let Some(callback_rc) = &self.on_disconnect {
            if let Ok(mut cb) = callback_rc.try_borrow_mut() {
                (*cb)();
            }
        }
        Ok(())
    }

    fn set_on_connect(&mut self, callback: Box<dyn FnMut() + 'static>) {
        self.on_connect = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_message(&mut self, callback: Box<dyn FnMut(serde_json::Value) + 'static>) {
        self.on_message = Some(Rc::new(RefCell::new(callback)));
    }

    fn set_on_disconnect(&mut self, callback: Box<dyn FnMut() + 'static>) {
        self.on_disconnect = Some(Rc::new(RefCell::new(callback)));
    }
}

impl MockWsClient {
    fn new() -> Self {
        Self {
            state: Rc::new(RefCell::new(ConnectionState::Disconnected)),
            sent_messages: Rc::new(RefCell::new(Vec::new())),
            on_connect: None,
            on_message: None,
            on_disconnect: None,
        }
    }

    fn simulate_receive_message(&self, message: Value) {
        if let Some(callback_rc) = &self.on_message {
            if let Ok(mut cb) = callback_rc.try_borrow_mut() {
                (*cb)(message);
            }
        }
    }
}

// Helper function to create a DashboardWsManager with mocked dependencies
fn create_mock_manager() -> (DashboardWsManager, Rc<RefCell<dyn IWsClient>>) {
    let mock_client = MockWsClient::new();
    let ws_client = Rc::new(RefCell::new(mock_client)) as Rc<RefCell<dyn IWsClient>>;
    let topic_manager = TopicManager::new(ws_client.clone());
    
    let manager = DashboardWsManager::new();
    // Replace the default WsClientV2 with our mock
    let manager = DashboardWsManager {
        topic_manager: RefCell::new(topic_manager),
        ws_client: ws_client.clone(),
    };
    
    (manager, ws_client)
}

#[wasm_bindgen_test]
async fn test_dashboard_ws_initialization() {
    let (manager, ws_client) = create_mock_manager();
    
    // Test initialization
    let result = manager.initialize();
    assert!(result.is_ok(), "Dashboard WebSocket initialization failed");
    
    // Verify the client is connected
    assert_eq!(
        ws_client.borrow().connection_state(),
        ConnectionState::Connected,
        "WebSocket should be connected after initialization"
    );
    
    // Verify subscription message was sent
    let client_ref = ws_client.borrow();
    let sent_messages = client_ref.sent_messages.borrow();
    assert!(!sent_messages.is_empty(), "No messages were sent during initialization");
    assert!(sent_messages[0].contains("agent:*"), "First message should be an agent subscription");
}

#[wasm_bindgen_test]
async fn test_dashboard_ws_cleanup() {
    let (manager, ws_client) = create_mock_manager();
    
    // Initialize first
    let init_result = manager.initialize();
    assert!(init_result.is_ok(), "Failed to initialize for cleanup test");
    
    // Test cleanup
    let cleanup_result = manager.cleanup();
    assert!(cleanup_result.is_ok(), "Dashboard WebSocket cleanup failed");
    
    // Verify the client is disconnected
    assert_eq!(
        ws_client.borrow().connection_state(),
        ConnectionState::Disconnected,
        "WebSocket should be disconnected after cleanup"
    );
}

#[wasm_bindgen_test]
async fn test_subscription_management() {
    let (manager, ws_client) = create_mock_manager();
    
    // Initialize the manager
    manager.initialize().expect("Failed to initialize manager");
    
    // Verify that we're subscribed to agent events
    let client_ref = ws_client.borrow();
    let sent_messages = client_ref.sent_messages.borrow();
    assert!(
        sent_messages.iter().any(|msg| msg.contains("agent:*")),
        "Should be subscribed to agent events"
    );
}

#[wasm_bindgen_test]
async fn test_reconnection_handling() {
    let (manager, ws_client) = create_mock_manager();
    
    // Set up a flag to track if on_connect was called
    let reconnected = Rc::new(RefCell::new(false));
    let reconnected_clone = reconnected.clone();
    
    // Set up the on_connect callback
    ws_client.borrow_mut().set_on_connect(Box::new(move || {
        *reconnected_clone.borrow_mut() = true;
    }));
    
    // Initialize the manager
    manager.initialize().expect("Failed to initialize manager");
    
    // Verify connection state
    assert_eq!(
        ws_client.borrow().connection_state(),
        ConnectionState::Connected,
        "WebSocket should be connected initially"
    );
    
    // Verify on_connect was called
    assert!(*reconnected.borrow(), "on_connect callback should have been called");
    
    // Simulate disconnect
    ws_client.borrow_mut().close().expect("Failed to close connection");
    
    assert_eq!(
        ws_client.borrow().connection_state(),
        ConnectionState::Disconnected,
        "WebSocket should be disconnected after close"
    );
}

#[wasm_bindgen_test]
async fn test_agent_event_handling() {
    let (manager, ws_client) = create_mock_manager();
    manager.initialize().expect("Failed to initialize manager");
    
    // Create a flag to track if the handler was called
    let handler_called = Rc::new(RefCell::new(false));
    let handler_called_clone = handler_called.clone();
    
    // Set up a handler for agent events by simulating a message that would trigger the handler
    let test_event = serde_json::json!({
        "type": "agent_created",
        "data": {
            "agent_id": 1,
            "timestamp": "2024-04-08T12:00:00Z"
        }
    });
    
    // Simulate receiving the event through the WebSocket
    ws_client.borrow().simulate_receive_message(test_event);
    
    if let Some(on_message) = &ws_client.borrow().on_message {
        if let Ok(mut callback) = on_message.try_borrow_mut() {
            (*callback)(test_event);
        }
    }
    
    // Verify the handler was called
    assert!(*handler_called.borrow(), "Agent event handler should have been called");
} 