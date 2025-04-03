pub mod canvas_editor;
pub mod model_selector;
pub mod dashboard;
pub mod chat_view;

// Re-export commonly used items
pub use dashboard::{init_dashboard_ws, cleanup_dashboard_ws};
