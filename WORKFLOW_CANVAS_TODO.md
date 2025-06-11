# Visual Workflow Canvas: Detailed Task List & Roadmap

## 1. Backend

### 1.1. Workflow CRUD API
- [x] **CRUD endpoints exist** (`/workflows`): create, read, update, delete (soft delete).
- [x] **Add automated tests for workflow CRUD**
    - [x] Unit tests for all endpoints (success, error, permissions).
    - [x] Test model constraints (unique, required fields, soft delete).
    - [x] Test edge cases (large canvas_data, concurrent edits).
    - [x] Regression tests for future schema changes.

### 1.2. Workflow Execution Engine *(IN-PROGRESS)*
> We now have DB models (`WorkflowExecution`, `NodeExecutionState`) **and** API routes
> (`/api/workflow-executions/*`) but the actual engine that runs workflows is still
> a stub (`backend/zerg/services/workflow_engine.py`).

- [ ] **Design and implement a workflow execution engine**
    - [x] Service/class to execute workflows node-by-node, resolving input/output dependencies **(implemented – `WorkflowExecutionEngine`)**
    - [/] Support for Tool, Trigger, and Agent nodes – **placeholder executions merged**; real integration still pending.
    - [ ] Integrate with MCP tool execution and trigger firing.
    - [/] Handle errors, retries, and partial failures gracefully – **basic try/except added**; retries TBD.
    - [x] Track execution state for each node (idle, queued, running, success, failed) – stored in `node_execution_states` table.
    - [x] Log execution details, errors, and data flow for debugging – simple text log persisted on `WorkflowExecution`.
    - [/] Support for live feedback via WebSocket – **backend broadcast implemented**, front-end subscription & UI pending.
    - [ ] Support for manual and scheduled execution (trigger nodes).

    **✅ Already done** (so engine work can build on top):
    - DB schema for execution tracking.
    - CRUD helpers for execution rows.
    - FastAPI routes to start an execution, query status/logs/history, and export data.

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

**Note:** Execution API endpoints are functional but will return minimal data
until the engine in 1.2 is completed.

---

## 2. Frontend

### 2.1. Workflow Persistence & API Integration
- [ ] **Connect frontend to backend workflow CRUD API**
    - [x] Fetch workflows from backend on app load (tab-bar reflects server data).
    - [x] **Create – plus-tab (`＋`) hooks to `POST /api/workflows`**
        - [x] `create_workflow()` helper added to `network/api_client.rs`.
        - [x] `workflow_switcher` now dispatches `Message::CreateWorkflow` which
              triggers a `Command::CreateWorkflowApi` side-effect.
        - [/] Optimistic UI (spinner/tab) & toast rollback still TBD.

    - [ ] **Rename – context-menu → `PATCH /api/workflows/{id}`**
        - [ ] Add a small *⋯* button on each tab that opens a dropdown with
              “Rename” and “Delete”.
        - [ ] For *Rename*: open a modal (reuse the existing `rename_thread`
              modal component) asking for *Name* + *Description*.
        - [x] `rename_workflow()` helper implemented in `ApiClient`; command &
              reducer wiring merged.
        - [ ] UI work (modal + context menu) + error handling still pending.

    - [ ] **Delete – context-menu → `DELETE /api/workflows/{id}`**
        - [ ] Show confirmation dialog (`Are you sure? This does not delete
              runs or data, you can restore via Support in 30 days …`).
        - [x] `delete_workflow()` helper + command/reducer integration done.
        - [ ] UI context menu + optimistic hide & toast rollback pending.

    - [ ] **LocalStorage sunset & one-shot import**
        - [ ] When the app loads, *before* hitting the backend, check for the
              old `localStorage["zerg_workflows_v1"]` key.
        - [ ] If present:
              1. Parse JSON and iterate workflows.
              2. POST each to `/api/workflows` (fire-and-forget).
              3. After **all** succeed (or timeout), delete the LS key so we
                 never import again.
        - [ ] Remove **all remaining reads/writes** to that key throughout the
              codebase (grep `zerg_workflows_v1`).

    - [ ] **Error handling & UX**
        - [ ] Normalize backend error toasts (409 duplicate, 422 validation,
              network error).
        - [ ] Disable *Save* buttons while request in-flight.

    - [ ] Ensure all workflow state is synced across devices and sessions (the
              moment we persist to backend this is implicitly true as long as
              we re-fetch on app boot and after CRUD actions).

    *Current status (Jan 2025)*: **Create** is wired up end-to-end (＋ tab →
    backend → state refresh). **Workflow tabs are now visible** in Canvas Editor
    with proper dark theme styling. `rename/delete` helpers exist but UI triggers
    are not implemented yet, and LocalStorage migration still pending.

### 2.1.1. Workflow Tab Bar UI *(COMPLETED)*
- [x] **Fix workflow tab visibility issue**
    - [x] Corrected DOM insertion logic to work with current canvas layout
    - [x] Fixed view-specific initialization (tabs only appear on canvas, not dashboard)
    - [x] Updated styling to match dark theme design system
    - [x] Proper cleanup when switching away from canvas view
    - [x] Responsive design with proper spacing and hover states
    - [x] Dropdown / context-menu now portals into `#overlay-root` to avoid
          canvas clipping (July 2025).  Z-index tokens (`--z-toolbar`,
          `--z-overlay`) replace hard-coded values.

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
    - [x] Playwright canvas editor smoke tests added (`e2e/tests/canvas_workflows.spec.ts`).
    - [ ] Test workflow CRUD and execution flows end-to-end (backend + front-end).
    - [ ] Test error handling, edge cases, and UI feedback.
    - [ ] Accessibility and performance tests for large workflows.

### 3.2. Documentation & Migration
- [ ] **Document all new APIs and frontend usage.**
- [ ] **Migration plan:** If any users have real data in localStorage, provide a migration/import tool and clear instructions.
- [ ] **Update onboarding and help docs** to reflect new workflow persistence and execution features.

### 3.3. Remove Technical Debt
- [ ] **Remove legacy LocalStorage code** once backend integration is complete.
    - [ ] Grep for `zerg_workflows_v1` and generic `localStorage` helpers inside
          `frontend/src/` – particularly `canvas`, `state.rs`, and old helper
          modules.
    - [ ] Delete `load_workflows_from_local_storage()` and
          `save_workflows_to_local_storage()` (or mark them deprecated during
          the migration PR, then remove in the following release).
    - [ ] Replace with an *import only* helper that warns (console + toast) if
          we still detect the stale key after the migration – indicates a user
          had import errors.
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
