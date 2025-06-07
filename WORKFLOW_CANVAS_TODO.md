# Visual Workflow Canvas: Detailed Task List & Roadmap

## 1. Backend

### 1.1. Workflow CRUD API
- [x] **CRUD endpoints exist** (`/workflows`): create, read, update, delete (soft delete).
- [x] **Add automated tests for workflow CRUD**
    - [x] Unit tests for all endpoints (success, error, permissions).
    - [x] Test model constraints (unique, required fields, soft delete).
    - [x] Test edge cases (large canvas_data, concurrent edits).
    - [x] Regression tests for future schema changes.

### 1.2. Workflow Execution Engine
- [x] **Design and implement a workflow execution engine**
    - [x] Service/class to execute workflows node-by-node, resolving input/output dependencies.
    - [x] Support for Tool, Trigger, and Agent nodes (as per PRD and frontend models).
    - [x] Integrate with MCP tool execution and trigger firing.
    - [x] Handle errors, retries, and partial failures gracefully.
    - [x] Track execution state for each node (idle, queued, running, success, failed).
    - [x] Log execution details, errors, and data flow for debugging.
    - [x] Support for live feedback via WebSocket or polling.
    - [x] Support for manual and scheduled execution (trigger nodes).

### 1.3. Workflow Execution Persistence & API
- [x] **Add models for execution tracking**
    - [x] `WorkflowExecution` (run instance, status, timestamps, error, log).
    - [x] `NodeExecutionState` (per-node status, output, error).
- [x] **API endpoints**
    - [x] Start workflow execution (manual or via trigger).
    - [x] Get execution status and logs (live and historical).
    - [x] List execution history for a workflow.
    - [x] Export execution data (JSON/CSV).
- [x] **Security & Permissions**
    - [x] Ensure only workflow owners can execute/view their workflows and logs.

---

## 2. Frontend

### 2.1. Workflow Persistence & API Integration
- [ ] **Connect frontend to backend workflow CRUD API**
    - [ ] Fetch workflows from backend on app load.
    - [ ] Create/update/delete workflows via API (not localStorage).
    - [ ] Remove or minimize localStorage usage for workflows.
    - [ ] (Optional) Provide a one-time migration/import for any existing localStorage workflows.
    - [ ] Handle API errors and show user-friendly messages.
    - [ ] Ensure all workflow state is synced across devices and sessions.

### 2.2. Canvas & Node System
- [x] **Node system, palette, drag-and-drop, and modal-based config are robust.**
- [ ] **Polish and extend:**
    - [ ] Complete trigger configuration modal (UI, parameter mapping, validation).
    - [ ] Advanced tool configuration: input mapping from node outputs, validation, test logic.
    - [ ] Node resizing and layout improvements for complex workflows.
    - [ ] Accessibility and keyboard navigation for all canvas features.

### 2.3. Workflow Execution UI
- [ ] **Add UI for workflow execution feedback**
    - [ ] Show live node execution state (highlighting, progress, error/success icons).
    - [ ] Animate data flow along connections during execution.
    - [ ] Display execution logs and errors in a side panel or modal.
    - [ ] Show execution history and allow users to inspect past runs.
    - [ ] Support for manual and scheduled execution triggers from the UI.

### 2.4. Templates & Examples
- [ ] **Template gallery and onboarding**
    - [ ] Build a template gallery with categories and previews.
    - [ ] One-click template deployment and customization wizard.
    - [ ] Provide starter templates for common use cases (Gmail → Summarize → Slack, etc.).
    - [ ] Document how to create and share templates.

---

## 3. General / Cross-Cutting

### 3.1. Testing & Quality
- [ ] **Add e2e and integration tests**
    - [ ] Test workflow CRUD and execution flows end-to-end.
    - [ ] Test error handling, edge cases, and UI feedback.
    - [ ] Accessibility and performance tests for large workflows.

### 3.2. Documentation & Migration
- [ ] **Document all new APIs and frontend usage.**
- [ ] **Migration plan:** If any users have real data in localStorage, provide a migration/import tool and clear instructions.
- [ ] **Update onboarding and help docs** to reflect new workflow persistence and execution features.

### 3.3. Remove Technical Debt
- [ ] **Remove legacy localStorage code** once backend integration is complete.
- [ ] Refactor any code that assumes single-user or single-device usage.

---

## 4. Risks & Mitigations

- **LocalStorage usage**: Causes data loss, sync issues, and migration pain. Mitigation: Move to backend ASAP.
- **No backend tests**: Increases risk of regressions. Mitigation: Add comprehensive tests.
- **No execution engine**: Workflows can't run. Mitigation: Prioritize backend execution logic.
- **No frontend-backend sync**: Users can't collaborate or use multiple devices. Mitigation: Connect frontend to backend API.

---

## 5. Milestone Roadmap (Suggested Order)

1. Backend: Add tests for workflow CRUD.
2. Frontend: Connect to backend workflow API, remove localStorage.
3. Backend: Implement workflow execution engine and API.
4. Frontend: Add execution feedback UI.
5. Templates, onboarding, and documentation.
6. Remove technical debt and legacy code.
