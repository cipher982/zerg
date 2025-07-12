# WebSocket Protocol Unification Implementation Plan

## Current State Analysis

### Problems Identified

1. **Complex Transformation Logic** (`frontend/src/network/topic_manager.rs:222-320`):
   - Runtime format detection between envelope vs flat format
   - Auto-detection that tries both wrapper and flattened formats
   - Performance overhead from double deserialization attempts
   - Complex branching logic with error-prone format guessing

2. **Manual Alias Management**:
   - Hand-maintained alias lists in frontend Rust enums
   - Backend event bus → WebSocket type mapping requires manual updates
   - No single source of truth for message types and aliases

3. **Dual Protocol Support**:
   - Backend supports both enveloped and legacy formats via `MessageEnvelopeHelper`
   - Frontend has v1 fallback logic with manual transformation
   - Inconsistent message handling across components

4. **Type Safety Gaps**:
   - Runtime message parsing that can fail silently
   - String-based message type matching instead of compile-time validation
   - Manual topic routing logic duplicated across handlers

### Generated Artifacts

✅ **Schema Definition**: `ws-protocol.yml` - Single source of truth
✅ **Code Generation**: `scripts/generate-ws-types-modern.py` - Modern AsyncAPI-based type generation  
✅ **Generated Types**: 
  - `backend/zerg/generated/ws_messages.py` - Pydantic models
  - `frontend/src/generated/ws_messages.rs` - Rust structs/enums
✅ **Contract Tests**: `tests/test_ws_protocol_contracts.py` - Validation suite

## Migration Strategy

### Phase 1: Foundation (Week 1)

#### 1.1 Validate Generated Types
```bash
# Run contract tests to ensure generated types work
uv run pytest tests/test_ws_protocol_contracts.py -v

# Test backend integration
uv run python -c "from backend.zerg.generated.ws_messages import Envelope; print('✅ Python types work')"

# Test frontend compilation
cd frontend && cargo check --features generated
```

#### 1.2 Update Backend Message Sending
**Target**: `backend/zerg/websocket/manager.py`

**Before** (lines 89-95):
```python
# Complex envelope detection and wrapping
if MessageEnvelopeHelper.is_enveloped(message):
    await websocket.send_json(message)
else:
    envelope = MessageEnvelopeHelper.ensure_envelope(message, topic)
    await websocket.send_json(envelope.model_dump())
```

**After**:
```python
# Always use generated Envelope
from zerg.generated.ws_messages import Envelope

envelope = Envelope.create(message_type, topic, payload_data, req_id)
await websocket.send_json(envelope.model_dump())
```

#### 1.3 Update Backend Message Handlers
**Target**: `backend/zerg/websocket/handlers.py`

Replace manual envelope wrapping with direct envelope creation:
```python
# Replace all broadcast_to_topic calls
def broadcast_run_update(run_data: dict, agent_id: int):
    envelope = Envelope.create("run_update", f"agent:{agent_id}", run_data)
    topic_manager.broadcast_to_topic(f"agent:{agent_id}", envelope)
```

#### 1.4 Backend Testing
```bash
# Ensure all existing WebSocket tests pass
uv run pytest backend/tests/test_websocket*.py -v
```

### Phase 2: Frontend Transformation Elimination (Week 2)

#### 2.1 Replace TopicManager Transformation Logic
**Target**: `frontend/src/network/topic_manager.rs:222-320`

**Current Complex Logic**:
```rust
// 100 lines of format detection and transformation
if let Ok(envelope) = serde_json::from_value::<Envelope>(message.clone()) {
    // Auto-detect if this message type needs data wrapper by trying both formats
    let needs_data_wrapper = {
        // Try wrapped format
        let wrapped = json!({ "type": message_type, "data": envelope.data.clone() });
        // Try flattened format  
        let flattened = if let Value::Object(mut map) = envelope.data.clone() {
            map.insert("type".to_string(), Value::String(message_type.clone()));
            Value::Object(map)
        } else { /* ... */ };
        // Complex decision logic...
    };
    // More transformation logic...
}
```

**Replacement**:
```rust
use crate::generated::ws_messages::{Envelope, WsMessage};

pub fn route_incoming_message(&self, message: serde_json::Value) {
    // Always expect envelope format - no fallback
    match serde_json::from_value::<Envelope>(message) {
        Ok(envelope) => {
            // Direct deserialization based on message type
            match envelope.message_type.as_str() {
                "run_update" => {
                    if let Ok(payload) = serde_json::from_value::<RunUpdateData>(envelope.data) {
                        self.dispatch_to_topic_handlers(&envelope.topic, WsMessage::RunUpdate { data: payload });
                    }
                }
                "agent_event" | "agent_created" | "agent_updated" | "agent_deleted" => {
                    if let Ok(payload) = serde_json::from_value::<AgentEventData>(envelope.data) {
                        self.dispatch_to_topic_handlers(&envelope.topic, WsMessage::AgentEvent { data: payload });
                    }
                }
                // Handle all message types explicitly - no auto-detection
                _ => web_sys::console::warn_1(&format!("Unknown message type: {}", envelope.message_type).into())
            }
        }
        Err(e) => {
            web_sys::console::error_1(&format!("Invalid envelope format: {}", e).into());
            // No v1 fallback - reject malformed messages
        }
    }
}
```

#### 2.2 Update Frontend Message Handlers
**Targets**: 
- `frontend/src/components/dashboard/ws_manager.rs`
- `frontend/src/components/chat/ws_manager.rs`

Replace string-based message handling with typed enums:
```rust
// Before: String matching
match message_data.get("type").and_then(|v| v.as_str()) {
    Some("run_update") => { /* manual JSON extraction */ }
    Some("agent_event") => { /* manual JSON extraction */ }
    _ => {}
}

// After: Typed matching  
match ws_message {
    WsMessage::RunUpdate { data } => self.handle_run_update(data),
    WsMessage::AgentEvent { data } => self.handle_agent_event(data),
    WsMessage::Unknown => {} // Explicit unknown handling
}
```

#### 2.3 Remove Legacy Schema Files
Delete transformation logic and manual type definitions:
- Remove `frontend/src/network/ws_schema.rs` (replace with generated)
- Remove manual alias handling in existing enums
- Clean up `frontend/src/network/event_types.rs` redundancies

### Phase 3: Contract Validation & Hardening (Week 3)

#### 3.1 Comprehensive Contract Testing
```bash
# Add contract tests to CI pipeline
echo "uv run pytest tests/test_ws_protocol_contracts.py" >> scripts/run_all_tests.sh

# Frontend contract tests
cat > frontend/src/tests/ws_contract_tests.rs << 'EOF'
use crate::generated::ws_messages::*;

#[test]
fn test_envelope_deserialization() {
    let json = r#"{"v":1,"type":"run_update","topic":"agent:123","ts":1641024000000,"data":{"id":123,"agent_id":456,"status":"running"}}"#;
    let envelope: Envelope = serde_json::from_str(json).unwrap();
    assert_eq!(envelope.message_type, "run_update");
}
EOF
```

#### 3.2 Add Schema Validation Middleware
**Backend**: `backend/zerg/websocket/middleware.py`
```python
from zerg.generated.ws_messages import Envelope
import jsonschema

def validate_incoming_message(message: dict) -> Envelope:
    """Validate all incoming messages against schema."""
    try:
        return Envelope.model_validate(message)
    except Exception as e:
        raise ValueError(f"Invalid message format: {e}")
```

**Frontend**: Schema validation at WebSocket boundary
```rust
fn validate_envelope(data: &Value) -> Result<Envelope, String> {
    serde_json::from_value(data.clone())
        .map_err(|e| format!("Envelope validation failed: {}", e))
}
```

#### 3.3 Version Negotiation
Add protocol version checking to prevent incompatible clients:
```python
# Backend connection handler
if envelope.v != 1:
    await websocket.close(code=4400, reason="Unsupported protocol version")
```

### Phase 4: Legacy Removal & Cleanup (Week 3)

#### 4.1 Remove Transformation Infrastructure
Delete files that are no longer needed:
- `backend/zerg/schemas/ws_messages.py` (replace with generated)
- Remove `MessageEnvelopeHelper` class entirely
- Clean up manual alias lists and compatibility shims

#### 4.2 Update Documentation
- Update WebSocket API documentation with new unified format
- Add schema evolution guide for future message types
- Document code generation workflow

#### 4.3 Performance Validation
```bash
# Benchmark message throughput before/after
uv run python scripts/benchmark_ws_performance.py

# Memory usage analysis
uv run python scripts/analyze_ws_memory.py
```

## Success Criteria Validation

### Technical Metrics
- **Zero runtime message parsing failures**: Contract tests ensure all formats work
- **Single wire protocol**: Only envelope format accepted/sent
- **Zero manual compatibility maintenance**: All types generated from schema
- **Compile-time contract validation**: Schema changes break builds, not users
- **Forward compatibility**: Additive schema changes don't break existing clients

### Performance Metrics  
- **Message parsing error rate**: 0% (schema validation prevents invalid messages)
- **WebSocket-related bug reports**: 0 per release (types prevent common errors)
- **Protocol maintenance time**: <30min per quarter (schema updates only)
- **New message type deployment**: Single commit (update schema, regenerate)

## Risk Mitigation

### Deployment Safety
- **Feature flags**: New protocol behind feature flag during transition
- **Gradual rollout**: Backend supports both formats temporarily during migration
- **Instant rollback**: Revert to old topic_manager.rs if issues arise
- **Monitoring**: Track message parsing errors and connection failures

### Testing Strategy
- **Contract tests**: Validate round-trip serialization for all message types
- **Integration tests**: End-to-end WebSocket flows in test environment  
- **Performance tests**: Ensure no regression in message throughput
- **Compatibility tests**: Verify existing client behavior unchanged

## Implementation Commands

### Week 1: Foundation
```bash
# Validate generated code
uv run pytest tests/test_ws_protocol_contracts.py

# Update backend message sending
# Edit: backend/zerg/websocket/manager.py
# Edit: backend/zerg/websocket/handlers.py

# Test backend changes
uv run pytest backend/tests/test_websocket*.py
```

### Week 2: Frontend Migration  
```bash
# Replace transformation logic
# Edit: frontend/src/network/topic_manager.rs
# Edit: frontend/src/components/dashboard/ws_manager.rs  
# Edit: frontend/src/components/chat/ws_manager.rs

# Test frontend changes
cd frontend && cargo test
```

### Week 3: Validation & Cleanup
```bash
# Add contract validation
# Create: backend/zerg/websocket/middleware.py
# Create: frontend/src/tests/ws_contract_tests.rs

# Remove legacy code
# Delete: backend/zerg/schemas/ws_messages.py
# Delete transformation logic from topic_manager.rs

# Full test suite
uv run python scripts/run_all_tests.sh
```

This plan eliminates the transformation complexity while maintaining backward compatibility during the migration, then removes all legacy code for a clean, maintainable WebSocket architecture.