# WebSocket Migration Plan: Single Multiplexed Connection

## Current State Assessment
- **Architecture**: Multiple WebSocket endpoints (/api/threads/{thread_id}/ws)
- **Issue**: Duplicate connections causing duplicate messages
- **Frontend**: Creates new WebSocket connections for each thread
- **Backend**: Separate FastAPI WebSocket route per thread

## Target Architecture
- Single persistent WebSocket connection established at app load
- Message-type based routing with client subscriptions
- Unified protocol with thread IDs in messages
- Clean separation between message handlers

## Implementation: Backend Components

### 1. Connection Manager
```python
# backend/zerg/app/websocket/manager.py
class ConnectionManager:
    """Manages WebSocket connections and message routing."""

    def __init__(self):
        """Initialize an empty connection manager."""
        self.active_connections: Dict[str, WebSocket] = {}
        self.thread_subscriptions: Dict[int, Set[str]] = {}

    async def connect(self, client_id: str, websocket: WebSocket) -> None:
        await websocket.accept()
        self.active_connections[client_id] = websocket
        
    async def disconnect(self, client_id: str) -> None:
        # Remove client from connections and subscriptions
        
    async def subscribe_to_thread(self, client_id: str, thread_id: int) -> None:
        # Subscribe a client to a specific thread
        
    async def broadcast_to_thread(self, thread_id: int, message: Dict[str, Any]) -> None:
        # Send a message to all clients subscribed to a thread
        
    async def broadcast_global(self, message: Dict[str, Any]) -> None:
        # Send a message to all connected clients
```

### 2. Message Schemas
```python
# backend/zerg/app/schemas/ws_messages.py
class MessageType(str, Enum):
    """Standardized message types for the WebSocket system."""
    
    # Connection messages
    CONNECT = "connect"
    DISCONNECT = "disconnect"
    ERROR = "error"
    PING = "ping"
    PONG = "pong"
    
    # Thread messages
    SUBSCRIBE_THREAD = "subscribe_thread"
    THREAD_HISTORY = "thread_history"
    THREAD_MESSAGE = "thread_message"
    SEND_MESSAGE = "send_message"
    
    # Streaming messages
    STREAM_START = "stream_start"
    STREAM_CHUNK = "stream_chunk"
    STREAM_END = "stream_end"
    
    # System events
    SYSTEM_STATUS = "system_status"

class BaseMessage(BaseModel):
    """Base message with common fields for all WebSocket messages."""
    type: MessageType
    message_id: Optional[str] = None

# Various message type classes (ErrorMessage, PingMessage, etc.)
```

### 3. Message Handlers
```python
# backend/zerg/app/websocket/handlers.py
async def handle_ping(client_id: str, message: Dict[str, Any]) -> None:
    """Handle ping messages to keep connections alive."""
    # Send pong response to client

async def handle_subscribe_thread(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle thread subscription requests."""
    # Validate thread exists
    # Subscribe client to thread
    # Send thread history to client

async def handle_send_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Handle requests to send messages to a thread."""
    # Validate thread exists
    # Create the message in the database
    # Broadcast the new message to all subscribed clients

# Message handler dispatcher
MESSAGE_HANDLERS = {
    MessageType.PING: handle_ping,
    MessageType.SUBSCRIBE_THREAD: handle_subscribe_thread,
    MessageType.SEND_MESSAGE: handle_send_message,
}

async def dispatch_message(client_id: str, message: Dict[str, Any], db: Session) -> None:
    """Dispatch a message to the appropriate handler."""
    # Route messages to the correct handler based on type
```

### 4. Unified WebSocket Endpoint
```python
# backend/zerg/app/routers/websocket.py
@router.websocket("/ws")
async def websocket_endpoint(websocket: WebSocket, db: Session = Depends(get_db)):
    """Main WebSocket endpoint for all real-time communication."""
    client_id = str(uuid.uuid4())
    
    try:
        # Accept connection and register client
        await manager.connect(client_id, websocket)
        
        # Main message loop
        while True:
            # Receive and parse JSON data
            # Dispatch message to appropriate handler
                
    except WebSocketDisconnect:
        # Handle client disconnect
        await manager.disconnect(client_id)
```

## Migration Plan

### Phase 1: Backend Implementation (1-2 weeks)

1. **Create Core Components**
   - Connection Manager to handle client WebSocket connections
   - Message schemas with Pydantic for strict typing 
   - Message handler dispatcher system
   - Unified WebSocket endpoint at `/api/ws`

2. **Implement Feature Parity**
   - Thread message subscription and broadcasting
   - Message sending functionality
   - Streaming response support
   - Error handling and reconnection

3. **Testing & Documentation**
   - Unit tests for message handlers
   - Integration tests for the WebSocket system
   - API documentation for frontend developers

### Phase 2: Frontend Implementation (1-2 weeks)

1. **Create WebSocket Client**
   ```rust
   // frontend/src/network/websocket_client.rs
   pub struct WebSocketClient {
       connection: Option<WebSocket>,
       reconnect_timer: Option<u32>,
       message_handlers: HashMap<String, Box<dyn Fn(JsValue)>>,
   }
   
   impl WebSocketClient {
       pub fn new() -> Self {
           Self {
               connection: None,
               reconnect_timer: None,
               message_handlers: HashMap::new(),
           }
       }
       
       pub fn connect(&mut self) -> Result<(), JsValue> {
           // Connect to the WebSocket endpoint
           // Set up event handlers
       }
       
       pub fn subscribe_to_thread(&self, thread_id: u32) -> Result<(), JsValue> {
           // Send subscription message
       }
       
       pub fn send_message(&self, thread_id: u32, content: &str) -> Result<(), JsValue> {
           // Send message to thread
       }
       
       pub fn register_handler<F>(&mut self, message_type: &str, handler: F)
       where
           F: Fn(JsValue) + 'static,
       {
           // Register message handler
       }
   }
   ```

2. **Initialize on App Start**
   ```rust
   // Initialize WebSocket connection when app loads
   let mut ws_client = WebSocketClient::new();
   ws_client.connect()?;
   
   // Register message handlers
   ws_client.register_handler("thread_message", handle_thread_message);
   ws_client.register_handler("thread_history", handle_thread_history);
   ```

3. **Update Thread Selection Logic**
   ```rust
   // When user selects a thread
   fn select_thread(thread_id: u32) {
       // Update UI state
       state.current_thread_id = Some(thread_id);
       
       // Subscribe to thread over WebSocket
       if let Err(e) = ws_client.subscribe_to_thread(thread_id) {
           console::log_1(&format!("Error subscribing to thread: {:?}", e).into());
       }
   }
   ```

### Phase 3: Deployment and Cleanup (1 week)

1. **Clean Switch Approach**
   - We will NOT maintain dual systems or compatibility layers
   - The new system will completely replace the old one
   - Old code may be commented out for reference purposes only

2. **Testing and Validation**
   - Thorough testing in development/staging environments
   - Full system testing before deployment
   - No A/B testing or gradual rollout needed

3. **Cleanup**
   - Remove or comment out old WebSocket endpoint code
   - Update documentation to reflect the new WebSocket system
   - Remove any references to the old WebSocket approach

## Code Organization

1. **Backend Structure**
   ```
   backend/zerg/app/
     ├── websocket/
     │    ├── __init__.py
     │    ├── manager.py       # Connection management
     │    ├── handlers.py      # Message handlers
     │    └── utils.py         # Helper functions
     ├── schemas/
     │    └── ws_messages.py   # Message schemas
     └── routers/
          └── websocket.py     # WebSocket endpoint
   ```

2. **Frontend Structure**
   ```
   frontend/src/
     └── network/
          └── websocket_client.rs  # New WebSocket client in Rust
   ```

## Testing Strategy

1. **Unit Tests**
   - Test each message handler in isolation
   - Test connection manager with mocked WebSockets

2. **Integration Tests**
   - Test WebSocket endpoint with real connections
   - Test message routing and subscription behavior

3. **End-to-End Tests**
   - Simulate real user flows
   - Test reconnection and error handling

## Implementation Notes

1. **Message Protocol**
   - JSON-based messages with type field for routing
   - Optional message_id for correlation
   - Thread ID included in thread-related messages

2. **Error Handling**
   - Standardized error responses with error code and message
   - Graceful disconnection and reconnection

3. **Performance Considerations**
   - Connection pooling for database access
   - Efficient subscription management

## Development Approach

1. **Clean Implementation**
   - Implement new code without worrying about backward compatibility
   - Focus on simple, clean code that does exactly what we need
   - No feature flags or runtime toggling between systems

2. **Reference Preservation**
   - Comment out old code rather than immediately deleting
   - This preserves reference for understanding the previous approach
   - Commented code will eventually be removed in future cleanup

3. **Documentation**
   - Clear documentation of the new WebSocket protocol
   - Examples for frontend usage of the new WebSocket system

## Conclusion

This migration transforms the application from multiple point-to-point WebSockets to a single robust connection with multiplexed channels. The architecture follows FastAPI best practices by utilizing its built-in WebSocket support while maintaining a clean separation of concerns.

The implementation is concise and maintainable, with clear message routing and typed messages. By avoiding compatibility layers or dual systems, we keep the codebase simple and focused on the new approach. The frontend will be implemented in Rust, maintaining consistency with the existing frontend codebase.
