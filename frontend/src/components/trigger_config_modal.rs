//! Minimal, working trigger configuration modal for canvas trigger nodes.
//! Follows the modal and message dispatch patterns used in tool_config_modal.rs.

use wasm_bindgen::prelude::*;
use web_sys::{Document};
use crate::models::{Node, NodeType, TriggerType};
use crate::components::modal;
use crate::messages::Message;
use crate::state::dispatch_global_message;

/// Public API for the trigger config modal.
pub struct TriggerConfigModal;

impl TriggerConfigModal {
    /// Opens the trigger config modal for the given node.
    pub fn open(document: &Document, node: &Node) -> Result<(), JsValue> {
        let (modal, modal_content) = modal::ensure_modal(document, "trigger-config-modal")?;

        // Only allow opening for trigger nodes
        let (trigger_type, config) = match &node.node_type {
            NodeType::Trigger { trigger_type, config } => (trigger_type, config),
            _ => return Err(JsValue::from_str("Not a trigger node")),
        };

        // Build modal HTML
        let mut body_html = format!(
            "<div class='modal-header'><h2>Configure Trigger: {}</h2><span class='close' id='trigger-modal-close'>&times;</span></div>\
             <div class='modal-body'>",
            match trigger_type {
                TriggerType::Webhook => "Webhook",
                TriggerType::Schedule => "Schedule",
                TriggerType::Email => "Email",
                TriggerType::Manual => "Manual",
            }
        );

        // Minimal config UI for each trigger type (expand as needed)
        match trigger_type {
            TriggerType::Webhook => {
                body_html.push_str("<label class='block mb-2'>Webhook URL</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-webhook-url' type='text' value='{}' readonly />",
                    config.params.get("url").and_then(|v| v.as_str()).unwrap_or("Will be generated")
                ));
                body_html.push_str("<label class='block mb-2'>Secret</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-webhook-secret' type='text' value='{}' readonly />",
                    config.params.get("secret").and_then(|v| v.as_str()).unwrap_or("Will be generated")
                ));
            }
            TriggerType::Schedule => {
                body_html.push_str("<label class='block mb-2'>Cron Expression</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-schedule-cron' type='text' value='{}' />",
                    config.params.get("cron").and_then(|v| v.as_str()).unwrap_or("")
                ));
                body_html.push_str("<label class='block mb-2'>Timezone</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-schedule-tz' type='text' value='{}' />",
                    config.params.get("timezone").and_then(|v| v.as_str()).unwrap_or("UTC")
                ));
            }
            TriggerType::Email => {
                body_html.push_str("<label class='block mb-2'>Sender Filter</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-email-sender' type='text' value='{}' />",
                    config.params.get("sender").and_then(|v| v.as_str()).unwrap_or("")
                ));
                body_html.push_str("<label class='block mb-2'>Subject Filter</label>");
                body_html.push_str(&format!(
                    "<input class='input w-full mb-4' id='trigger-email-subject' type='text' value='{}' />",
                    config.params.get("subject").and_then(|v| v.as_str()).unwrap_or("")
                ));
            }
            TriggerType::Manual => {
                body_html.push_str("<p class='mb-4'>This trigger must be started manually.</p>");
            }
        }

        body_html.push_str("</div>\
            <div class='modal-buttons'><button class='btn' id='trigger-modal-cancel'>Cancel</button><button class='btn-primary' id='trigger-modal-save'>Save</button></div>");

        modal_content.set_inner_html(&body_html);

        // Event listeners for close/cancel/save
        if let Some(close_btn) = document.get_element_by_id("trigger-modal-close") {
            let modal = modal.clone();
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_event: web_sys::Event| {
                let _ = modal::hide(&modal);
            }));
            let _ = close_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }
        if let Some(cancel_btn) = document.get_element_by_id("trigger-modal-cancel") {
            let modal = modal.clone();
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_event: web_sys::Event| {
                let _ = modal::hide(&modal);
            }));
            let _ = cancel_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }
        if let Some(save_btn) = document.get_element_by_id("trigger-modal-save") {
            let node_id = node.node_id.clone();
            let trigger_type = trigger_type.clone();
            let document = document.clone();
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_event: web_sys::Event| {
                // Gather updated config from modal fields
                let mut params = serde_json::Map::new();
                match trigger_type {
                    TriggerType::Webhook => {
                        // Webhook params are readonly, so nothing to update
                    }
                    TriggerType::Schedule => {
                        let cron = document.get_element_by_id("trigger-schedule-cron")
                            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|input| input.value())
                            .unwrap_or_default();
                        let tz = document.get_element_by_id("trigger-schedule-tz")
                            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|input| input.value())
                            .unwrap_or("UTC".to_string());
                        params.insert("cron".to_string(), serde_json::Value::String(cron));
                        params.insert("timezone".to_string(), serde_json::Value::String(tz));
                    }
                    TriggerType::Email => {
                        let sender = document.get_element_by_id("trigger-email-sender")
                            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|input| input.value())
                            .unwrap_or_default();
                        let subject = document.get_element_by_id("trigger-email-subject")
                            .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok())
                            .map(|input| input.value())
                            .unwrap_or_default();
                        params.insert("sender".to_string(), serde_json::Value::String(sender));
                        params.insert("subject".to_string(), serde_json::Value::String(subject));
                    }
                    TriggerType::Manual => {}
                }
                // Dispatch message to update node config
                dispatch_global_message(Message::UpdateTriggerNodeConfig {
                    node_id: node_id.clone(),
                    params: serde_json::Value::Object(params),
                });
                // Close modal
                if let Some(modal) = document.get_element_by_id("trigger-config-modal") {
                    let _ = modal::hide(&modal);
                }
            }));
            let _ = save_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
            cb.forget();
        }

        modal::show(&modal);
        Ok(())
    }
}
