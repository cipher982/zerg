pub mod chat;
pub mod chat_view;
pub mod dashboard;
pub mod model_selector;
pub mod canvas_editor;

// Re-export commonly used items
pub use dashboard::{init_dashboard_ws, cleanup_dashboard_ws};
