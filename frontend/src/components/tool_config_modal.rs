use wasm_bindgen::prelude::*;
use web_sys::{Document, HtmlInputElement, HtmlSelectElement};
use wasm_bindgen::JsCast;
use std::collections::HashMap;
use crate::models::{InputMapping, ToolConfig};

/// Minimal, working tool configuration modal for tool nodes only.
pub struct ToolConfigModal;

impl ToolConfigModal {
    pub fn open(
        document: &Document,
        node_id: String,
        node_text: &str,
        tool_description: &str,
        current_config: &ToolConfig,
    ) -> Result<(), JsValue> {
        let (modal, modal_content) = crate::components::modal::ensure_modal(document, "tool-config-modal")?;

        // Build modal HTML
        let mut body_html = format!(
            "<div class='modal-header'><h2>Configure: {}</h2><span class='close' id='tool-modal-close'>&times;</span></div>\
            <div class='modal-body'><p class='text-sm text-gray-400 mb-4'>{}</p>",
            node_text, tool_description
        );

        // Sample input fields (replace with real schema as needed)
        let sample_inputs = vec![
            ("title", "Title", true),
            ("body", "Body/Description", true),
            ("labels", "Labels", false),
            ("assignee", "Assignee", false),
        ];

        for (input_key, input_label, is_required) in &sample_inputs {
            let mapping_type = match current_config.input_mappings.get(*input_key) {
                Some(InputMapping::Static(_)) | None => "static",
                Some(InputMapping::FromNode { .. }) => "node_output",
            };
            let static_value = match current_config.input_mappings.get(*input_key) {
                Some(InputMapping::Static(val)) => val.as_str().unwrap_or("").to_string(),
                _ => "".to_string(),
            };
            let required_html = if *is_required { "<span class='required'>*</span>" } else { "" };
            body_html.push_str(&format!(
                "<div class='input-row'>
                    <label>{} {}</label>
                    <select class='mapping-type-selector' data-input-key='{}'>
                        <option value='static' {}>Static Value</option>
                        <option value='node_output' {}>From Node Output</option>
                    </select>
                    <input type='text' class='static-value-input' data-input-key='{}' value='{}' style='display:{}'>
                </div>",
                input_label,
                required_html,
                input_key,
                if mapping_type == "static" { "selected" } else { "" },
                if mapping_type == "node_output" { "selected" } else { "" },
                input_key,
                static_value,
                if mapping_type == "static" { "block" } else { "none" }
            ));
        }

        body_html.push_str("</div>\
            <div class='modal-buttons'><button class='btn' id='tool-modal-cancel'>Cancel</button><button class='btn-primary' id='tool-modal-save'>Save</button></div>");

        modal_content.set_inner_html(&body_html);

        // Event listeners for close/cancel/save
        if let Some(close_btn) = document.get_element_by_id("tool-modal-close") {
            let modal = modal.clone();
            let cb = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                let _ = crate::components::modal::hide(&modal);
            }));
            close_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }
        if let Some(cancel_btn) = document.get_element_by_id("tool-modal-cancel") {
            let modal = modal.clone();
            let cb = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                let _ = crate::components::modal::hide(&modal);
            }));
            cancel_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }
        if let Some(save_btn) = document.get_element_by_id("tool-modal-save") {
            let node_id = node_id.clone();
            let document = document.clone();
            let cb = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                // Collect values for each input
                let mut new_config = ToolConfig {
                    static_params: HashMap::new(),
                    input_mappings: HashMap::new(),
                    auto_execute: true,
                };
                let sample_inputs = vec!["title", "body", "labels", "assignee"];
                for input_key in &sample_inputs {
                    let mapping_type = document
                        .query_selector(&format!(".mapping-type-selector[data-input-key='{}']", input_key))
                        .ok()
                        .flatten()
                        .and_then(|sel| sel.dyn_into::<HtmlSelectElement>().ok())
                        .map(|sel| sel.value())
                        .unwrap_or_else(|| "static".to_string());
                    if mapping_type == "static" {
                        let value = document
                            .query_selector(&format!(".static-value-input[data-input-key='{}']", input_key))
                            .ok()
                            .flatten()
                            .and_then(|inp| inp.dyn_into::<HtmlInputElement>().ok())
                            .map(|inp| inp.value())
                            .unwrap_or_default();
                        new_config.input_mappings.insert(
                            input_key.to_string(),
                            InputMapping::Static(serde_json::Value::String(value)),
                        );
                    } else {
                        // For demo, just store a dummy mapping
                        new_config.input_mappings.insert(
                            input_key.to_string(),
                            InputMapping::FromNode {
                                node_id: "some_node".to_string(),
                                output_key: "result".to_string(),
                            },
                        );
                    }
                }
                // Dispatch a message to update state (do NOT mutate directly)
                crate::state::dispatch_global_message(crate::messages::Message::SaveToolConfig {
                    node_id: node_id.clone(),
                    config: new_config,
                });
                // Close modal
                if let Some(modal) = document.get_element_by_id("tool-config-modal") {
                    let _ = crate::components::modal::hide(&modal);
                }
            }));
            save_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        crate::components::modal::show(&modal);
        Ok(())
    }
}
