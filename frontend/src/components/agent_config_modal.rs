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
use web_sys::Element;
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

        // --- NEW: Schedule controls (Frequency builder) ---
        let sched_container = document.create_element("div")?;
        sched_container.set_id("schedule-controls");

        // Frequency dropdown
        let freq_label = document.create_element("label")?;
        freq_label.set_inner_html("Frequency:");
        freq_label.set_attribute("for", "sched-frequency")?;

        let freq_select = document.create_element("select")?;
        freq_select.set_id("sched-frequency");

        let frequencies = [
            ("none", "Not scheduled"),
            ("minutes", "Every N minutes"),
            ("hourly", "Hourly"),
            ("daily", "Daily"),
            ("weekly", "Weekly"),
            ("monthly", "Monthly"),
        ];

        for (value, label) in &frequencies {
            let option = document.create_element("option")?;
            option.set_attribute("value", value)?;
            option.set_inner_html(label);
            freq_select.append_child(&option)?;
        }

        // Create labeled containers for each input
        let create_input_container = |id: &str, label_text: &str, min: &str, max: &str, default_val: &str| -> Result<Element, JsValue> {
            let container = document.create_element("div")?;
            container.set_id(&format!("{}-container", id));
            container.set_class_name("schedule-input-container");
            container.set_attribute("style", "display: none;")?;

            let label = document.create_element("label")?;
            label.set_inner_html(label_text);
            label.set_attribute("for", id)?;

            let input = document.create_element("input")?;
            input.set_id(id);
            input.set_attribute("type", "number")?;
            input.set_attribute("min", min)?;
            input.set_attribute("max", max)?;
            input.set_attribute("placeholder", default_val)?;
            input.set_attribute("value", default_val)?;  // Set actual default value

            container.append_child(&label)?;
            container.append_child(&input)?;
            Ok(container)
        };

        // Special function for weekday select
        let create_weekday_container = |id: &str, label_text: &str, default_day: u8| -> Result<Element, JsValue> {
            let container = document.create_element("div")?;
            container.set_id(&format!("{}-container", id));
            container.set_class_name("schedule-input-container");
            container.set_attribute("style", "display: none;")?;

            let label = document.create_element("label")?;
            label.set_inner_html(label_text);
            label.set_attribute("for", id)?;

            let select = document.create_element("select")?;
            select.set_id(id);

            let days = [
                (0, "Sunday"),
                (1, "Monday"),
                (2, "Tuesday"),
                (3, "Wednesday"),
                (4, "Thursday"),
                (5, "Friday"),
                (6, "Saturday"),
            ];

            for (value, name) in &days {
                let option = document.create_element("option")?;
                option.set_attribute("value", &value.to_string())?;
                if *value == default_day {
                    option.set_attribute("selected", "true")?;
                }
                option.set_inner_html(name);
                select.append_child(&option)?;
            }

            container.append_child(&label)?;
            container.append_child(&select)?;
            Ok(container)
        };

        // Create containers for each input type
        let interval_container = create_input_container(
            "sched-interval",
            "Run every N minutes (1-59):",
            "1",
            "59",
            "15"
        )?;

        let hour_container = create_input_container(
            "sched-hour",
            "Hour (0-23):",
            "0",
            "23",
            "9"
        )?;

        let minute_container = create_input_container(
            "sched-minute",
            "Minute (0-59):",
            "0",
            "59",
            "0"
        )?;

        let weekday_container = create_weekday_container(
            "sched-weekday",
            "Day of Week:",
            1  // Default to Monday (1)
        )?;

        let day_container = create_input_container(
            "sched-day",
            "Day of Month (1-31):",
            "1",
            "31",
            "15"
        )?;

        // Summary line
        let summary_div = document.create_element("div")?;
        summary_div.set_id("sched-summary");
        summary_div.set_attribute("style", "font-size: 0.9em; color: #555; margin-top: 6px;")?;
        summary_div.set_inner_html("No schedule set");

        // Assemble schedule container
        sched_container.append_child(&freq_label)?;
        sched_container.append_child(&freq_select)?;
        sched_container.append_child(&interval_container)?;
        sched_container.append_child(&hour_container)?;
        sched_container.append_child(&minute_container)?;
        sched_container.append_child(&weekday_container)?;
        sched_container.append_child(&day_container)?;
        sched_container.append_child(&summary_div)?;

        // Set default value "none"
        if let Some(select_el) = freq_select.dyn_ref::<web_sys::HtmlSelectElement>() {
            select_el.set_value("none");
        }

        // Append schedule container to main_content
        main_content.append_child(&sched_container)?;

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

                // NEW schedule builder: read controls and convert to cron
                let schedule_value_opt = {
                    let freq_sel_val = document
                        .get_element_by_id("sched-frequency")
                        .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
                        .map(|i| i.value());

                    match freq_sel_val.as_deref() {
                        Some("minutes") => {
                            document
                                .get_element_by_id("sched-interval")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok())
                                .map(|n| crate::scheduling::Frequency::EveryNMinutes(n).to_cron())
                        }
                        Some("hourly") => {
                            document
                                .get_element_by_id("sched-minute")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok())
                                .map(|m| crate::scheduling::Frequency::Hourly { minute: m }.to_cron())
                        }
                        Some("daily") => {
                            let hour_opt = document
                                .get_element_by_id("sched-hour")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            let minute_opt = document
                                .get_element_by_id("sched-minute")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            match (hour_opt, minute_opt) {
                                (Some(h), Some(m)) => Some(
                                    crate::scheduling::Frequency::Daily { hour: h, minute: m }.to_cron()
                                ),
                                _ => None,
                            }
                        }
                        Some("weekly") => {
                            let weekday_opt = document
                                .get_element_by_id("sched-weekday")
                                .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
                                .and_then(|s| s.value().parse::<u8>().ok());
                            let hour_opt = document
                                .get_element_by_id("sched-hour")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            let minute_opt = document
                                .get_element_by_id("sched-minute")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            match (weekday_opt, hour_opt, minute_opt) {
                                (Some(w), Some(h), Some(m)) => Some(
                                    crate::scheduling::Frequency::Weekly {
                                        weekday: w,
                                        hour: h,
                                        minute: m,
                                    }
                                    .to_cron()
                                ),
                                _ => None,
                            }
                        }
                        Some("monthly") => {
                            let day_opt = document
                                .get_element_by_id("sched-day")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            let hour_opt = document
                                .get_element_by_id("sched-hour")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            let minute_opt = document
                                .get_element_by_id("sched-minute")
                                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                                .and_then(|i| i.value().parse::<u8>().ok());
                            match (day_opt, hour_opt, minute_opt) {
                                (Some(day), Some(h), Some(m)) => Some(
                                    crate::scheduling::Frequency::Monthly {
                                        day,
                                        hour: h,
                                        minute: m,
                                    }
                                    .to_cron()
                                ),
                                _ => None,
                            }
                        }
                        Some("none") | _ => None,
                    }
                };

                let selected_model = crate::state::APP_STATE.with(|s| s.borrow().selected_model.clone());

                dispatch(crate::messages::Message::SaveAgentDetails {
                    name: name_value,
                    system_instructions,
                    task_instructions,
                    model: selected_model,
                    schedule: schedule_value_opt,
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

        // ----------------- Scheduling UI handlers --------------------
        use wasm_bindgen::closure::Closure as WasmClosure;
        use web_sys::{HtmlSelectElement};

        // Helper to toggle visibility of input fields
        let toggle_inputs = |freq: &str, doc: &Document| {
            let set_vis = |id: &str, show: bool| {
                if let Some(el) = doc.get_element_by_id(&format!("{}-container", id)) {
                    let style_val = if show { "" } else { "display:none;" };
                    let _ = el.set_attribute("style", style_val);
                }
            };

            match freq {
                "minutes" => {
                    set_vis("sched-interval", true);
                    set_vis("sched-hour", false);
                    set_vis("sched-minute", false);
                    set_vis("sched-weekday", false);
                    set_vis("sched-day", false);
                }
                "hourly" => {
                    set_vis("sched-interval", false);
                    set_vis("sched-hour", false);
                    set_vis("sched-minute", true);
                    set_vis("sched-weekday", false);
                    set_vis("sched-day", false);
                }
                "daily" => {
                    set_vis("sched-interval", false);
                    set_vis("sched-hour", true);
                    set_vis("sched-minute", true);
                    set_vis("sched-weekday", false);
                    set_vis("sched-day", false);
                }
                "weekly" => {
                    set_vis("sched-interval", false);
                    set_vis("sched-hour", true);
                    set_vis("sched-minute", true);
                    set_vis("sched-weekday", true);
                    set_vis("sched-day", false);
                }
                "monthly" => {
                    set_vis("sched-interval", false);
                    set_vis("sched-hour", true);
                    set_vis("sched-minute", true);
                    set_vis("sched-weekday", false);
                    set_vis("sched-day", true);
                }
                "none" | _ => {
                    // hide all
                    set_vis("sched-interval", false);
                    set_vis("sched-hour", false);
                    set_vis("sched-minute", false);
                    set_vis("sched-weekday", false);
                    set_vis("sched-day", false);
                }
            }
        };

        // Function to update summary line
        let update_summary = |doc: &Document| {
            // Acquire Frequency from current UI state via helper inside Save logic.
            let freq_sel_val = doc
                .get_element_by_id("sched-frequency")
                .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
                .map(|i| i.value());

            let frequency_opt = match freq_sel_val.as_deref() {
                Some("minutes") => doc
                    .get_element_by_id("sched-interval")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    .and_then(|i| i.value().parse::<u8>().ok())
                    .map(crate::scheduling::Frequency::EveryNMinutes),
                Some("hourly") => doc
                    .get_element_by_id("sched-minute")
                    .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                    .and_then(|i| i.value().parse::<u8>().ok())
                    .map(|m| crate::scheduling::Frequency::Hourly { minute: m }),
                Some("daily") => {
                    let hour_opt = doc
                        .get_element_by_id("sched-hour")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    let minute_opt = doc
                        .get_element_by_id("sched-minute")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    match (hour_opt, minute_opt) {
                        (Some(h), Some(m)) => Some(crate::scheduling::Frequency::Daily { hour: h, minute: m }),
                        _ => None,
                    }
                }
                Some("weekly") => {
                    let weekday_opt = doc
                        .get_element_by_id("sched-weekday")
                        .and_then(|e| e.dyn_into::<web_sys::HtmlSelectElement>().ok())
                        .and_then(|s| s.value().parse::<u8>().ok());
                    let hour_opt = doc
                        .get_element_by_id("sched-hour")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    let minute_opt = doc
                        .get_element_by_id("sched-minute")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    match (weekday_opt, hour_opt, minute_opt) {
                        (Some(w), Some(h), Some(m)) => Some(crate::scheduling::Frequency::Weekly {
                            weekday: w,
                            hour: h,
                            minute: m,
                        }),
                        _ => None,
                    }
                }
                Some("monthly") => {
                    let day_opt = doc
                        .get_element_by_id("sched-day")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    let hour_opt = doc
                        .get_element_by_id("sched-hour")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    let minute_opt = doc
                        .get_element_by_id("sched-minute")
                        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                        .and_then(|i| i.value().parse::<u8>().ok());
                    match (day_opt, hour_opt, minute_opt) {
                        (Some(d), Some(h), Some(m)) => Some(crate::scheduling::Frequency::Monthly {
                            day: d,
                            hour: h,
                            minute: m,
                        }),
                        _ => None,
                    }
                }
                Some("none") | _ => None,
            };

            if let Some(summary_el) = doc.get_element_by_id("sched-summary") {
                let text = frequency_opt
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "No schedule set".to_string());
                summary_el.set_inner_html(&text);
            }
        };

        // Attach change listeners to frequency select
        if let Some(freq_sel) = document.get_element_by_id("sched-frequency") {
            if let Ok(freq_select) = freq_sel.dyn_into::<HtmlSelectElement>() {
                let doc_clone = document.clone();
                let select_for_listener = freq_select.clone();
                let cb = WasmClosure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                    let val = select_for_listener.value();
                    toggle_inputs(&val, &doc_clone);
                    update_summary(&doc_clone);
                }));
                freq_select.add_event_listener_with_callback("change", cb.as_ref().unchecked_ref())?;
                cb.forget();
            }
        }

        // Attach input listeners to all schedule inputs to refresh summary
        let input_ids = [
            "sched-interval",
            "sched-hour",
            "sched-minute",
            "sched-weekday",
            "sched-day",
        ];
        for id in &input_ids {
            if let Some(inp) = document.get_element_by_id(id) {
                let doc_clone = document.clone();
                let cb = WasmClosure::<dyn FnMut(_)>::wrap(Box::new(move |_e: Event| {
                    update_summary(&doc_clone);
                }));
                inp.add_event_listener_with_callback("input", cb.as_ref().unchecked_ref())?;
                cb.forget();
            }
        }

        // --- Initial state: ensure correct inputs visibility & summary ---
        if let Some(freq_sel) = document.get_element_by_id("sched-frequency") {
            if let Some(select) = freq_sel.dyn_ref::<web_sys::HtmlSelectElement>() {
                let val = select.value();
                toggle_inputs(&val, document);
                update_summary(document);
            }
        }

        Ok(())
    }

    /// Open the modal for the given `agent_id`.
    pub fn open(document: &Document, agent_id: u32) -> Result<(), JsValue> {
        // Ensure DOM exists first
        let _ = Self::init(document);

        // The logic below is largely copied from the former
        // `ui::modals::open_agent_modal` implementation with only minimal
        // changes (namespacing, error handling).

        // --------------------------------------------------------------
        // 1. Gather data from AppState
        // --------------------------------------------------------------
        let (agent_name, system_instructions, task_instructions, schedule_value) = APP_STATE
            .with(|state| {
                let state = state.borrow();
                if let Some(agent) = state.agents.get(&agent_id) {
                    (
                        agent.name.clone(),
                        agent.system_instructions.clone().unwrap_or_default(),
                        agent.task_instructions.clone().unwrap_or_default(),
                        agent.schedule.clone(),
                    )
                } else {
                    (
                        "Unnamed Agent".to_string(),
                        String::new(),
                        String::new(),
                        None,
                    )
                }
            });

        // --------------------------------------------------------------
        // 2. Populate DOM elements
        // --------------------------------------------------------------

        // Store current agent id for later
        if let Some(modal) = document.get_element_by_id("agent-modal") {
            let _ = modal.set_attribute("data-agent-id", &agent_id.to_string());
        }

        // Title
        if let Some(modal_title) = document.get_element_by_id("modal-title") {
            modal_title.set_inner_html(&format!("Agent: {}", agent_name));
        }

        // Name input
        if let Some(name_elem) = document.get_element_by_id("agent-name") {
            if let Some(input) = name_elem.dyn_ref::<web_sys::HtmlInputElement>() {
                input.set_value(&agent_name);
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

        // NEW schedule UI – pre-fill controls based on existing cron string
        if let Some(schedule_str) = schedule_value {
            if !schedule_str.trim().is_empty() {
                if let Ok(freq) = crate::scheduling::Frequency::try_from(schedule_str.as_str()) {
                    // ------------------------------------------------------------------
                    // Apply the parsed schedule directly to the DOM.  No read-back from
                    // the <select> necessary – this avoids timing races and keeps the UI
                    // a pure function of our Rust state.
                    // ------------------------------------------------------------------

                    fn apply_frequency_ui(freq: &crate::scheduling::Frequency, document: &web_sys::Document) {
                        // Helper: set input/select value to numeric u8
                        let set_input_value = |id: &str, val: u8| {
                            if let Some(el) = document.get_element_by_id(id) {
                                if let Some(i) = el.dyn_ref::<web_sys::HtmlInputElement>() {
                                    i.set_value(&val.to_string());
                                } else if let Some(sel) = el.dyn_ref::<web_sys::HtmlSelectElement>() {
                                    sel.set_value(&val.to_string());
                                }
                            }
                        };

                        // Determine key used by dropdown & visibility matcher
                        let key = match *freq {
                            crate::scheduling::Frequency::EveryNMinutes(_) => "minutes",
                            crate::scheduling::Frequency::Hourly { .. } => "hourly",
                            crate::scheduling::Frequency::Daily { .. } => "daily",
                            crate::scheduling::Frequency::Weekly { .. } => "weekly",
                            crate::scheduling::Frequency::Monthly { .. } => "monthly",
                        };

                        // --- Dropdown ---
                        if let Some(sel_el) = document.get_element_by_id("sched-frequency") {
                            if let Some(sel) = sel_el.dyn_ref::<web_sys::HtmlSelectElement>() {
                                sel.set_value(key);
                            }
                        }

                        // --- Individual inputs ---
                        match *freq {
                            crate::scheduling::Frequency::EveryNMinutes(n) => {
                                set_input_value("sched-interval", n);
                            }
                            crate::scheduling::Frequency::Hourly { minute } => {
                                set_input_value("sched-minute", minute);
                            }
                            crate::scheduling::Frequency::Daily { hour, minute } => {
                                set_input_value("sched-hour", hour);
                                set_input_value("sched-minute", minute);
                            }
                            crate::scheduling::Frequency::Weekly { weekday, hour, minute } => {
                                set_input_value("sched-weekday", weekday);
                                set_input_value("sched-hour", hour);
                                set_input_value("sched-minute", minute);
                            }
                            crate::scheduling::Frequency::Monthly { day, hour, minute } => {
                                set_input_value("sched-day", day);
                                set_input_value("sched-hour", hour);
                                set_input_value("sched-minute", minute);
                            }
                        }

                        // --- Visibility toggles ---
                        let set_vis = |id: &str, show: bool| {
                            if let Some(el) = document.get_element_by_id(&format!("{}-container", id)) {
                                let _ = el.set_attribute("style", if show { "" } else { "display:none;" });
                            }
                        };

                        match key {
                            "minutes" => {
                                set_vis("sched-interval", true);
                                set_vis("sched-hour", false);
                                set_vis("sched-minute", false);
                                set_vis("sched-weekday", false);
                                set_vis("sched-day", false);
                            }
                            "hourly" => {
                                set_vis("sched-interval", false);
                                set_vis("sched-hour", false);
                                set_vis("sched-minute", true);
                                set_vis("sched-weekday", false);
                                set_vis("sched-day", false);
                            }
                            "daily" => {
                                set_vis("sched-interval", false);
                                set_vis("sched-hour", true);
                                set_vis("sched-minute", true);
                                set_vis("sched-weekday", false);
                                set_vis("sched-day", false);
                            }
                            "weekly" => {
                                set_vis("sched-interval", false);
                                set_vis("sched-hour", true);
                                set_vis("sched-minute", true);
                                set_vis("sched-weekday", true);
                                set_vis("sched-day", false);
                            }
                            "monthly" => {
                                set_vis("sched-interval", false);
                                set_vis("sched-hour", true);
                                set_vis("sched-minute", true);
                                set_vis("sched-weekday", false);
                                set_vis("sched-day", true);
                            }
                            _ => {
                                set_vis("sched-interval", false);
                                set_vis("sched-hour", false);
                                set_vis("sched-minute", false);
                                set_vis("sched-weekday", false);
                                set_vis("sched-day", false);
                            }
                        }

                        // --- Summary line ---
                        if let Some(summary_el) = document.get_element_by_id("sched-summary") {
                            summary_el.set_inner_html(&freq.to_string());
                        }
                    }

                    apply_frequency_ui(&freq, &document);
                }
            } else {
                // Empty string schedule - reset to "none"
                if let Some(sel_el) = document.get_element_by_id("sched-frequency") {
                    if let Some(sel) = sel_el.dyn_ref::<web_sys::HtmlSelectElement>() {
                        sel.set_value("none");
                    }
                }
                // Hide all input containers
                let set_vis = |id: &str, show: bool| {
                    if let Some(el) = document.get_element_by_id(&format!("{}-container", id)) {
                        let style_val = if show { "" } else { "display:none;" };
                        let _ = el.set_attribute("style", style_val);
                    }
                };
                set_vis("sched-interval", false);
                set_vis("sched-hour", false);
                set_vis("sched-minute", false);
                set_vis("sched-weekday", false);
                set_vis("sched-day", false);
                // Reset summary
                if let Some(summary_el) = document.get_element_by_id("sched-summary") {
                    summary_el.set_inner_html("No schedule set");
                }
            }
        } else {
            // No schedule at all (None) - reset to "none"
            if let Some(sel_el) = document.get_element_by_id("sched-frequency") {
                if let Some(sel) = sel_el.dyn_ref::<web_sys::HtmlSelectElement>() {
                    sel.set_value("none");
                }
            }
            // Hide all input containers
            let set_vis = |id: &str, show: bool| {
                if let Some(el) = document.get_element_by_id(&format!("{}-container", id)) {
                    let style_val = if show { "" } else { "display:none;" };
                    let _ = el.set_attribute("style", style_val);
                }
            };
            set_vis("sched-interval", false);
            set_vis("sched-hour", false);
            set_vis("sched-minute", false);
            set_vis("sched-weekday", false);
            set_vis("sched-day", false);
            // Reset summary
            if let Some(summary_el) = document.get_element_by_id("sched-summary") {
                summary_el.set_inner_html("No schedule set");
            }
        }

        // History tab – temporarily disabled until a dedicated agent history
        // API is wired up on the backend.  Leaving the DOM empty for now.
        if let Some(container) = document.get_element_by_id("history-container") {
            container.set_inner_html("<p>History view coming soon …</p>");
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
