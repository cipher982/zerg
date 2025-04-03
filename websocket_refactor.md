# WebSocket Refactoring Plan: Single Connection + Event-Driven Architecture

## Status: Backend Complete ✓, Frontend Migration Starting
Last Updated: April 3, 2024

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

### Frontend Migration Starting 🔄
1. Planning phase:
   - Review existing `ws_client.rs` implementation
   - Design topic subscription management
   - Plan view-by-view migration strategy

### To Do 📋
1. Frontend Implementation:
   - Update `ws_client.rs` to use `/api/ws/v2` endpoint
   - Add topic subscription management
   - Migrate views one at a time:
     a) Agent list view (dashboard)
     b) Agent detail view
     c) Thread views
     d) Chat interface

2. Testing & Verification:
   - Add frontend integration tests
   - Test view migrations individually
   - Verify no message loss during transition
   - Monitor performance impacts

3. Clean up phase:
   - Remove old WebSocket code once migration is complete
   - Update documentation
   - Add performance monitoring
   - Final integration testing

## 3. Next Immediate Steps

1. Frontend Client Implementation:
   - Review current `ws_client.rs` structure
   - Plan changes needed for topic-based system
   - Design subscription management interface
   - Create migration plan for each view

2. View Migration Planning:
   - Document current WebSocket usage in each view
   - Plan subscription topics needed for each view
   - Design gradual rollout strategy

3. Testing Strategy:
   - Define test cases for new client
   - Plan view-specific integration tests
   - Set up performance monitoring

## 4. Migration Strategy

1. **Phase 1 - Frontend Preparation** [Next]
   - Create new WebSocket client implementation
   - Add topic subscription management
   - Keep old client functional during transition

2. **Phase 2 - Gradual Migration**
   - Start with agent events (already implemented)
   - Move thread events next
   - Monitor for any issues or performance impacts

3. **Phase 3 - Clean Up**
   - Remove old connection manager
   - Clean up deprecated code
   - Update documentation

## 5. Testing Status

### Completed ✓
- Event bus unit tests
- Topic manager unit tests
- WebSocket integration tests
- Error handling tests
- Connection management tests

### Remaining
- Frontend integration tests
- Load testing
- Migration tests
- Performance benchmarks

## 6. Notes & Considerations

- Keep both old and new systems running in parallel during migration
- Monitor for any performance impacts
- Document all changes for other developers
- Consider adding metrics/monitoring for event flow

## 7. Open Questions

- How to handle existing connections during the migration?
- What metrics should we collect to verify the new system's performance?
- Should we implement any circuit breakers or fallback mechanisms?

## 8. Benefits Achieved So Far

- ✓ Decoupled event handling from WebSocket management
- ✓ Cleaner subscription management with topics
- ✓ Better error handling and resource cleanup
- ✓ Improved testability with comprehensive test suite
- ✓ Complete backend implementation with topic-based system
- ✓ Backward compatibility maintained during migration

The backend implementation is now complete and well-tested. The focus shifts to the frontend migration, which will be done gradually to ensure stability and minimize disruption. Each view will be migrated individually, starting with the agent dashboard view.
