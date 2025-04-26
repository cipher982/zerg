//! Agent Configuration Modal – new component-style wrapper around the legacy
//! implementation that lived across multiple `ui/*` modules.  This is **step
//! one** of the ongoing refactor described in `modal_refactor.md` – we create
//! a single Rust module that owns the modal DOM and exposes a minimal public
//! API (`init`, `open`, `close`).
//!
//! For the moment we delegate most of the heavy lifting to the existing
//! helper functions in `crate::ui`, so behaviour is unchanged while we migrate
//! call-sites incrementally.  Future commits will move DOM creation and event
//! wiring here and delete the legacy helpers.

use wasm_bindgen::prelude::*;
use web_sys::Document;
use wasm_bindgen::JsCast;

use crate::state::APP_STATE;
use crate::constants::{DEFAULT_SYSTEM_INSTRUCTIONS, DEFAULT_TASK_INSTRUCTIONS};

/// Thin wrapper around the existing agent configuration modal.  The internal
/// fields will expand once we fully migrate DOM ownership.
#[derive(Clone, Debug)]
pub struct AgentConfigModal;

impl AgentConfigModal {
    /// Ensure the modal DOM exists.  At the moment we just call the existing
    /// `ui::setup::create_agent_input_modal` helper.  The function is
    /// idempotent, so it is safe to call multiple times.
    pub fn init(document: &Document) -> Result<Self, JsValue> {
        // Ensure the base DOM exists. UI startup already calls this once, but
        // it is idempotent so calling it again is safe.
        crate::ui::setup::create_agent_input_modal(document)?;

        // Attach local event listeners only once – use a data attribute on
        // the modal root to mark completion.
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            if modal.get_attribute("data-listeners-attached").is_none() {
                Self::attach_listeners(document)?;
                let _ = modal.set_attribute("data-listeners-attached", "true");
            }
        }
        Ok(Self)
    }

    /// Internal helper – adds event listeners to modal controls.  Calling it
    /// multiple times is safe because we guard with `data-listeners-attached`.
    fn attach_listeners(document: &Document) -> Result<(), JsValue> {
        use wasm_bindgen::closure::Closure;
        use web_sys::{Event, HtmlInputElement, HtmlTextAreaElement};

        // Helper: dispatch via global function
        let dispatch = |msg| crate::state::dispatch_global_message(msg);

        // Close (×) button
        if let Some(btn) = document.get_element_by_id("modal-close") {
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                dispatch(crate::messages::Message::CloseAgentModal);
            }));
            btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        // Tab switching – simple class toggle
        if let (Some(main_tab), Some(history_tab)) = (
            document.get_element_by_id("main-tab"),
            document.get_element_by_id("history-tab"),
        ) {
            // Main
            let cb_main = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                dispatch(crate::messages::Message::SwitchToMainTab);
            }));
            main_tab.add_event_listener_with_callback("click", cb_main.as_ref().unchecked_ref())?;
            cb_main.forget();

            // History
            let cb_hist = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                dispatch(crate::messages::Message::SwitchToHistoryTab);
            }));
            history_tab.add_event_listener_with_callback("click", cb_hist.as_ref().unchecked_ref())?;
            cb_hist.forget();
        }

        // Save
        if let Some(save_btn) = document.get_element_by_id("save-agent") {
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                let window = web_sys::window().unwrap();
                let document = window.document().unwrap();

                let name_value = document
                    .get_element_by_id("agent-name")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    .map(|i| i.value())
                    .unwrap_or_default();

                let system_instructions = document
                    .get_element_by_id("system-instructions")
                    .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
                    .map(|t| t.value())
                    .unwrap_or_default();

                let task_instructions = document
                    .get_element_by_id("default-task-instructions")
                    .and_then(|e| e.dyn_into::<HtmlTextAreaElement>().ok())
                    .map(|t| t.value())
                    .unwrap_or_default();

                // Schedule and flag
                let schedule_value_opt = document
                    .get_element_by_id("agent-schedule")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    .map(|i| {
                        let v = i.value();
                        if v.trim().is_empty() { None } else { Some(v) }
                    })
                    .flatten();

                let run_on_schedule_flag = document
                    .get_element_by_id("agent-run-on-schedule")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    .map(|i| i.checked())
                    .unwrap_or(false);

                let selected_model = crate::state::APP_STATE.with(|s| s.borrow().selected_model.clone());

                dispatch(crate::messages::Message::SaveAgentDetails {
                    name: name_value,
                    system_instructions,
                    task_instructions,
                    model: selected_model,
                    schedule: schedule_value_opt,
                    run_on_schedule: run_on_schedule_flag,
                });

                // feedback
                if let Some(btn) = document.get_element_by_id("save-agent") {
                    let original = btn.inner_html();
                    btn.set_inner_html("Saved!");
                    let btn_clone = btn.clone();
                    let reset = Closure::once_into_js(move || {
                        btn_clone.set_inner_html(&original);
                    });
                    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                        reset.as_ref().unchecked_ref(),
                        1500,
                    );
                }

                // auto-close
                let _ = Self::close(&document);
            }));
            save_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        // Send task
        if let Some(send_btn) = document.get_element_by_id("send-task") {
            let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                dispatch(crate::messages::Message::SendTaskToAgent);
            }));
            send_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
        }

        Ok(())
    }

    /// Open the modal for the given `node_id` (agent node ID).  Delegates to
    /// the legacy `ui::modals::open_agent_modal` for now.
    pub fn open(document: &Document, node_id: &str) -> Result<(), JsValue> {
        // Ensure DOM exists first
        let _ = Self::init(document);

        // The logic below is largely copied from the former
        // `ui::modals::open_agent_modal` implementation with only minimal
        // changes (namespacing, error handling).

        // --------------------------------------------------------------
        // 1. Gather data from AppState
        // --------------------------------------------------------------
        let (node_text, system_instructions, task_instructions, history_data) =
            APP_STATE.with(|state| {
                let state = state.borrow();

                if let Some(node) = state.nodes.get(node_id) {
                    (
                        node.text.clone(),
                        node.system_instructions().unwrap_or_default(),
                        node.task_instructions().unwrap_or_default(),
                        node.history().unwrap_or_default(),
                    )
                } else {
                    (String::new(), String::new(), String::new(), Vec::new())
                }
            });

        // Fetch schedule values if present
        let (schedule_value, run_on_schedule_flag) = APP_STATE.with(|state| {
            let state = state.borrow();
            if let Some(node) = state.nodes.get(node_id) {
                let agent_id_opt = node
                    .agent_id
                    .or_else(|| node_id.strip_prefix("agent-").and_then(|s| s.parse::<u32>().ok()));

                if let Some(agent_id) = agent_id_opt {
                    if let Some(agent) = state.agents.get(&agent_id) {
                        return (
                            agent.schedule.clone().unwrap_or_default(),
                            agent.run_on_schedule.unwrap_or(false),
                        );
                    }
                }
            }
            (String::new(), false)
        });

        // --------------------------------------------------------------
        // 2. Populate DOM elements
        // --------------------------------------------------------------

        // Store current node id for later
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            let _ = modal.set_attribute("data-node-id", node_id);
        }

        // Title
        if let Some(modal_title) = document.get_element_by_id("modal-title") {
            modal_title.set_inner_html(&format!("Agent: {}", node_text));
        }

        // Name input
        if let Some(name_elem) = document.get_element_by_id("agent-name") {
            if let Some(input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
                input.set_value(&node_text);
            }
        }

        // System instructions (with default fallback)
        let sys_val = if system_instructions.trim().is_empty() {
            DEFAULT_SYSTEM_INSTRUCTIONS.to_string()
        } else {
            system_instructions
        };

        if let Some(elem) = document.get_element_by_id("system-instructions") {
            if let Some(txt) = elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                txt.set_value(&sys_val);
            }
        }

        // Task instructions (with default fallback)
        let task_val = if task_instructions.trim().is_empty() {
            DEFAULT_TASK_INSTRUCTIONS.to_string()
        } else {
            task_instructions
        };

        if let Some(elem) = document.get_element_by_id("default-task-instructions") {
            if let Some(txt) = elem.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                txt.set_value(&task_val);
            }
        }

        // Schedule cron
        if let Some(elem) = document.get_element_by_id("agent-schedule") {
            if let Some(input) = elem.dyn_ref::<web_sys::HtmlInputElement>() {
                input.set_value(&schedule_value);
            }
        }

        // run_on_schedule checkbox
        if let Some(elem) = document.get_element_by_id("agent-run-on-schedule") {
            if let Some(cb) = elem.dyn_ref::<web_sys::HtmlInputElement>() {
                cb.set_checked(run_on_schedule_flag);
            }
        }

        // History tab content
        if let Some(container) = document.get_element_by_id("history-container") {
            if history_data.is_empty() {
                container.set_inner_html("<p>No history available.</p>");
            } else {
                container.set_inner_html("");

                for message in history_data {
                    if let Ok(msg_elem) = document.create_element("div") {
                        msg_elem.set_class_name(&format!("history-item {}", message.role));

                        if let Ok(p) = document.create_element("p") {
                            p.set_inner_html(&message.content);
                            let _ = msg_elem.append_child(&p);
                            let _ = container.append_child(&msg_elem);
                        }
                    }
                }
            }
        }

        // Finally, show the modal
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            modal.set_attribute("style", "display: block;")?;
        }

        Ok(())
    }

    /// Hide the modal.
    pub fn close(document: &Document) -> Result<(), JsValue> {
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            modal.set_attribute("style", "display: none;")?;
        }
        Ok(())
    }
}
