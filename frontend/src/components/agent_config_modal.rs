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
    /// Ensure the modal DOM exists.  If the element with id `agent-modal`
    /// is missing we **build the complete DOM tree here** (logic migrated
    /// from the former `ui::setup::create_agent_input_modal` helper).
    ///
    /// The operation is idempotent – if the DOM already exists we skip the
    /// heavy work and only make sure the listeners are attached once.
    pub fn init(document: &Document) -> Result<Self, JsValue> {
        // Build DOM on-demand
        if document.get_element_by_id("agent-modal").is_none() {
            Self::build_dom(document)?;
        }

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

    // ---------------------------------------------------------------------
    // DOM construction – migrated verbatim (with tiny tweaks) from
    // `ui::setup::create_agent_input_modal` so that the modal component is
    // now fully self-contained.
    // ---------------------------------------------------------------------
    fn build_dom(document: &Document) -> Result<(), JsValue> {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        // Create modal container
        let modal = document.create_element("div")?;
        modal.set_id("agent-modal");
        modal.set_class_name("modal");
        modal.set_attribute("style", "display: none;")?;

        // Create modal content
        let modal_content = document.create_element("div")?;
        modal_content.set_class_name("modal-content");

        // Header
        let modal_header = document.create_element("div")?;
        modal_header.set_class_name("modal-header");

        let modal_title = document.create_element("h2")?;
        modal_title.set_id("modal-title");
        modal_title.set_inner_html("Agent Configuration");

        let close_button = document.create_element("span")?;
        close_button.set_class_name("close");
        close_button.set_inner_html("&times;");
        close_button.set_id("modal-close");

        modal_header.append_child(&modal_title)?;
        modal_header.append_child(&close_button)?;

        // Tabs: Main / History
        let tab_container = document.create_element("div")?;
        tab_container.set_class_name("tab-container");

        let main_tab = document.create_element("button")?;
        main_tab.set_class_name("tab-button active");
        main_tab.set_id("main-tab");
        main_tab.set_inner_html("Main");

        let history_tab = document.create_element("button")?;
        history_tab.set_class_name("tab-button");
        history_tab.set_id("history-tab");
        history_tab.set_inner_html("History");

        tab_container.append_child(&main_tab)?;
        tab_container.append_child(&history_tab)?;

        // Main content
        let main_content = document.create_element("div")?;
        main_content.set_class_name("tab-content");
        main_content.set_id("main-content");

        // --- Agent name ---
        let name_label = document.create_element("label")?;
        name_label.set_inner_html("Agent Name:");
        name_label.set_attribute("for", "agent-name")?;

        let name_input = document.create_element("input")?;
        name_input.set_id("agent-name");
        name_input.set_attribute("type", "text")?;
        name_input.set_attribute("placeholder", "Enter agent name...")?;

        // --- System instructions ---
        let system_label = document.create_element("label")?;
        system_label.set_inner_html("System Instructions:");
        system_label.set_attribute("for", "system-instructions")?;

        let system_textarea = document.create_element("textarea")?;
        system_textarea.set_id("system-instructions");
        system_textarea.set_attribute("rows", "6")?;
        system_textarea.set_attribute("placeholder", "Enter system-level instructions for this agent...")?;

        // --- Task instructions ---
        let default_task_label = document.create_element("label")?;
        default_task_label.set_inner_html("Task Instructions:");
        default_task_label.set_attribute("for", "default-task-instructions")?;

        let default_task_textarea = document.create_element("textarea")?;
        default_task_textarea.set_id("default-task-instructions");
        default_task_textarea.set_attribute("rows", "4")?;
        default_task_textarea.set_attribute("placeholder", "Optional task instructions that will be used when running this agent. If empty, a default 'begin' prompt will be used.")?;

        // Append to main_content
        main_content.append_child(&name_label)?;
        main_content.append_child(&name_input)?;
        main_content.append_child(&system_label)?;
        main_content.append_child(&system_textarea)?;
        main_content.append_child(&default_task_label)?;
        main_content.append_child(&default_task_textarea)?;

        // --- Schedule controls ---
        let schedule_label = document.create_element("label")?;
        schedule_label.set_inner_html("Schedule (Cron expression):");
        schedule_label.set_attribute("for", "agent-schedule")?;

        let schedule_input = document.create_element("input")?;
        schedule_input.set_id("agent-schedule");
        schedule_input.set_attribute("type", "text")?;
        schedule_input.set_attribute("placeholder", "*/15 * * * * (min hour day month weekday)")?;

        let schedule_help = document.create_element("div")?;
        schedule_help.set_class_name("help-text");
        schedule_help.set_inner_html("Format: minute(0-59) hour(0-23) day(1-31) month(1-12) weekday(0-6). Example: */15 * * * * runs every 15 minutes.");
        schedule_help.set_attribute("style", "font-size: 0.6em; color: #666; margin-bottom: 10px;")?;

        let enable_label = document.create_element("label")?;
        enable_label.set_inner_html("Enable schedule:");

        let enable_checkbox = document.create_element("input")?;
        enable_checkbox.set_id("agent-run-on-schedule");
        enable_checkbox.set_attribute("type", "checkbox")?;

        // Append schedule controls
        main_content.append_child(&schedule_label)?;
        main_content.append_child(&schedule_input)?;
        main_content.append_child(&schedule_help)?;
        main_content.append_child(&enable_label)?;
        main_content.append_child(&enable_checkbox)?;

        // --- History content ---
        let history_content = document.create_element("div")?;
        history_content.set_class_name("tab-content");
        history_content.set_id("history-content");
        history_content.set_attribute("style", "display: none;")?;

        let history_container = document.create_element("div")?;
        history_container.set_id("history-container");
        history_container.set_inner_html("<p>No history available.</p>");

        history_content.append_child(&history_container)?;

        // --- Buttons ---
        let button_container = document.create_element("div")?;
        button_container.set_class_name("modal-buttons");

        let save_button = document.create_element("button")?;
        save_button.set_id("save-agent");
        save_button.set_inner_html("Save");

        button_container.append_child(&save_button)?;

        // Assemble content hierarchy
        modal_content.append_child(&modal_header)?;
        modal_content.append_child(&tab_container)?;
        modal_content.append_child(&main_content)?;
        modal_content.append_child(&history_content)?;
        modal_content.append_child(&button_container)?;

        modal.append_child(&modal_content)?;

        // Inject into DOM (body)
        let body = document.body().ok_or(JsValue::from_str("No body found"))?;
        body.append_child(&modal)?;

        // ------------------------------------------------------------------
        // UX: Close when clicking on backdrop (outside modal-content)
        // ------------------------------------------------------------------
        if modal.get_attribute("data-overlay-listener").is_none() {
            let modal_clone = modal.clone();
            let cb = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(move |evt: web_sys::MouseEvent| {
                if let Some(target) = evt.target() {
                    if let Ok(target_node) = target.dyn_into::<web_sys::Node>() {
                        if modal_clone.is_same_node(Some(&target_node)) {
                            if let Some(window) = web_sys::window() {
                                if let Some(doc) = window.document() {
                                    let _ = Self::close(&doc);
                                }
                            }
                        }
                    }
                }
            }));
            modal.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
            cb.forget();
            modal.set_attribute("data-overlay-listener", "true")?;
        }

        // Stop propagation inside content box
        if modal_content.get_attribute("data-stop-propagation").is_none() {
            let stopper = Closure::<dyn FnMut(web_sys::MouseEvent)>::wrap(Box::new(|e: web_sys::MouseEvent| {
                e.stop_propagation();
            }));
            modal_content.add_event_listener_with_callback("click", stopper.as_ref().unchecked_ref())?;
            stopper.forget();
            modal_content.set_attribute("data-stop-propagation", "true")?;
        }

        Ok(())
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
