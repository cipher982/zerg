//! Simple tab-bar widget that lists workflows and lets the user switch or
//! create new ones.  First iteration – no rename or delete yet.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document};

use crate::state::{APP_STATE, dispatch_global_message};
use crate::messages::Message;

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

    let list_el = document.create_element("ul")?;
    list_el.set_attribute("class", "workflow-tab-list")?;

    // Toolbar actions container (right side)
    let actions_el = document.create_element("div")?;
    actions_el.set_attribute("class", "toolbar-actions")?;

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
    dropdown_container.append_child(&dropdown_toggle)?;
    dropdown_container.append_child(&dropdown_menu)?;
    actions_el.append_child(&dropdown_container)?;

    // Dropdown toggle click handler
    {
        let dropdown_menu_clone = dropdown_menu.clone();
        let dropdown_toggle_clone = dropdown_toggle.clone();
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: web_sys::MouseEvent| {
            e.stop_propagation();
            let is_open = dropdown_menu_clone.class_list().contains("show");
            if is_open {
                let _ = dropdown_menu_clone.class_list().remove_1("show");
                let _ = dropdown_toggle_clone.class_list().remove_1("active");
            } else {
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
        let cb = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: web_sys::MouseEvent| {
            if let Some(target) = e.target() {
                if let Ok(element) = target.dyn_into::<web_sys::Element>() {
                    if !dropdown_container_clone.contains(Some(&element)) {
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
        // Insert at the top of main content area, before input panel
        main_content.insert_before(&bar_el, main_content.first_child().as_ref())?;
    } else if let Some(app_container) = document.get_element_by_id("app-container") {
        // Fallback: insert at top of app container (current canvas layout)
        app_container.insert_before(&bar_el, app_container.first_child().as_ref())?;
    } else {
        // Last resort fallback
        document.body().unwrap().append_child(&bar_el)?;
    }

    refresh(document)?;

    Ok(())
}

/// Rebuild the tab list from current AppState
pub fn refresh(document: &Document) -> Result<(), JsValue> {
    let list_el = document
        .get_element_by_id("workflow-bar")
        .and_then(|b| b.first_child())
        .unwrap()
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

    Ok(())
}
