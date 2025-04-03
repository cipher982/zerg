# WebSocket Refactoring Plan: Single Connection + Event-Driven Architecture

## Status: Backend Complete ✓, Frontend Infrastructure Complete ✓, View Integration Starting 🚀
Last Updated: April 5, 2024

## 1. Original Problem & Goals [✓]

**Observation:** Logs show frequent socket openings/closings, particularly when moving from a "Dashboard" view to a "Chat" view. This repeated setup and teardown leads to:  
- Socket churn (overhead in opening new connections)  
- Potential state-sync issues (missed or out-of-order messages)  
- Higher complexity (frontend code with multiple WebSocket endpoints and event handlers)  

**Goal:** Replace the multiple-socket approach with a single, persistent WebSocket for the application lifecycle, plus a streamlined method to manage real-time events.

## 2. Implementation Progress

### Backend Completed ✓
1. Created event bus infrastructure:
   - `EventType` enum for standardized event types
   - `EventBus` class with publish/subscribe functionality
   - Global event bus instance
   - Full test coverage for event bus functionality

2. Implemented event publishing for agent operations:
   - Created `@publish_event` decorator for clean event emission
   - Added events for agent CRUD operations (created, updated, deleted)
   - Added comprehensive tests for agent events
   - Kept existing functionality intact while adding events

3. Created new connection manager:
   - Implemented `TopicConnectionManager` in `new_manager.py`
   - Added topic-based subscription system
   - Integrated with event bus for agent and thread events
   - Added comprehensive unit tests in `test_topic_manager.py`

4. Created new message handlers:
   - Implemented handlers in `new_handlers.py` for subscribe/unsubscribe
   - Added support for initial topic subscriptions
   - Integrated with existing message types (ping/pong)
   - Added error handling for invalid topics/resources

5. Added new WebSocket endpoint:
   - Created `/api/ws/v2` endpoint in `new_websocket.py`
   - Supports topic-based subscriptions
   - Maintains backward compatibility (old endpoint still works)
   - Added integration tests in `test_new_websocket.py`

### Frontend Implementation Progress ✓

#### Completed Core Infrastructure ✓
1. **Event Types Module** (`event_types.rs`):
   - Mirrors backend's event and message types
   - Provides type-safe enums for all message types
   - Includes topic formatting helpers
   - Full test coverage

2. **Message System** (`messages.rs`):
   - Structured message types matching backend schema
   - Serialization/deserialization support
   - Message builders for common operations
   - Message parsing and type inference
   - Comprehensive test suite
   - Added structs for backend event data payloads (e.g., `AgentEventData`, `ThreadCreatedEventData`).

3. **WebSocket Client v2** (`ws_client_v2.rs`):
   - Single connection management
   - Automatic reconnection with exponential backoff
   - Ping/pong keep-alive mechanism
   - Type-safe message handling
   - Connection state management
   - Error handling and logging
   - Refactored to use a callback pattern (`on_connect`, `on_message`, `on_disconnect`) for better integration.

4. **Topic Manager** (`topic_manager.rs`) [NEW ✓]:
   - Manages topic subscriptions (`agent:*`, `thread:*`).
   - Sends `subscribe`/`unsubscribe` messages to backend.
   - Handles automatic resubscription on reconnect (`resubscribe_all_topics`).
   - Routes incoming messages based on inferred topic to registered handlers.

5. **Global Instance Management** [NEW ✓]:
   - Integrated `WsClientV2` and `TopicManager` into global `AppState` using `Rc<RefCell<>>`.
   - Initialized instances and connected WebSocket in `lib.rs::start()`.
   - Linked `WsClientV2` callbacks to `TopicManager` methods.

#### Key Learnings & Decisions
1. **Type Safety**:
   - Using Rust's type system to prevent message format errors
   - Trait-based approach for message handling (`WsMessage` trait)
   - Macro-based implementation to reduce boilerplate

2. **State Management**:
   - Using `Rc<RefCell<>>` for shared state
   - Clear separation between connection and message handling
   - Proper cleanup of resources (ping intervals, handlers)
   - Confirmed `Rc<RefCell<>>` is effective for sharing WS client and topic manager instances.

3. **Testing Strategy**:
   - Unit tests with `wasm-bindgen-test`
   - Test helpers for message creation and validation
   - Mocking considerations for WebSocket interactions

4. **Callback Pattern** [NEW]: Adopted `on_connect`, `on_message`, `on_disconnect` callbacks in `WsClientV2` for decoupling and reacting to connection state changes.

5. **Topic Routing** [NEW]: `TopicManager` infers topic from incoming message `type`/`data`, aligning with backend broadcast logic, rather than requiring topic in message payload.

#### Current Challenges Addressed / Resolved ✓
1. **Message Handler Lifetime**: Addressed by using `Rc<RefCell<dyn FnMut(...)>>` for handlers in `TopicManager`. Components subscribing will need to manage their subscription lifecycle (e.g., unsubscribe on drop/unmount).

2. **Reconnection Edge Cases**: Addressed automatic topic resubscription via `TopicManager::resubscribe_all_topics` triggered by `WsClientV2`'s `on_connect` callback. State recovery within components will be handled during view integration.

## 3. Next Immediate Steps [Revised]

1. **View Integration** [Next Priority]:
   - Identify components needing real-time updates (Dashboard, Chat View, etc.).
   - Update components to use the global `TopicManager` instance (via `AppState`).
   - Implement `subscribe`/`unsubscribe` calls within component lifecycles.
   - Add handlers within components to process event data (using `messages.rs` structs) and dispatch `AppState` updates.

2. **Cleanup**:
   - Gradually remove old WebSocket client (`ws_client.rs`) and related code as views are migrated.

## 4. Testing Requirements

1. **Unit Tests**:
   - Message serialization/deserialization
   - Topic subscription logic
   - Reconnection behavior
   - State management

2. **Integration Tests**:
   - Connection lifecycle
   - Message flow
   - Topic subscription/unsubscription
   - View-specific behaviors
   - Add tests for view-specific event handling and state updates.

3. **End-to-End Tests**:
   - Full user workflows
   - Error scenarios
   - Performance metrics

## 5. Risk Assessment

1. **Technical Risks**:
   - Message ordering during reconnection
   - Memory leaks in long-running connections
   - Browser compatibility issues
   - Performance impact of single connection

2. **Migration Risks**:
   - Data loss during transition
   - Backward compatibility issues
   - User experience disruption

## 6. Success Metrics

1. **Connection Metrics**:
   - Number of active connections
   - Connection lifetime
   - Reconnection frequency
   - Message latency

2. **Code Quality**:
   - Test coverage
   - Error handling completeness
   - Documentation quality

## 7. Next Actions [Revised Timeline]

1. **Immediate (Next 1-2 Days)**:
   - Start view migration with the **Dashboard** component.
     - Subscribe to relevant agent topics (e.g., `agent:*` or a general "all agents" topic if backend supports).
     - Handle `agent_created`, `agent_updated`, `agent_deleted` events to refresh the dashboard list.

2. **Short Term (1 Week)**:
   - Complete Dashboard migration.
   - Begin migration of the **Chat View**.
     - Subscribe to specific `thread:*` topics when a thread is opened.
     - Handle `thread_message_created` events.
     - Handle `thread_updated`, `thread_deleted` events.
   - Refine integration tests for migrated views.

3. **Medium Term (2-3 Weeks)**:
   - Complete Chat View migration.
   - Migrate any other necessary components (e.g., Canvas if it needs real-time updates).
   - Complete removal of old WebSocket code (`ws_client.rs`).
   - Performance optimization and documentation updates.

## 8. Open Questions [Partially Addressed]

1. **Architecture**:
   - How to handle view-specific message processing? -> *Addressed: Via `TopicManager` handlers within components dispatching `AppState` messages.*
   - Best approach for global state updates? -> *Addressed: Confirmed `Rc<RefCell<AppState>>` with `dispatch_global_message` pattern.*
   - Strategy for handling offline mode? -> *Remains Open: Current focus is online mode.*

2. **Performance**:
   - Impact of single connection on memory usage?
   - Message queuing strategy during disconnection?
   - Optimal ping interval for different network conditions?

3. **Testing**:
   - How to properly test WebSocket in Rust/WASM environment?
   - Best approach for mocking WebSocket in tests?
   - Performance testing methodology?

## 9. Dependencies

1. **Required Crate Updates**:
   - serde for serialization
   - uuid for message IDs
   - wasm-bindgen-test for testing
   - web-sys for browser APIs

2. **Browser Requirements**:
   - WebSocket support
   - JSON parsing capabilities
   - Local storage for state persistence

The focus now shifts to **integrating views** with the new topic-based WebSocket system, starting with the Dashboard.
