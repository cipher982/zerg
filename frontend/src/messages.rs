// frontend/src/messages.rs
//
// The events that can occur in your UI. Expand as needed.
//
use crate::storage::ActiveView;
use crate::models::NodeType;

#[derive(Debug)]
#[allow(dead_code)] // Suppress warnings about unused variants
pub enum Message {
    // View switching
    ToggleView(ActiveView),              // Switch between Dashboard and Canvas
    
    // Agent management
    CreateAgent(String),                 // Create a new agent with a given name
    CreateAgentWithDetails {              // Enhanced version with complete details
        name: String,
        agent_id: u32,                   // API-provided ID
        system_instructions: String,
        task_instructions: String,
    },
    EditAgent(String),                   // Edit existing agent by ID
    DeleteAgent(String),                 // Delete an agent by ID
    
    // Node manipulation
    UpdateNodePosition {
        node_id: String,
        x: f64,
        y: f64,
    },
    AddNode {
        text: String,
        x: f64,
        y: f64,
        node_type: NodeType,
    },
    AddResponseNode {
        parent_id: String,
        response_text: String,
    },
    
    // Canvas view controls
    ToggleAutoFit,                       // Toggle auto-fit functionality
    CenterView,                          // Center the canvas view
    ClearCanvas,                         // Clear all nodes from the canvas
    
    // Canvas zooming
    ZoomCanvas {
        new_zoom: f64,
        viewport_x: f64,
        viewport_y: f64,
    },
    
    // Input handling
    UpdateInputText(String),             // Update the input text field
    
    // Dragging state
    StartDragging {
        node_id: String,
        offset_x: f64,
        offset_y: f64,
    },
    StopDragging,
    
    // Canvas dragging
    StartCanvasDrag {
        start_x: f64,
        start_y: f64,
    },
    UpdateCanvasDrag {
        current_x: f64,
        current_y: f64,
    },
    StopCanvasDrag,
    
    // Other actions can be added as needed
    
    // Modal operations
    SaveAgentDetails {
        name: String,
        system_instructions: String,
        task_instructions: String,
    },
    
    CloseAgentModal,
    
    // Task operations
    SendTaskToAgent,
    
    // Tab switching in modal
    SwitchToMainTab,
    SwitchToHistoryTab,
    
    // Auto-save operations
    UpdateSystemInstructions(String),
    UpdateAgentName(String),
} 