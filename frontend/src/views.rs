// frontend/src/views.rs
//
// This file contains functions to render different parts of the UI
// based on the current application state.
//
use web_sys::Document;
use crate::storage::ActiveView;
use wasm_bindgen::JsValue;
use crate::components::agent_config_modal::AgentConfigModal;


// Handle agent modal display
pub fn hide_agent_modal(document: &Document) -> Result<(), JsValue> {
    if let Some(modal) = document.get_element_by_id("agent-modal") {
        modal.set_attribute("style", "display: none;")?;
    }
    
    Ok(())
}

// Add a function to display the agent modal
pub fn show_agent_modal(agent_id: u32, document: &Document) -> Result<(), JsValue> {
    // Directly open the modal for the given agent
    AgentConfigModal::open(document, agent_id)?;
    Ok(())
}

// Render the appropriate view based on the explicit view type
// This avoids requiring a reference to AppState, preventing potential borrow issues
pub fn render_active_view_by_type(view_type: &ActiveView, document: &Document) -> Result<(), JsValue> {
    // First log which view we're switching to for debugging
    web_sys::console::log_1(&format!("Switching to view: {:?}", view_type).into());
    
    // Unmount dashboard if it's mounted
    if crate::pages::dashboard::is_dashboard_mounted(document) {
        crate::pages::dashboard::unmount_dashboard(document)?;
    }
    
    // Unmount canvas if it's mounted
    if crate::pages::canvas::is_canvas_mounted(document) {
        crate::pages::canvas::unmount_canvas(document)?;
    }

    // Unmount profile page if mounted
    if crate::pages::profile::is_profile_mounted(document) {
        crate::pages::profile::unmount_profile(document)?;
    }
    
    // Hide chat view if it exists
    if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
        chat_container.set_attribute("style", "display: none;")?;
    }
    
    // Now mount the requested view
    match view_type {
        ActiveView::Dashboard => {
            crate::pages::dashboard::mount_dashboard(document)?;
        },
        ActiveView::Canvas => {
            crate::pages::canvas::mount_canvas(document)?;
        },
        ActiveView::Profile => {
            crate::pages::profile::mount_profile(document)?;
        },
        ActiveView::ChatView => {
            // Setup the chat view if needed
            crate::components::chat_view::setup_chat_view(document)?;
            
            // Show chat view container
            if let Some(chat_container) = document.get_element_by_id("chat-view-container") {
                chat_container.set_attribute("style", "display: flex;")?;
            }
        }
    }
    
    // Update tab styling based on active view
    update_tab_styling(view_type, document)?;
    
    Ok(())
}

// Helper function to update tab styling based on active view
fn update_tab_styling(view_type: &ActiveView, document: &Document) -> Result<(), JsValue> {
    // Reset all tabs to inactive state first
    if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
        dashboard_tab.set_class_name("tab-button");
    }
    
    if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
        canvas_tab.set_class_name("tab-button");
    }
    
    // Now set the active tab based on the current view
    match view_type {
        ActiveView::Dashboard => {
            if let Some(dashboard_tab) = document.get_element_by_id("dashboard-tab") {
                dashboard_tab.set_class_name("tab-button active");
            }
        },
        ActiveView::Canvas => {
            if let Some(canvas_tab) = document.get_element_by_id("canvas-tab") {
                canvas_tab.set_class_name("tab-button active");
            }
        },
        ActiveView::Profile => {
            // Profile currently has no tab; ensure others are inactive.
        },
        ActiveView::ChatView => {
            // Chat doesn't have a tab currently, could add one in the future
        }
    }
    
    Ok(())
}