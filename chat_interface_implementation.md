# Chat Interface Implementation Plan

## Context

We've recently refactored the backend to implement a thread-based model for agent interactions. This update moves from ephemeral API calls to a stateful system where:

1. **Agents** represent AI assistants with specific capabilities
2. **Threads** represent ongoing conversations with an agent
3. **Messages** are stored within threads with proper history

While the backend now supports this model, the frontend doesn't yet have an appropriate UI for interacting with threads. Our current interfaces include:

- **Dashboard**: For listing and managing agents
- **Canvas**: For node-based workflows

Neither of these effectively presents the thread-based conversation model. We need a dedicated chat interface that showcases the powerful capabilities of our thread system.

## Design Approach

We'll implement a new "Chat View" as a third major interface in our application. The flow will be:

1. User views agents in the dashboard
2. User clicks "Chat" on an agent
3. This opens the Chat View with that agent
4. User can see existing threads or create a new one
5. User interacts with the agent in a clean chat interface

### Key UI Components

#### 1. Thread Sidebar
- List of existing threads for the selected agent
- "New Thread" button 
- Thread items showing:
  - Thread title
  - Creation date or last activity
  - Preview of last message

#### 2. Conversation Area
- Messages displayed in typical chat style
- User messages on right, agent responses on left
- Timestamps and status indicators
- Support for streaming responses with typing indicators

#### 3. Input Area
- Text input for new messages
- Send button
- Optional: attachment support, formatting controls

#### 4. Header Area
- Agent name and avatar
- Current thread title (editable)
- Back navigation to dashboard

## Implementation Plan

### 1. Frontend State Changes

We need to extend the `AppState` struct in `frontend/src/state.rs` with the following:

```rust
// Thread-related state
pub current_thread_id: Option<u32>,
pub threads: HashMap<u32, ApiThread>,
pub thread_messages: HashMap<u32, Vec<ApiThreadMessage>>,
pub active_view: ActiveView,  // Extended to include ChatView
pub is_chat_loading: bool,
```

And we'll need to update the `ActiveView` enum in `frontend/src/storage.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ActiveView {
    Dashboard,
    Canvas,
    ChatView,
}
```

### 2. New Models

We'll add these models to `frontend/src/models.rs`:

```rust
/// Thread model from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThread {
    pub id: Option<u32>,
    pub agent_id: u32,
    pub title: String,
    pub active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadCreate {
    pub agent_id: u32,
    pub title: String,
    pub active: bool,
}

/// Update a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadUpdate {
    pub title: Option<String>,
    pub active: Option<bool>,
}

/// Thread message from the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadMessage {
    pub id: Option<u32>,
    pub thread_id: u32,
    pub role: String,
    pub content: String,
    pub created_at: Option<String>,
}

/// Create a new thread message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiThreadMessageCreate {
    pub role: String,
    pub content: String,
}
```

### 3. New API Methods

We'll need to add these methods to our API client in `frontend/src/network/api_client.rs`:

```rust
// Thread management
pub async fn get_threads(agent_id: Option<u32>) -> Result<String, JsValue> {
    let url = if let Some(id) = agent_id {
        format!("{}/api/threads?agent_id={}", Self::api_base_url(), id)
    } else {
        format!("{}/api/threads", Self::api_base_url())
    };
    Self::fetch_json(&url, "GET", None).await
}

pub async fn get_thread(thread_id: u32) -> Result<String, JsValue> {
    let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
    Self::fetch_json(&url, "GET", None).await
}

pub async fn create_thread(agent_id: u32, title: &str) -> Result<String, JsValue> {
    let url = format!("{}/api/threads", Self::api_base_url());
    let thread_data = format!("{{\"agent_id\": {}, \"title\": \"{}\", \"active\": true}}", agent_id, title);
    Self::fetch_json(&url, "POST", Some(&thread_data)).await
}

pub async fn update_thread(thread_id: u32, title: &str) -> Result<String, JsValue> {
    let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
    let thread_data = format!("{{\"title\": \"{}\"}}", title);
    Self::fetch_json(&url, "PUT", Some(&thread_data)).await
}

pub async fn delete_thread(thread_id: u32) -> Result<(), JsValue> {
    let url = format!("{}/api/threads/{}", Self::api_base_url(), thread_id);
    let _ = Self::fetch_json(&url, "DELETE", None).await?;
    Ok(())
}

// Thread messages
pub async fn get_thread_messages(thread_id: u32, skip: u32, limit: u32) -> Result<String, JsValue> {
    let url = format!("{}/api/threads/{}/messages?skip={}&limit={}", 
                   Self::api_base_url(), thread_id, skip, limit);
    Self::fetch_json(&url, "GET", None).await
}

pub async fn create_thread_message(thread_id: u32, content: &str) -> Result<String, JsValue> {
    let url = format!("{}/api/threads/{}/messages", Self::api_base_url(), thread_id);
    let message_data = format!("{{\"role\": \"user\", \"content\": \"{}\"}}", content);
    Self::fetch_json(&url, "POST", Some(&message_data)).await
}
```

### 4. WebSocket Updates

We'll need to update the WebSocket client in `frontend/src/network/ws_client.rs` to support thread-based WebSockets:

```rust
pub fn setup_thread_websocket(thread_id: u32) -> Result<(), JsValue> {
    let ws_url = format!("ws://localhost:8001/api/threads/{}/ws", thread_id);
    
    // Create a new WebSocket connection
    let ws = WebSocket::new(&ws_url)?;
    
    // Set up handlers (similar to existing setup_websocket function)
    // ...
    
    // Store WebSocket instance and thread ID
    APP_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.websocket = Some(ws);
        state.current_thread_id = Some(thread_id);
    });
    
    Ok(())
}
```

### 5. New Message Types

We'll need to extend the Message enum in `frontend/src/messages.rs` to support thread-related actions:

```rust
pub enum Message {
    // Existing messages...
    
    // New thread-related messages
    LoadThreads(u32),                // Load threads for an agent
    ThreadsLoaded(String),           // Threads loaded from API
    CreateThread(u32, String),       // Create a new thread
    ThreadCreated(String),           // Thread created response
    SelectThread(u32),               // Select a thread
    LoadThreadMessages(u32),         // Load messages for a thread
    ThreadMessagesLoaded(String),    // Thread messages loaded
    SendThreadMessage(u32, String),  // Send a message to a thread
    ThreadMessageSent(String),       // Message sent response
    UpdateThreadTitle(u32, String),  // Update thread title
    DeleteThread(u32),               // Delete thread
    
    // Navigation messages
    NavigateToChatView(u32),         // Navigate to chat view with agent
    NavigateToThreadView(u32),       // Navigate to specific thread
    NavigateToDashboard,             // Back to dashboard
}
```

### 6. UI Components

We'll create these new components in the frontend:

#### `frontend/src/views/chat_view.rs`
```rust
/// Main chat view component
pub struct ChatView {
    agent_id: u32,
    thread_id: Option<u32>,
}

impl ChatView {
    pub fn new(agent_id: u32, thread_id: Option<u32>) -> Self {
        Self {
            agent_id,
            thread_id,
        }
    }
    
    pub fn render(&self) -> String {
        // Render chat layout with sidebar and conversation area
        html! {
            <div class="chat-view">
                <div class="chat-header">
                    { self.render_header() }
                </div>
                <div class="chat-body">
                    <div class="thread-sidebar">
                        { self.render_thread_sidebar() }
                    </div>
                    <div class="conversation-area">
                        { self.render_conversation() }
                    </div>
                </div>
                <div class="chat-input-area">
                    { self.render_input() }
                </div>
            </div>
        }
    }
    
    // Helper methods to render parts of the UI
    // ...
}
```

## Technical Considerations

### WebSocket Management
- Need to switch WebSocket connections when changing threads
- Handle connection status and reconnection
- Support streaming responses during agent replies

### State Management
- Need to track current thread ID and agent ID
- Cache thread messages to reduce API calls
- Handle UI state (loading, error states, etc.)

### Thread List Updates
- Real-time updates when new threads are created
- Update thread previews when new messages arrive
- Handle sorting by most recent activity

## Phase 1: State and API Implementation

1. **Update Models**
   - Add ApiThread, ApiThreadMessage models
   - Update ApiClient with thread methods
   - Extend state with thread-related fields

2. **Update Message Handling**
   - Add thread-related messages
   - Update message dispatch in state.rs

3. **WebSocket Updates**
   - Add thread WebSocket connection support
   - Handle streaming messages from thread WebSockets

## Phase 2: UI Component Implementation

1. **Create Chat View**
   - Implement basic chat view layout
   - Add thread sidebar component
   - Create conversation display component
   - Implement message input component

2. **Update Navigation**
   - Add "Chat" button to agent rows in dashboard
   - Implement view switching logic
   - Handle back navigation

## Phase 3: Integration and Styling

1. **Connect Components to API**
   - Wire up thread listing to API calls
   - Implement message sending and receiving
   - Handle real-time updates

2. **Styling**
   - Apply consistent styling with rest of application
   - Implement responsive design
   - Add loading states and animations

## Next Steps

1. Start implementing the models and state changes
2. Add API client methods for thread operations
3. Update message handling for thread actions
4. Begin building the UI components

Let's begin by:
1. Adding the thread and message models to models.rs
2. Extending the AppState struct in state.rs
3. Adding the thread API methods to api_client.rs 