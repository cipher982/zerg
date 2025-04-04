use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
// Remove direct dependency on specific types if we get them from AppState
// use crate::network::{WsClientV2, TopicManager};
use crate::network::TopicManager; // Keep for borrowing
use crate::network::ws_client_v2::IWsClient; // Keep for borrowing
use crate::state::APP_STATE;
use crate::network::topic_manager::{TopicHandler, ITopicManager}; // Import Trait

/// Manages WebSocket subscriptions and message handling for the Dashboard lifecycle.
pub struct DashboardWsManager {
    // No longer stores ws_client or topic_manager fields, uses globals from AppState.
    // Store the handler Rc to allow unsubscribing later.
    // Make pub(crate) for test access
    pub(crate) agent_subscription_handler: Option<TopicHandler>,
}

impl DashboardWsManager {
    /// Create a new DashboardWsManager instance.
    /// It doesn't hold the ws_client or topic_manager, assumes they are managed globally.
    pub fn new() -> Self {
        Self {
            agent_subscription_handler: None,
        }
    }

    /// Initialize subscriptions for the dashboard.
    /// Assumes global WebSocket is already connected or will connect.
    pub fn initialize(&mut self) -> Result<(), JsValue> {
        // Get the global TopicManager from AppState
        let topic_manager_rc = APP_STATE.with(|state_ref| {
            // We still need the Rc<RefCell<dyn ITopicManager>> here
            state_ref.borrow().topic_manager.clone() as Rc<RefCell<dyn ITopicManager>>
        });
        self.subscribe_to_agent_events(topic_manager_rc)?;
        Ok(())
    }

    /// Subscribe to agent-related events using the provided ITopicManager.
    // Change signature to accept the trait object
    pub(crate) fn subscribe_to_agent_events(&mut self, topic_manager_rc: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
        // Borrowing and using the trait object works the same
        let mut topic_manager = topic_manager_rc.borrow_mut();

        // Handler remains largely the same, just operates on global state/API.
        let handler = Rc::new(RefCell::new(|data: serde_json::Value| {
            web_sys::console::log_1(&format!("Dashboard handler received agent event: {:?}", data).into());

            if let Some(event_type) = data.get("type").and_then(|t| t.as_str()) {
                match event_type {
                    // TODO (Refine): Use more granular updates based on event data
                    // instead of reloading all agents every time.
                    "agent_created" | "agent_updated" | "agent_deleted" => {
                        crate::network::api_client::load_agents();
                    },
                    _ => {
                        web_sys::console::warn_1(&format!("Dashboard handler: Unhandled agent event type: {}", event_type).into());
                    }
                }
            }
        }));

        self.agent_subscription_handler = Some(handler.clone());
        // Call subscribe via the trait object
        topic_manager.subscribe("agent:*".to_string(), handler)?;
        Ok(())
    }

    /// Clean up WebSocket subscriptions for the dashboard.
    // Change signature to accept the trait object
    pub fn cleanup(&mut self, topic_manager_rc: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();
        
        if let Some(handler) = self.agent_subscription_handler.take() {
             web_sys::console::log_1(&"DashboardWsManager: Cleaning up agent subscription handler".into());
            // Call unsubscribe_handler via the trait object
             topic_manager.unsubscribe_handler(&"agent:*".to_string(), &handler)?;
        } else {
            web_sys::console::warn_1(&"DashboardWsManager cleanup: No handler found to unsubscribe.".into());
        }
        Ok(())
    }

    // Test-specific helpers might need removal or adjustment as the manager no longer holds state.
    // #[cfg(test)]
    // pub fn get_connection_state(&self) -> String {
    //     // Cannot get state from self anymore
    // }

    // #[cfg(test)]
    // pub fn is_subscribed_to_topic(&self, topic: &str) -> bool {
    //    // Cannot check subscription state from self anymore
    //    // Need to check global TopicManager via AppState in tests
    // }
}

// Create a singleton instance of the DashboardWsManager
thread_local! {
    pub static DASHBOARD_WS: RefCell<Option<DashboardWsManager>> = RefCell::new(None);
}

/// Initialize the dashboard WebSocket manager singleton
pub fn init_dashboard_ws() -> Result<(), JsValue> {
    DASHBOARD_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if manager_opt.is_none() {
            web_sys::console::log_1(&"Initializing DashboardWsManager singleton...".into());
            let mut manager = DashboardWsManager::new();
            // initialize now uses APP_STATE internally, no need to pass manager here
            manager.initialize()?; 
            *manager_opt = Some(manager);
        } else {
            web_sys::console::warn_1(&"DashboardWsManager singleton already initialized.".into());
        }
        Ok(())
    })
}

/// Cleanup the dashboard WebSocket manager singleton subscriptions
pub fn cleanup_dashboard_ws() -> Result<(), JsValue> {
    DASHBOARD_WS.with(|cell| {
        let mut manager_opt = cell.borrow_mut();
        if let Some(manager) = manager_opt.as_mut() {
            web_sys::console::log_1(&"Cleaning up DashboardWsManager singleton...".into());
            // Get the trait object from APP_STATE to pass to cleanup
            let topic_manager_trait_rc = APP_STATE.with(|state_ref| {
                state_ref.borrow().topic_manager.clone() as Rc<RefCell<dyn ITopicManager>>
            });
            manager.cleanup(topic_manager_trait_rc)?; // Pass the trait object
        }
        Ok(())
    })
} 