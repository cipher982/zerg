# WebSocket Refactoring Plan: Single Connection + Event-Driven Architecture

## Status: In Progress
Last Updated: [Current Date]

## 1. Original Problem & Goals [✓]

**Observation:** Logs show frequent socket openings/closings, particularly when moving from a "Dashboard" view to a "Chat" view. This repeated setup and teardown leads to:  
- Socket churn (overhead in opening new connections)  
- Potential state-sync issues (missed or out-of-order messages)  
- Higher complexity (frontend code with multiple WebSocket endpoints and event handlers)  

**Goal:** Replace the multiple-socket approach with a single, persistent WebSocket for the application lifecycle, plus a streamlined method to manage real-time events.

## 2. Implementation Progress

### Completed ✓
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

### In Progress 🔄
1. Creating new connection manager:
   - Design topic-based subscription system
   - Integrate with event bus
   - Plan coexistence with current manager

### To Do 📋
1. Complete new connection manager:
   - Implement topic-based subscriptions
   - Add WebSocket message handlers for new events
   - Create migration path from old to new system

2. Migrate existing functionality:
   - Move thread events to new system
   - Update frontend to handle unified events
   - Add support for topic-based subscriptions in frontend

3. Clean up and testing:
   - Remove old connection manager
   - Update documentation
   - Add integration tests
   - Performance testing

## 3. Architecture Details

### Event Bus (Implemented ✓)
The new event bus provides:
- Type-safe event publishing and subscribing
- Async event handlers
- Error isolation between handlers
- Debug logging
- Clean unsubscribe functionality

Example usage:
```python
# Publishing events via decorator
@publish_event(EventType.AGENT_CREATED)
async def create_agent(agent: AgentCreate, db: Session):
    new_agent = crud.create_agent(db, **agent.dict())
    return new_agent  # Event is automatically published with agent data

# Subscribing to events
async def handle_agent_update(data: dict):
    agent_id = data["id"]
    # Handle the update...

event_bus.subscribe(EventType.AGENT_UPDATED, handle_agent_update)
```

### New Connection Manager (To Do)
Will provide:
- Single WebSocket connection per client
- Topic-based subscription system
- Integration with event bus
- Graceful connection management

### Migration Strategy
1. Build new components alongside existing ones
2. Migrate one event type at a time
3. Test thoroughly before removing old code
4. Keep both systems running until migration is complete

## 4. Testing Requirements

- [✓] Event bus unit tests
- [ ] Connection manager tests
- [ ] Integration tests with real WebSocket connections
- [ ] Load testing with multiple concurrent connections
- [ ] Migration tests (ensuring no events are lost)

## 5. Next Immediate Tasks

1. Integrate event bus with agent routes:
   - Modify CRUD operations to publish events
   - Create WebSocket handler for agent events
   - Test with frontend

2. Begin new connection manager:
   - Create basic structure
   - Implement topic subscription
   - Add tests

## 6. Notes & Considerations

- Keep both old and new systems running in parallel during migration
- Monitor for any performance impacts
- Document all changes for other developers
- Consider adding metrics/monitoring for event flow

## 7. Questions & Decisions Needed

- Should we migrate all event types at once or gradually?
- How to handle existing connections during migration?
- What metrics should we collect?

## 8. Key Issues and Consequences

- **Multiple WebSocket Endpoints:** Each new screen or action triggers a new connection (agent updates, thread messages, ephemeral tasks), sometimes duplicating subscriptions and data.  
- **Interwoven HTTP Requests & WebSockets:** Some updates come via REST calls, others through WebSockets, complicating state synchronization.  
- **Complex Client Lifecycle Management:** Because each channel has its own `onopen`, `onclose`, and `onmessage` logic, the frontend code is scattered and prone to race conditions.  
- **Tight Coupling:** Business logic that updates agents or messages often directly calls WebSocket broadcast methods, linking data persistence too closely with real-time notification.

## 9. Unified Single WebSocket Connection

The overarching concept is to maintain just one client–server WebSocket. Rather than open/close websockets on each navigation, the application keeps a single persistent connection alive, subscribing/unsubscribing to particular topics as needed.

### How It Works at a High Level

1. **Client-Side**  
   - On application startup, create and hold one WebSocket connection in a global manager (e.g., `wsClient` in your React or Vue store).  
   - When the user navigates from the dashboard to a thread, the frontend simply sends a "subscribe" message, like:  
     "type": "subscribe", "topics": ["thread:45"]  
   - Similarly, unsubscribing from a previous thread is done by sending another message:  
     "type": "unsubscribe", "topics": ["thread:12"]  
   - The client no longer tears down the entire socket on every route change—just updates its subscriptions.

2. **Server-Side**  
   - A single `/api/ws` endpoint handles all client connections.  
   - The server tracks topics that each connection is subscribed to, e.g., agent events, thread events, or other domain-specific notifications.  
   - When server-side code publishes an event (e.g., "agent updated," "new message," "chat chunk"), it uses a "ConnectionManager" to route those events only to clients subscribed to the relevant topic.

## 10. Event-Driven Architecture (EDA) with a Global Event Bus

By layering an internal event bus on top of the single WebSocket approach, we decouple business logic from real-time transport.  
- **Event Bus:** A central mechanism where any code (from CRUD endpoints, background tasks, or streaming logic) can publish an event (e.g., `AGENT_UPDATED`, `MESSAGE_CREATED`, etc.).  
- **WebSocket Subscriber:** A dedicated module subscribes to all event types, receiving them as they're published. It then checks which clients (via `ConnectionManager`) want to hear about those topics and broadcasts accordingly.  
- **Decorators for Emitting Events:** In your API routes, you can use decorators like "@emit_event(EventType.AGENT_UPDATED, ...) after a successful DB commit. This approach avoids writing direct broadcast calls in every route.

### Benefits

- **Decoupling:** Updating the DB and notifying subscribers are conceptually separate.  
- **Maintainability:** Adding new event types or real-time features is as simple as adding a publish call in the relevant code path.  
- **Scalability:** If you add more subscribers later (e.g., analytics, logs, or external services), you just subscribe them to the event bus.  
- **Consistent Real-Time Flow:** All real-time messages flow from the event bus through the single WebSocket, removing confusion.

## 11. Step-by-Step Refactoring Plan

1. **Create a Single WebSocket Endpoint**  
   - Replace multiple "/api/ws?thread_id=... /api/ws?agent_id=... with a single "/api/ws" route.  
   - Keep a global `ConnectionManager` that assigns each client a unique ID and tracks topic subscriptions.

2. **Build the Event Bus**  
   - Define an enum `EventType` with possible actions like "AGENT_CREATED," "THREAD_UPDATED," etc.  
   - Create a global `EventBus` with `subscribe()` and `publish()`.  
   - A `handle_event` function reacts to published events and calls the `ConnectionManager` to broadcast as needed.

3. **Implement Topic-Based Subscriptions**  
   - In `ConnectionManager`, maintain a structure like `topic -> set of client_ids`.  
   - When a client sends a "subscribe" message, add its ID to that topic's subscriber set.

4. **Use Decorators/Explicit Publishing**  
   - For each API route (create agent, update agent, post message, etc.), either use a decorator or explicitly call something like `event_bus.publish(EventType.AGENT_UPDATED, data=agent_data, targets=["agent:123"])`.

5. **Refactor the Frontend**  
   - A single `wsClient` is opened on app init.  
   - Remove code that closes/re-opens websockets upon route changes. Instead, dispatch subscription/unsubscription messages.  
   - Parse incoming messages by `message.type` and update state accordingly (e.g., if `message.type` == "MESSAGE_CREATED", dispatch to the thread's chat store).

6. **Gradually Remove Redundant REST Calls**  
   - You may still retain REST endpoints for initial data fetch or bulk queries.  
   - For real-time updates (like new chat messages, streaming partial responses), rely on the WebSocket events.

## 12. Conclusion

Moving to a single WebSocket approach, backed by an event bus, centralizes real-time logic. It reduces overhead from multiple socket endpoints, avoids race conditions, and unifies how the client subscribes to new data streams. Decoupling real-time notifications from business logic via decorators or explicit event publishing clarifies code structure and makes future features simpler to add.

--------------------------------------------------------------------------------

This final document merges the central ideas of the original and revised plans:  
• Only one WebSocket per client.  
• A robust event-driven mechanism on the server.  
• Topic-based subscribe/unsubscribe for targeted updates.  
• Decorators or explicit calls for emitting events after core actions—no more mixing broadcast logic directly into database operations.

By adopting these patterns, you'll streamline both frontend logic (single socket + subscription management) and backend code (clean separation of business logic and real-time notification).
