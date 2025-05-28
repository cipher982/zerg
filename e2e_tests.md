# E2E Test Expansion Plan for Zerg Agent Platform

## Context

The Zerg Agent Platform is a sophisticated AI agent management system with:
- **Frontend**: Rust/WASM-based SPA with multiple views (Dashboard, Canvas, Profile, Chat)
- **Backend**: FastAPI with PostgreSQL, supporting agents, threads, triggers, and real-time WebSocket communication
- **Current E2E Coverage**: Only 4 basic tests covering dashboard rendering, scope toggle, canvas agent shelf, and modal tabs

## Goals

1. **Comprehensive Coverage**: Test all major user workflows end-to-end
2. **User-Centric Testing**: Focus on typical user journeys rather than implementation details
3. **Regression Prevention**: Catch integration issues between frontend and backend
4. **Documentation**: Tests serve as living documentation of system behavior
5. **Confidence**: Enable rapid development with safety net of thorough tests

## Test Implementation Checklist

### 🔴 Priority 1: Core Agent Management
- [x] **agent_crud.spec.ts**
  - [ ] Create agent with minimal configuration
  - [ ] Create agent with all fields (instructions, model, temperature)
  - [ ] Validate required fields show errors
  - [ ] Edit agent name and verify update
  - [ ] Edit agent instructions
  - [ ] Change agent model selection
  - [ ] Delete agent with confirmation dialog
  - [ ] Cancel delete operation
  - [ ] Verify agent appears in dashboard after creation
  - [ ] Test agent status toggle (active/inactive)

### 🔴 Priority 2: Thread & Chat Functionality
- [x] **thread_chat.spec.ts**
  - [ ] Create new thread from agent dashboard
  - [ ] Send user message in thread
  - [ ] Wait for and verify agent response
  - [ ] Send follow-up message in same thread
  - [ ] Create multiple threads for same agent
  - [ ] Switch between threads
  - [ ] Delete thread and verify removal
  - [ ] Test thread title editing
  - [ ] Verify message history persistence
  - [ ] Test empty thread state

### 🔴 Priority 3: Authentication & Authorization
- [x] **auth_flows.spec.ts**
  - [ ] Test login redirect when unauthenticated
  - [ ] Mock Google OAuth flow
  - [ ] Verify user profile data after login
  - [ ] Test logout functionality
  - [ ] Verify session persistence on reload
  - [ ] Test unauthorized access attempts
  - [ ] Admin vs regular user permissions
  - [ ] Profile view navigation and display

### 🟡 Priority 4: Canvas Editor
- [x] **canvas_workflows.spec.ts**
  - [ ] Drag agent from shelf to canvas
  - [ ] Position node at specific coordinates
  - [ ] Create edge between two nodes
  - [ ] Delete node from canvas
  - [ ] Delete edge between nodes
  - [ ] Select multiple nodes
  - [ ] Save workflow state
  - [ ] Load saved workflow
  - [ ] Clear canvas
  - [ ] Test zoom/pan controls

### 🟡 Priority 5: Agent Scheduling
- [x] **agent_scheduling.spec.ts**
  - [ ] Set cron schedule on agent
  - [ ] Verify next_run_at displays correctly
  - [ ] Edit existing schedule
  - [ ] Remove schedule from agent
  - [ ] Test invalid cron expressions
  - [ ] Verify scheduled status indicator
  - [ ] Test schedule in different timezones
  - [ ] View last_run_at after execution

### 🟡 Priority 6: Trigger Management
- [x] **trigger_webhooks.spec.ts**
  - [ ] Create webhook trigger
  - [ ] Copy webhook URL
  - [ ] View webhook secret
  - [ ] Delete webhook trigger
  - [ ] Test multiple triggers per agent
  
- [x] **trigger_email.spec.ts**
  - [ ] Create Gmail trigger
  - [ ] Configure email filters
  - [ ] Test Gmail connection status
  - [ ] Remove email trigger
  - [ ] Handle disconnected Gmail account

### 🟢 Priority 7: Run History & Monitoring
- [x] **agent_runs.spec.ts**
  - [ ] View run history for agent
  - [ ] Check run status indicators
  - [ ] View run details (duration, tokens)
  - [ ] Filter runs by status
  - [ ] View error details for failed runs
  - [ ] Test pagination of run history
  - [ ] Export run data

### 🟢 Priority 8: Real-time Features
- [x] **realtime_updates.spec.ts**
  - [ ] Test live dashboard updates
  - [ ] Verify WebSocket connection
  - [ ] Test message streaming
  - [ ] Multi-tab synchronization
  - [ ] Connection recovery after disconnect
  - [ ] Test presence indicators

### 🟢 Priority 9: Search & Filtering
- [x] **search_filter.spec.ts**
  - [ ] Search agents by name
  - [ ] Filter by agent status
  - [ ] Filter by owner (admin view)
  - [ ] Sort by name ascending/descending
  - [ ] Sort by created date
  - [ ] Sort by last run time
  - [ ] Combine search and filters
  - [ ] Clear all filters

### 🔵 Priority 10: Error Handling
- [x] **error_scenarios.spec.ts**
  - [ ] Network timeout handling
  - [ ] 404 error pages
  - [ ] Form validation errors
  - [ ] API error messages
  - [ ] Session expiry handling
  - [ ] Rate limit responses
  - [ ] Offline mode behavior

### 🔵 Priority 11: Performance Tests
- [x] **performance.spec.ts**
  - [ ] Load dashboard with 100+ agents
  - [ ] Scroll through long message history
  - [ ] Rapid message sending
  - [ ] Multiple concurrent operations
  - [ ] Large canvas with many nodes
  - [ ] Memory leak detection

## Implementation Guidelines

### Test Structure
```typescript
// Example test structure
test.describe('Feature: Agent Management', () => {
  test.beforeEach(async ({ page }) => {
    // Setup: Login, navigate to dashboard
  });

  test('should create new agent successfully', async ({ page }) => {
    // Arrange
    // Act
    // Assert
  });
});
```

### Best Practices
- ✅ Use data-testid attributes for reliable element selection
- ✅ Test user-visible behavior, not implementation
- ✅ Include both happy path and error scenarios
- ✅ Use meaningful test descriptions
- ✅ Keep tests independent and idempotent
- ✅ Mock external services (OAuth, email) where needed
- ✅ Use Page Object Model for complex interactions
