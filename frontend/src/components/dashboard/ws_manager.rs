use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
use crate::network::{WsClientV2, TopicManager};
use crate::network::ws_client_v2::IWsClient;
use crate::state::APP_STATE;

/// Manages WebSocket subscriptions and message handling for the Dashboard
pub struct DashboardWsManager {
    pub topic_manager: RefCell<TopicManager>,
    pub ws_client: Rc<RefCell<dyn IWsClient>>,
}

impl DashboardWsManager {
    /// Create a new DashboardWsManager instance
    pub fn new() -> Self {
        let ws_client = Rc::new(RefCell::new(WsClientV2::new_default()));
        let topic_manager = RefCell::new(TopicManager::new(ws_client.clone()));
        
        Self {
            topic_manager,
            ws_client,
        }
    }

    /// Initialize WebSocket connection and subscriptions
    pub fn initialize(&self) -> Result<(), JsValue> {
        // Connect WebSocket
        self.ws_client.borrow_mut().connect()?;

        // Subscribe to agent events
        self.subscribe_to_agent_events()?;

        Ok(())
    }

    /// Subscribe to agent-related events
    fn subscribe_to_agent_events(&self) -> Result<(), JsValue> {
        let mut topic_manager = self.topic_manager.borrow_mut();

        // Subscribe to all agent events using wildcard
        let handler = Rc::new(RefCell::new(|data: serde_json::Value| {
            web_sys::console::log_1(&format!("Received agent event: {:?}", data).into());
            
            // Handle different event types
            if let Some(event_type) = data.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    "agent_created" | "agent_updated" | "agent_deleted" => {
                        // Refresh agents from API
                        crate::network::api_client::load_agents();
                    },
                    _ => {
                        web_sys::console::warn_1(&format!("Unhandled agent event type: {}", event_type).into());
                    }
                }
            }
        }));

        topic_manager.subscribe("agent:*".to_string(), handler)?;
        Ok(())
    }

    /// Clean up WebSocket connection and subscriptions
    pub fn cleanup(&self) -> Result<(), JsValue> {
        // Close WebSocket connection
        self.ws_client.borrow_mut().close()?;
        Ok(())
    }

    #[cfg(test)]
    pub fn get_connection_state(&self) -> String {
        self.ws_client.borrow().connection_state().to_string()
    }

    #[cfg(test)]
    pub fn is_subscribed_to_topic(&self, topic: &str) -> bool {
        self.topic_manager.borrow().has_subscription(topic)
    }
}

// Create a singleton instance of the DashboardWsManager
thread_local! {
    pub static DASHBOARD_WS: RefCell<Option<DashboardWsManager>> = RefCell::new(None);
}

/// Initialize the dashboard WebSocket manager
pub fn init_dashboard_ws() -> Result<(), JsValue> {
    DASHBOARD_WS.with(|manager| {
        let mut manager = manager.borrow_mut();
        if manager.is_none() {
            let new_manager = DashboardWsManager::new();
            new_manager.initialize()?;
            *manager = Some(new_manager);
        }
        Ok(())
    })
}

/// Clean up the dashboard WebSocket manager
pub fn cleanup_dashboard_ws() -> Result<(), JsValue> {
    DASHBOARD_WS.with(|manager| {
        if let Some(manager) = manager.borrow_mut().take() {
            manager.cleanup()?;
        }
        Ok(())
    })
} 