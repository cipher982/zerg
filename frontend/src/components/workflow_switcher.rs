//! Simple tab-bar widget that lists workflows and lets the user switch or
//! create new ones.  First iteration – no rename or delete yet.

use wasm_bindgen::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document};

use crate::state::{APP_STATE, dispatch_global_message};
use crate::messages::Message;
use crate::dom_utils::mount_in_overlay;

/// Call once on canvas view mount.  Renders the bar under the header.
pub fn init(document: &Document) -> Result<(), JsValue> {
    // Check if already initialised
    if document.get_element_by_id("workflow-bar").is_some() {
        return Ok(());
    }

    // Root bar element
    let bar_el = document.create_element("div")?;
    bar_el.set_attribute("id", "workflow-bar")?;
    bar_el.set_attribute("class", "workflow-bar")?;

    // Inject run-button CSS once
    if document.get_element_by_id("run-btn-style").is_none() {
        let style_el = document.create_element("style")?;
        style_el.set_attribute("id", "run-btn-style")?;
        style_el.set_text_content(Some(r#"
.run-btn.running{pointer-events:none;animation:spin 1s linear infinite;}
.run-btn.success{color:#6bff92;}
.run-btn.failed{color:#ff4e4e;}
@keyframes spin{from{transform:rotate(0deg);}to{transform:rotate(360deg);}}
"#));
        document.body().unwrap().append_child(&style_el)?;
    }

    let list_el = document.create_element("ul")?;
    list_el.set_attribute("class", "workflow-tab-list")?;

    // Toolbar actions container (right side)
    let actions_el = document.create_element("div")?;
    actions_el.set_attribute("class", "toolbar-actions")?;

    // --------------------------------------------------------------------
    // Run (▶︎) button – trigger workflow execution
    // --------------------------------------------------------------------
    let run_btn = document.create_element("button")?;
    run_btn.set_attribute("type", "button")?;
    run_btn.set_inner_html("▶︎");
    run_btn.set_attribute("class", "toolbar-btn")?;
    run_btn.set_attribute("id", "run-workflow-btn")?;
    run_btn.set_attribute("title", "Run Workflow (⌘/Ctrl + R)")?;

    {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            // Read current workflow id at click time (avoid stale capture)
            if let Some(current_id) = APP_STATE.with(|st| st.borrow().current_workflow_id) {
                dispatch_global_message(Message::StartWorkflowExecution { workflow_id: current_id });
            } else {
                web_sys::console::warn_1(&"No workflow selected – cannot run".into());
            }
        }));
        run_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }

    let _ = actions_el.append_child(&run_btn);

    // Logs button 📜
    let logs_btn = document.create_element("button")?;
    logs_btn.set_attribute("type", "button")?;
    logs_btn.set_inner_html("📜");
    logs_btn.set_attribute("class", "toolbar-btn")?;
    logs_btn.set_attribute("title", "Toggle Logs")?;
    {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            crate::state::dispatch_global_message(crate::messages::Message::ToggleLogDrawer);
        }));
        logs_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    let _ = actions_el.append_child(&logs_btn);

    // Execution history button 🕒
    let hist_btn = document.create_element("button")?;
    hist_btn.set_attribute("type", "button")?;
    hist_btn.set_inner_html("🕒");
    hist_btn.set_attribute("class", "toolbar-btn")?;
    hist_btn.set_attribute("title", "Execution History")?;
    {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            crate::state::dispatch_global_message(crate::messages::Message::ToggleExecutionHistory);
            // If opening, also trigger load for current workflow
            crate::state::APP_STATE.with(|st| {
                if st.borrow().exec_history_open {
                    if let Some(wf_id) = st.borrow().current_workflow_id {
                        crate::state::dispatch_global_message(crate::messages::Message::LoadExecutionHistory { workflow_id: wf_id });
                    }
                }
            });
        }));
        hist_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    let _ = actions_el.append_child(&hist_btn);

    // Center view button
    let center_btn = document.create_element("button")?;
    center_btn.set_attribute("type", "button")?;
    center_btn.set_inner_html("⌖");
    center_btn.set_attribute("class", "toolbar-btn")?;
    center_btn.set_attribute("title", "Center View")?;
    {
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            dispatch_global_message(Message::CenterView);
        }));
        center_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    actions_el.append_child(&center_btn)?;

    // Dropdown menu (⋮)
    let dropdown_container = document.create_element("div")?;
    dropdown_container.set_attribute("class", "dropdown-container")?;

    let dropdown_toggle = document.create_element("button")?;
    dropdown_toggle.set_attribute("type", "button")?;
    dropdown_toggle.set_inner_html("⋮");
    dropdown_toggle.set_attribute("class", "dropdown-toggle")?;
    dropdown_toggle.set_attribute("title", "More Options")?;

    let dropdown_menu = document.create_element("div")?;
    dropdown_menu.set_attribute("class", "dropdown-menu")?;

    let clear_btn = document.create_element("button")?;
    clear_btn.set_attribute("type", "button")?;
    clear_btn.set_inner_html("Clear Canvas");
    clear_btn.set_attribute("class", "dropdown-item danger")?;
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let _ = dropdown_menu_clone.class_list().remove_1("show");
            let _ = dropdown_toggle_clone.class_list().remove_1("active");
            dispatch_global_message(Message::ClearCanvas);
        }));
        clear_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    dropdown_menu.append_child(&clear_btn)?;

    // --------------------------------------------------------------
    // Rename workflow action
    // --------------------------------------------------------------
    let rename_btn = document.create_element("button")?;
    rename_btn.set_attribute("type", "button")?;
    rename_btn.set_inner_html("Rename Workflow");
    rename_btn.set_attribute("class", "dropdown-item")?;
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            // Close dropdown
            let _ = dropdown_menu_clone.class_list().remove_1("show");
            let _ = dropdown_toggle_clone.class_list().remove_1("active");

            // Ask for new name & description
            let win = web_sys::window().unwrap();
            let new_name_opt = win.prompt_with_message("New workflow name?").unwrap_or(None);
            if let Some(new_name) = new_name_opt {
                let new_name = new_name.trim();
                if !new_name.is_empty() {
                    let desc_opt = win.prompt_with_message("Description (optional)").unwrap_or(None);
                    let description = desc_opt.unwrap_or_default();
                    // Dispatch rename command
                    crate::state::APP_STATE.with(|st| {
                        if let Some(current_id) = st.borrow().current_workflow_id {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::RenameWorkflow {
                            workflow_id: current_id,
                            name: new_name.to_string(),
                            description: description.clone(),
                        }
                    );
                        } else {
                            web_sys::console::warn_1(&"No workflow selected to rename".into());
                        }
                    });
                }
            }
        }));
        rename_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    dropdown_menu.append_child(&rename_btn)?;

    // --------------------------------------------------------------
    // Delete workflow action
    // --------------------------------------------------------------
    let delete_btn = document.create_element("button")?;
    delete_btn.set_attribute("type", "button")?;
    delete_btn.set_inner_html("Delete Workflow");
    delete_btn.set_attribute("class", "dropdown-item danger")?;
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            // Close dropdown
            let _ = dropdown_menu_clone.class_list().remove_1("show");
            let _ = dropdown_toggle_clone.class_list().remove_1("active");

            let confirm = web_sys::window().unwrap().confirm_with_message("Are you sure you want to delete this workflow? This can be restored by support within 30 days.").unwrap_or(false);
            if !confirm {
                return;
            }

            crate::state::APP_STATE.with(|st| {
                if let Some(current_id) = st.borrow().current_workflow_id {
                    crate::state::dispatch_global_message(
                        crate::messages::Message::DeleteWorkflow { workflow_id: current_id }
                    );
                } else {
                    web_sys::console::warn_1(&"No workflow selected to delete".into());
                }
            });
        }));
        delete_btn.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    dropdown_menu.append_child(&delete_btn)?;
    dropdown_container.append_child(&dropdown_toggle)?;
    dropdown_container.append_child(&dropdown_menu)?;
    actions_el.append_child(&dropdown_container)?;

    // Dropdown toggle click handler – opens the menu and positions it
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let doc_clone = document.clone();

        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();

            let is_open = dropdown_menu_clone.class_list().contains("show");

            if is_open {
                let _ = dropdown_menu_clone.class_list().remove_1("show");
                let _ = dropdown_toggle_clone.class_list().remove_1("active");
            } else {
                // Mount dropdown in #overlay-root portal
                mount_in_overlay(&dropdown_menu_clone);

                // Position the menu right-aligned to the toggle button
                let bbox = dropdown_toggle_clone.get_bounding_client_rect();

                // Scroll offsets
                let win = web_sys::window().unwrap();
                let scroll_x = win.scroll_x().unwrap_or(0.0);
                let scroll_y = win.scroll_y().unwrap_or(0.0);

                let top_px = bbox.bottom() + scroll_y;
                let left_px = bbox.right() + scroll_x - dropdown_menu_clone.client_width() as f64;

                if let Some(html_el) = dropdown_menu_clone.dyn_ref::<web_sys::HtmlElement>() {
                    let _ = html_el.style().set_property("top", &format!("{}px", top_px));
                    let _ = html_el.style().set_property("left", &format!("{}px", left_px));
                }

                let _ = dropdown_menu_clone.class_list().add_1("show");
                let _ = dropdown_toggle_clone.class_list().add_1("active");
            }
        }));
        dropdown_toggle.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }
    // Document click handler to close dropdown when clicking outside
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let dropdown_container_clone = dropdown_container.clone();
        let dropdown_menu_for_contains = dropdown_menu.clone();
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: web_sys::MouseEvent| {
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                    // Close only if the click happened outside both the toggle
                    // container *and* the menu itself (when it is appended to body)
                    if !dropdown_container_clone.contains(Some(&element))
                        && !dropdown_menu_for_contains.contains(Some(&element))
                    {
                        let _ = dropdown_menu_clone.class_list().remove_1("show");
                        let _ = dropdown_toggle_clone.class_list().remove_1("active");
                    }
                }
            }
        }));
        document.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();
    }

    bar_el.append_child(&list_el)?;
    bar_el.append_child(&actions_el);

    // Insert bar into the main content area of the canvas layout
    // Try multiple locations in order of preference
    if let Some(main_content) = document.get_element_by_id("main-content-area") {
        web_sys::console::log_1(&"WORKFLOW_SWITCHER: Inserting into main-content-area".into());
        // Insert at the top of main content area, before canvas container
        main_content.insert_before(&bar_el, main_content.first_child().as_ref())?;
    } else if let Some(app_container) = document.get_element_by_id("app-container") {
        web_sys::console::log_1(&"WORKFLOW_SWITCHER: Fallback - inserting into app-container".into());
        // Create main-content-area if it doesn't exist and we're in canvas view
        if app_container.class_list().contains("canvas-view") {
            let main_content = document.create_element("div")?;
            main_content.set_id("main-content-area");
            main_content.set_class_name("main-content-area");
            
            // Move existing children to main content area
            while let Some(child) = app_container.first_child() {
                if let Some(element) = child.dyn_ref::<web_sys::Element>() {
                    // Don't move the agent shelf
                    if element.id() == "agent-shelf" {
                        break;
                    }
                }
                main_content.append_child(&child)?;
            }
            
            app_container.append_child(&main_content)?;
            main_content.insert_before(&bar_el, main_content.first_child().as_ref())?;
        } else {
            // Not in canvas view, use old behavior
            app_container.insert_before(&bar_el, app_container.first_child().as_ref())?;
        }
    } else {
        web_sys::console::log_1(&"WORKFLOW_SWITCHER: Last resort - inserting into body".into());
        // Last resort fallback
        document.body().unwrap().append_child(&bar_el)?;
    }

    refresh(document)?;

    Ok(())
}

/// Rebuild the tab list from current AppState
pub fn refresh(document: &Document) -> Result<(), JsValue> {
    // Make sure workflow bar is initialized first
    let workflow_bar = match document.get_element_by_id("workflow-bar") {
        Some(bar) => bar,
        None => {
            // Workflow bar not initialized yet, call init first
            init(document)?;
            document
                .get_element_by_id("workflow-bar")
                .ok_or_else(|| JsValue::from_str("workflow-bar element not found after init"))?
        }
    };
    
    let list_el = workflow_bar
        .first_child()
        .ok_or_else(|| JsValue::from_str("workflow-bar has no children - initialization may have failed"))?
        .dyn_into::<web_sys::Element>()?;

    // Clear existing children
    while let Some(child) = list_el.first_child() {
        list_el.remove_child(&child)?;
    }

    // Borrow state once
    let (workflows_vec, current_id) = APP_STATE.with(|state| {
        let st = state.borrow();
        (st.workflows.values().cloned().collect::<Vec<_>>(), st.current_workflow_id)
    });

    for wf in workflows_vec {
        let li = document.create_element("li")?;
        li.set_attribute("class", if Some(wf.id) == current_id { "tab active" } else { "tab" })?;
        li.set_attribute("data-id", &wf.id.to_string())?;
        li.set_text_content(Some(&wf.name));

        // Click handler – select workflow
        let id_clone = wf.id;
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            dispatch_global_message(Message::SelectWorkflow { workflow_id: id_clone });
        }));
        li.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref())?;
        cb.forget();

        list_el.append_child(&li)?;
    }

    // Plus-tab
    let plus_li = document.create_element("li")?;
    plus_li.set_attribute("class", "tab plus-tab")?;
    plus_li.set_text_content(Some("＋"));

    let cb_new = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        // Prompt for name
        let name = web_sys::window().unwrap().prompt_with_message("Workflow name?").unwrap_or(None);
        if let Some(n) = name {
            if !n.trim().is_empty() {
                dispatch_global_message(Message::CreateWorkflow { name: n.trim().to_string() });
                // UI will refresh via below command from reducer – but we also manually refresh after short delay
            }
        }
    }));
    plus_li.add_event_listener_with_callback("click", cb_new.as_ref().unchecked_ref())?;
    cb_new.forget();

    list_el.append_child(&plus_li)?;

    // Update run button state if it exists
    update_run_button(document)?;

    Ok(())
}

/// Update the ▶︎ Run button CSS classes based on AppState.current_execution.
/// Adds one of: `running`, `success`, `failed` or removes all for idle.
pub fn update_run_button(document: &Document) -> Result<(), JsValue> {
    use crate::state::{APP_STATE, ExecPhase};

    if let Some(btn) = document.get_element_by_id("run-workflow-btn") {
        let class_list = btn.class_list();
        let _ = class_list.remove_1("running");
        let _ = class_list.remove_1("success");
        let _ = class_list.remove_1("failed");

        let phase_opt = APP_STATE.with(|st| st.borrow().current_execution.clone().map(|e| e.status));

        if let Some(phase) = phase_opt {
            match phase {
                ExecPhase::Running | ExecPhase::Starting => {
                    let _ = class_list.add_1("running");
                }
                ExecPhase::Success => {
                    let _ = class_list.add_1("success");
                }
                ExecPhase::Failed => {
                    let _ = class_list.add_1("failed");
                }
            }
        }
    }

    Ok(())
}
