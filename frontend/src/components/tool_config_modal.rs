use wasm_bindgen::prelude::*;
use web_sys::Document;
use wasm_bindgen::JsCast;

pub struct ToolConfigModal;

impl ToolConfigModal {
    pub fn open(
        document: &Document,
        node_text: &str,
        tool_description: &str,
    ) -> Result<(), JsValue> {
        let (modal, modal_content) = crate::components::modal::ensure_modal(document, "tool-config-modal")?;

        let modal_header = document.create_element("div")?;
        modal_header.set_class_name("modal-header");
        let modal_title = document.create_element("h2")?;
        modal_title.set_inner_html(&format!("Configure: {}", node_text));
        let close_button = document.create_element("span")?;
        close_button.set_class_name("close");
        close_button.set_inner_html("&times;");
        let close_button_closure = Closure::wrap(Box::new(move || {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(modal) = document.get_element_by_id("tool-config-modal") {
                        let _ = crate::components::modal::hide(&modal);
                    }
                }
            }
        }) as Box<dyn FnMut()>);
        close_button.add_event_listener_with_callback("click", close_button_closure.as_ref().unchecked_ref())?;
        close_button_closure.forget();

        modal_header.append_child(&modal_title)?;
        modal_header.append_child(&close_button)?;

        let content_body = document.create_element("div")?;
        content_body.set_class_name("modal-body");
        content_body.set_inner_html(&format!(
            r#"
            <p class="text-sm text-gray-400 mb-4">{}</p>
            <div class="space-y-4">
                <div class="bg-gray-700 p-3 rounded">
                    <p class="text-center text-gray-500">Input configuration UI coming soon.</p>
                </div>
            </div>
            "#,
            tool_description
        ));

        let modal_footer = document.create_element("div")?;
        modal_footer.set_class_name("modal-buttons");
        modal_footer.set_inner_html(r#"
            <button class="btn">Cancel</button>
            <button class="btn-primary">Save</button>
        "#);

        modal_content.set_inner_html(""); // Clear previous content
        modal_content.append_child(&modal_header)?;
        modal_content.append_child(&content_body)?;
        modal_content.append_child(&modal_footer)?;

        crate::components::modal::show(&modal);

        Ok(())
    }
}
