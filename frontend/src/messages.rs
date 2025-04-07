// frontend/src/messages.rs
//
// The events that can occur in your UI. Expand as needed.
//
use crate::storage::ActiveView;
use crate::models::{NodeType, ApiThread, ApiThreadMessage, ApiAgent};
use crate::network::messages::AgentEventData;
use std::collections::HashMap;

#[derive(Debug, Clone)]
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
    
    // Canvas generation from agents
    GenerateCanvasFromAgents,           // Create nodes for agents that don't have one
    
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
    
    // Node manipulation with agent references
    AddAgentNode {
        agent_id: Option<u32>,
        x: f64,
        y: f64,
        node_type: NodeType,
        text: String,
    },
    DeleteNode {
        node_id: String,
    },
    
    // Explicit sync between agents and nodes
    SyncNodeToAgent {
        node_id: String,
        agent_id: u32,
    },
    SyncAgentToNode {
        agent_id: u32,
        node_id: String,
    },
    
    // Workflow management
    CreateWorkflow {
        name: String,
    },
    SelectWorkflow {
        workflow_id: u32,
    },
    AddEdge {
        from_node_id: String,
        to_node_id: String,
        label: Option<String>,
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
        model: String,
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
    
    // WebSocket related messages
    UpdateNodeText {
        node_id: String,
        text: String,
        is_first_chunk: bool,
    },
    
    CompleteNodeResponse {
        node_id: String,
        final_text: String,
    },
    
    UpdateNodeStatus {
        node_id: String,
        status: String,
    },
    
    // Database management
    ResetDatabase,                      // Clear all agent data from database
    
    // Reload agents from the API to refresh state
    RefreshAgentsFromAPI,
    
    // Animation related
    AnimationTick,
    
    // Thread-related messages
    LoadThreads(u32),                // Load threads for an agent
    ThreadsLoaded(Vec<ApiThread>), // Changed String to Vec<ApiThread> for direct use
    CreateThread(u32, String),       // Create a new thread for an agent
    ThreadCreated(ApiThread),        // Changed String to ApiThread for direct use
    SelectThread(u32),               // Select a thread
    LoadThreadMessages(u32),         // Load messages for a thread
    ThreadMessagesLoaded(u32, Vec<ApiThreadMessage>), // Changed String to Vec<ApiThreadMessage> + thread_id
    SendThreadMessage(u32, String),  // Send a message to a thread
    ThreadMessageSent(String, String),  // Message sent response with client_id
    ThreadMessageFailed(u32, String), // Message failed to send with client_id
    UpdateThreadTitle(u32, String),  // Update thread title (request)
    DeleteThread(u32),               // Delete thread
    
    // --- NEW WebSocket Received Messages ---
    ReceiveNewMessage(ApiThreadMessage), // New message received via WebSocket
    ReceiveThreadUpdate {              // Thread metadata updated via WebSocket
        thread_id: u32,
        title: Option<String>,
    },
    ReceiveStreamStart(u32),          // Start of streaming response for thread_id
    ReceiveStreamChunk {               // Chunk of streaming response
        thread_id: u32,
        content: String,
    },
    ReceiveStreamEnd(u32),            // End of streaming response for thread_id
    // Using UpdateConversation for thread history
    // --- END NEW WebSocket Received Messages ---

    // Navigation messages
    NavigateToChatView(u32),         // Navigate to chat view with agent
    NavigateToThreadView(u32),       // Navigate to specific thread
    NavigateToDashboard,             // Back to dashboard
    
    // Chat view messages
    LoadAgentInfo(u32),                    // Request to load agent info
    AgentInfoLoaded(Box<ApiAgent>),        // Changed String to Box<ApiAgent>
    RequestNewThread,                      // Request to create new thread
    RequestSendMessage(String),            // Request to send message
    RequestUpdateThreadTitle(String),      // Request to update thread title
    RequestThreadTitleUpdate,              // Request to refresh thread title UI
    UpdateThreadList(Vec<ApiThread>, Option<u32>, HashMap<u32, Vec<ApiThreadMessage>>),  // Update thread list UI
    UpdateConversation(Vec<ApiThreadMessage>),  // Update conversation UI
    UpdateThreadTitleUI(String),           // Update thread title UI with provided title
    UpdateLoadingState(bool),              // Update loading state in UI

    ReceiveAgentUpdate(AgentEventData),
    ReceiveAgentDelete(i32),
    ReceiveThreadHistory(Vec<ApiThreadMessage>),
}

/// Commands represent side effects that should be executed after state updates.
/// This separates pure state changes from effects like UI updates, API calls, etc.
#[derive(Debug, Clone)]
pub enum Command {
    /// Chain another message to be processed
    SendMessage(Message),
    /// Represents no side effect
    NoOp,
}

impl Command {
    /// Helper to create a SendMessage command
    pub fn send(msg: Message) -> Self {
        Command::SendMessage(msg)
    }

    /// Helper to create a NoOp command
    pub fn none() -> Self {
        Command::NoOp
    }
} 