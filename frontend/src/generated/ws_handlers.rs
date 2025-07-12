// AUTO-GENERATED FILE - DO NOT EDIT
// Generated from ws-protocol.yml at 2025-07-11T13:28:20.713811Z
//
// This file contains handler traits and message routing infrastructure.
// To update, modify the schema file and run: python scripts/generate-ws-types.py

use std::{cell::RefCell, rc::Rc};
use serde_json::Value;
use wasm_bindgen::JsValue;
use crate::generated::ws_messages::*;

// Handler trait definitions

/// Handler trait for Handles agent management, runs, and workflow execution events
pub trait DashboardHandler {
    fn handle_run_update(&self, data: RunUpdateData) -> Result<(), JsValue>;
    fn handle_agent_event(&self, data: AgentEventData) -> Result<(), JsValue>;
    fn handle_execution_finished(&self, data: ExecutionFinishedData) -> Result<(), JsValue>;
    fn handle_node_state(&self, data: NodeStateData) -> Result<(), JsValue>;
    fn handle_node_log(&self, data: NodeLogData) -> Result<(), JsValue>;
}

/// Handler trait for Handles thread messages and streaming events
pub trait ChatHandler {
    fn handle_thread_message(&self, data: ThreadMessageData) -> Result<(), JsValue>;
    fn handle_stream_start(&self, data: StreamStartData) -> Result<(), JsValue>;
    fn handle_stream_chunk(&self, data: StreamChunkData) -> Result<(), JsValue>;
    fn handle_stream_end(&self, data: StreamEndData) -> Result<(), JsValue>;
    fn handle_assistant_id(&self, data: AssistantIdData) -> Result<(), JsValue>;
}

// Message router implementation

/// Message router for dashboard handler
pub struct DashboardMessageRouter<T: DashboardHandler> {
    handler: Rc<RefCell<T>>,
}

impl<T: DashboardHandler> DashboardMessageRouter<T> {
    pub fn new(handler: Rc<RefCell<T>>) -> Self {
        Self { handler }
    }

    /// Route a message to the appropriate handler method
    pub fn route_message(&self, envelope: &Envelope) -> Result<(), JsValue> {
        let message_type = &envelope.message_type;
        let message_data = &envelope.data;

        match message_type.as_str() {
            "run_update" => {
                match serde_json::from_value::<RunUpdateData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_run_update(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse run_update: {}", e).into());
                        Ok(())
                    }
                }
            }
            "agent_event" => {
                match serde_json::from_value::<AgentEventData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_agent_event(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse agent_event: {}", e).into());
                        Ok(())
                    }
                }
            }
            "agent_created" => {
                match serde_json::from_value::<AgentEventData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_agent_event(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse agent_created: {}", e).into());
                        Ok(())
                    }
                }
            }
            "agent_updated" => {
                match serde_json::from_value::<AgentEventData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_agent_event(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse agent_updated: {}", e).into());
                        Ok(())
                    }
                }
            }
            "agent_deleted" => {
                match serde_json::from_value::<AgentEventData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_agent_event(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse agent_deleted: {}", e).into());
                        Ok(())
                    }
                }
            }
            "agent_state" => {
                match serde_json::from_value::<AgentEventData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_agent_event(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse agent_state: {}", e).into());
                        Ok(())
                    }
                }
            }
            "execution_finished" => {
                match serde_json::from_value::<ExecutionFinishedData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_execution_finished(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse execution_finished: {}", e).into());
                        Ok(())
                    }
                }
            }
            "node_state" => {
                match serde_json::from_value::<NodeStateData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_node_state(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse node_state: {}", e).into());
                        Ok(())
                    }
                }
            }
            "node_log" => {
                match serde_json::from_value::<NodeLogData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_node_log(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse node_log: {}", e).into());
                        Ok(())
                    }
                }
            }
            _ => {
                web_sys::console::warn_1(&format!("DashboardMessageRouter: Unknown message type: {}", message_type).into());
                Ok(())
            }
        }
    }
}

/// Message router for chat handler
pub struct ChatMessageRouter<T: ChatHandler> {
    handler: Rc<RefCell<T>>,
}

impl<T: ChatHandler> ChatMessageRouter<T> {
    pub fn new(handler: Rc<RefCell<T>>) -> Self {
        Self { handler }
    }

    /// Route a message to the appropriate handler method
    pub fn route_message(&self, envelope: &Envelope) -> Result<(), JsValue> {
        let message_type = &envelope.message_type;
        let message_data = &envelope.data;

        match message_type.as_str() {
            "thread_message" => {
                match serde_json::from_value::<ThreadMessageData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_thread_message(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse thread_message: {}", e).into());
                        Ok(())
                    }
                }
            }
            "thread_message_created" => {
                match serde_json::from_value::<ThreadMessageData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_thread_message(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse thread_message_created: {}", e).into());
                        Ok(())
                    }
                }
            }
            "stream_start" => {
                match serde_json::from_value::<StreamStartData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_stream_start(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse stream_start: {}", e).into());
                        Ok(())
                    }
                }
            }
            "stream_chunk" => {
                match serde_json::from_value::<StreamChunkData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_stream_chunk(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse stream_chunk: {}", e).into());
                        Ok(())
                    }
                }
            }
            "stream_end" => {
                match serde_json::from_value::<StreamEndData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_stream_end(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse stream_end: {}", e).into());
                        Ok(())
                    }
                }
            }
            "assistant_id" => {
                match serde_json::from_value::<AssistantIdData>(message_data.clone()) {
                    Ok(data) => self.handler.borrow().handle_assistant_id(data),
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse assistant_id: {}", e).into());
                        Ok(())
                    }
                }
            }
            _ => {
                web_sys::console::warn_1(&format!("ChatMessageRouter: Unknown message type: {}", message_type).into());
                Ok(())
            }
        }
    }
}

// Routing helper functions

/// Validate that a message type is handled by a specific handler
pub fn is_message_handled_by(message_type: &str, handler_type: &str) -> bool {
    // TODO: Generate validation based on schema message_routing
    match handler_type {
        "dashboard" => matches!(message_type, 
            "run_update" | "agent_event" | "agent_created" | "agent_updated" | 
            "agent_deleted" | "agent_state" | "execution_finished" | "node_state" | "node_log"
        ),
        "chat" => matches!(message_type,
            "thread_message" | "thread_message_created" | "stream_start" | 
            "stream_chunk" | "stream_end" | "assistant_id"
        ),
        _ => false,
    }
}

/// Extract handler type from topic pattern
pub fn get_handler_for_topic(topic: &str) -> Option<&'static str> {
    if topic.starts_with("agent:") || topic.starts_with("workflow_execution:") {
        Some("dashboard")
    } else if topic.starts_with("thread:") {
        Some("chat")
    } else {
        None
    }
}

