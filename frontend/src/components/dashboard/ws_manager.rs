use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::JsValue;
// Remove direct dependency on specific types if we get them from AppState
// use crate::network::{WsClientV2, TopicManager};
 // Keep for borrowing
 // Keep for borrowing
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

            // Backend broadcasts as: { "type": "agent_event", "data": { .. }}.
            // Extract the inner payload so we can update local state immediately
            // instead of doing an extra round‑trip to the REST API (which may
            // already contain an *older* status if the agent finished very
            // quickly).
            let payload = if let Some(inner) = data.get("data") {
                inner.clone()
            } else {
                data.clone()
            };

            // -----------------------------------------------------------------
            // 1. Apply the delta contained in the event (`id`, `status`,
            //    `last_run_at`, ... ) directly to APP_STATE so the UI can flip
            //    the status badge to "Running" without waiting for the REST
            //    refresh.
            // -----------------------------------------------------------------
            if let (Some(id_val), Some(status_val)) = (
                payload.get("id"),
                payload.get("status"),
            ) {
                if let (Some(id_num), Some(status_str)) = (
                    id_val.as_u64(),
                    status_val.as_str(),
                ) {
                    crate::state::APP_STATE.with(|state_ref| {
                        let mut state = state_ref.borrow_mut();
                        if let Some(agent) = state.agents.get_mut(&(id_num as u32)) {
                            agent.status = Some(status_str.to_string());

                            // Also update last_run_at / next_run_at if present so
                            // the dashboard timestamp column refreshes instantly.
                            if let Some(ts) = payload.get("last_run_at").and_then(|v| v.as_str()) {
                                agent.last_run_at = Some(ts.to_string());
                            }
                            if let Some(ts) = payload.get("next_run_at").and_then(|v| v.as_str()) {
                                agent.next_run_at = Some(ts.to_string());
                            }
                        }
                    });

                    // Refresh UI optimistically so the change is visible right
                    // away.
                    if let Err(e) = crate::state::AppState::refresh_ui_after_state_change() {
                        web_sys::console::error_1(&format!("UI refresh error: {:?}", e).into());
                    }

                    // -----------------------------------------------------------------
                    // 2. Decide if we still want to hit the REST endpoint:
                    //    • For a transition *into* "running" we skip the
                    //      immediate reload so that we don't clobber the just
                    //      applied optimistic state with a stale "idle" one.
                    //    • For all other statuses (e.g. "idle", "deleted" …)
                    //      we perform a targeted reload of that single agent
                    //      object so that any additional fields changed by the
                    //      backend are picked up.
                    // -----------------------------------------------------------------
                    if status_str != "running" && status_str != "processing" {
                        crate::network::api_client::reload_agent(id_num as u32);
                    }

                    // Nothing more to do for handled events.
                    return;
                }
            }

            // Fall‑back behaviour – when payload is missing the expected
            // fields we keep the previous strategy of reloading everything.
            crate::network::api_client::load_agents();

            // Subscribe to the specific agent topic so we receive future
            // status updates immediately.
            // (Optional) could subscribe to this specific agent topic here if we
            // had a reference to the original handler. For simplicity we rely
            // on load_agents() + re‑initialisation to add subscriptions.
        }));

        self.agent_subscription_handler = Some(handler.clone());

        // Subscribe to each existing agent individually (wildcards not supported by backend).
        let agent_ids: Vec<u32> = crate::state::APP_STATE.with(|state_ref| {
            state_ref.borrow().agents.keys().cloned().collect()
        });

        for aid in agent_ids {
            let topic = format!("agent:{}", aid);
            // Cast the concrete closure type to the trait object type expected by TopicManager.
            let cloned: TopicHandler = Rc::clone(&handler) as TopicHandler;
            topic_manager.subscribe(topic, cloned)?;
        }
        Ok(())
    }

    /// Clean up WebSocket subscriptions for the dashboard.
    // Change signature to accept the trait object
    pub fn cleanup(&mut self, topic_manager_rc: Rc<RefCell<dyn ITopicManager>>) -> Result<(), JsValue> {
        let mut topic_manager = topic_manager_rc.borrow_mut();
        
        if let Some(handler) = self.agent_subscription_handler.take() {
             web_sys::console::log_1(&"DashboardWsManager: Cleaning up agent subscription handler".into());
            // We do not track which specific agent topics we subscribed to.
            // As a simple cleanup we iterate over the agents currently in
            // state and attempt to unsubscribe from their topics.
            let agent_ids: Vec<u32> = crate::state::APP_STATE.with(|state_ref| {
                state_ref.borrow().agents.keys().cloned().collect()
            });

            for aid in agent_ids {
                let topic = format!("agent:{}", aid);
                let _ = topic_manager.unsubscribe_handler(&topic, &handler);
            }
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