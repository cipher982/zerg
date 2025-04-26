//! Generic (non-modal) UI event handlers.
//!
//! Modal-specific logic was migrated to `components::agent_config_modal` as
//! part of the modal refactor (see `modal_refactor.md`).  The helpers here are
//! limited to toolbar buttons and other global widgets.

use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Event, MouseEvent};

use crate::{messages::Message, state::dispatch_global_message};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Entry point – call once after the base UI was rendered.  Attaches all
/// global (non-modal) event handlers.
pub fn setup_ui_event_handlers(document: &Document) -> Result<(), JsValue> {
    setup_auto_fit_button_handler(document)?;
    setup_center_view_handler(document)?;
    setup_clear_button_handler(document)?;
    setup_create_agent_button_handler(document)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Individual handlers
// ---------------------------------------------------------------------------

/// <input type="checkbox" id="auto-fit-toggle">
fn setup_auto_fit_button_handler(document: &Document) -> Result<(), JsValue> {
    if let Some(toggle) = document.get_element_by_id("auto-fit-toggle") {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
            dispatch_global_message(Message::ToggleAutoFit);
        }));
        toggle.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

/// Center view button
fn setup_center_view_handler(document: &Document) -> Result<(), JsValue> {
    if let Some(btn) = document.get_element_by_id("center-view") {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: MouseEvent| {
            dispatch_global_message(Message::CenterView);
        }));
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

/// Clear canvas button
fn setup_clear_button_handler(document: &Document) -> Result<(), JsValue> {
    if let Some(btn) = document.get_element_by_id("clear-button") {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
            let window = web_sys::window().expect("no global window exists");
            let confirm = window
                .confirm_with_message("Are you sure you want to clear all nodes? This cannot be undone.")
                .unwrap_or(false);
            if confirm {
                dispatch_global_message(Message::ClearCanvas);
                if let Err(err) = crate::storage::clear_storage() {
                    web_sys::console::error_1(&format!("Failed to clear storage: {:?}", err).into());
                }
            }
        }));
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}

/// "Create Agent" button – spawns a brand-new agent node.
fn setup_create_agent_button_handler(document: &Document) -> Result<(), JsValue> {
    if let Some(btn) = document.get_element_by_id("create-agent-button") {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: MouseEvent| {
            let name = format!("Agent {}", (js_sys::Math::random() * 10000.0).round());
            dispatch_global_message(Message::RequestCreateAgent {
                name,
                system_instructions: "You are a helpful AI assistant.".into(),
                task_instructions: "Respond to user questions accurately and concisely.".into(),
            });
        }));
        btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    Ok(())
}
