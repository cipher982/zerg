# Visual Workflow Canvas: Detailed Task List & Roadmap

## 0. WebSocket Protocol Alignment  *(pre-requisite for all realtime work)*

The WebSocket contract is now the single source-of-truth, driven by
`asyncapi/chat.yml` and enforced by CI via code-generation & Pact tests.
Any new realtime payload *must* follow this flow:

1. Extend **`asyncapi/chat.yml`** – add the channel and message schemas.
2. `make regen-ws-code` – regenerate Rust & TS SDKs and commit the diff.
3. `make pact-capture && make pact-verify` – update consumer contract and
   ensure the provider passes.
4. Only then implement backend emitters and frontend handlers.

Upcoming additions required for the Workflow Canvas work:

• `workflow_execution:{execution_id}` channel
• `NodeState`  – live per-node status updates  
• `ExecutionFinished` – marks the overall run completion  
• `NodeLog` (optional) – streamed `stdout/stderr` lines for the log drawer

All three messages are now in the spec and the generated SDKs / Pact files are
committed (Jul-14-2025).  ✅ **Protocol alignment complete**.

## 1. Backend

### 1.1. Workflow CRUD API
- [x] **CRUD endpoints exist** (`/workflows`): create, read, update, delete (soft delete).
- [x] **Add automated tests for workflow CRUD**
    - [x] Unit tests for all endpoints (success, error, permissions).
    - [x] Test model constraints (unique, required fields, soft delete).
    - [x] Test edge cases (large canvas_data, concurrent edits).
    - [x] Regression tests for future schema changes.

### 1.2. Workflow Execution Engine *(v0 COMPLETE — upcoming milestones)*
The first production-quality engine shipped in
`backend/zerg/services/workflow_engine.py`. It executes nodes **linearly**,
persists results, and streams per-node updates over WebSocket. The checklist
below tracks what is left for **v1** (anything unticked blocks GA):

- [x] Linear execution engine + DB persistence
- [ ] Real execution for Tool / Trigger / Agent nodes (currently mock output)
- [ ] MCP tool integration & trigger firing *(moved from “partial” → clearly
  **todo** so risk tracking is accurate)*
- [ ] Error handling & configurable retries (basic try/except exists ‑ needs
  policy & back-off strategy)
- [x] WebSocket broadcast (`node_state` envelope)
- [x] WebSocket broadcast (`execution_finished`, `node_log`) events
- [ ] Front-end subscription & visualisation (see section 2.3)
- [ ] DAG traversal & parallel branches
- [ ] Manual vs scheduled runs (APScheduler hook via Trigger nodes)

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

**Note:** Endpoints now stream real-time node data produced by the linear
engine; payload shape is considered **beta** until DAG support lands.

---

## 2. Frontend

> **Protocol prerequisite** — The WebSocket contract is now validated against
> `asyncapi/chat.yml` at every CI run.  Before adding or changing frontend
> features that depend on new realtime payloads you **must first** extend the
> AsyncAPI spec (and regenerate SDKs + Pact files) so the build stays green.

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

    - [x] **LocalStorage migration** – legacy key `zerg_workflows_v1` has been
          fully removed from the code-base; no import flow required for new
          users.  Focus shifts to persisting the remaining *layout* data:
          - [ ] Add `/api/workflows/{id}/layout` (GET/PUT) – viewport & node positions
          - [ ] Front-end save/load helpers; drop LS fallback once API is live

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

### 2.1a. Workflow Tab Bar UI *(COMPLETED — kept for changelog)*
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

### 2.3. Workflow Execution UI & UX polish
- [ ] **Add rich execution feedback**
    - [x] Highlight nodes on **running / success / failed** (colour-coded).
    - [ ] Spinner / pulse states on the ▶︎ *Run* button (start → running → ok/error).
    - [ ] Glow animation on *connections* while a node is running.
    - [ ] **Log drawer** (collapsible, 25 vh) that streams `node_log` frames.
        * Backend must emit `node_log` envelope; see Section 0 below.*
        * Backend emitter implemented (Jul-14-2025) – front-end still needs UI.*
    - [ ] Display execution history and allow inspection of past runs.
    - [ ] Manual & scheduled execution triggers from the UI.

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
- [x] **Purge legacy LocalStorage workflow code** (`zerg_workflows_v1`) – helpers and key removed.
- [ ] Remove LocalStorage fallback for **canvas layout** once new endpoint ships (see 2.1).
- [ ] Refactor any code that assumes single-user or single-device usage (e.g. cached JWT, hard-coded `user_id=1`).

---

## 4. Risks & Mitigations

- **LocalStorage (canvas layout only)** – Workflows themselves live on the backend; the only client-side risk left is unsaved viewport/node positions.  **Mitigation**: land `/layout` endpoint (section 2.1) and then delete the LS fallback.
- **WebSocket contract drift** – AsyncAPI 3 spec is enforced at CI; adding new messages (NodeState, NodeLog, ExecutionFinished) without updating the spec will break the build.  **Mitigation**: update `asyncapi/chat.yml` *before* backend emits anything new and run `make regen-ws-code`.
- **Missing backend tests** – Increases regression risk.  **Mitigation**: extend existing pytest suite for new execution & log endpoints.
- **Incomplete execution engine** – Without real node execution, workflows provide little value.  **Mitigation**: prioritise section 1.2 work.
- **Frontend–backend state drift** – Users cannot collaborate across devices unless canvas layout & execution state are fully server-backed.  **Mitigation**: finish 2.1 (layout persistence) and ensure re-fetch on app boot.

---

## 5. Milestone Roadmap (Suggested Order)

**0. Protocol alignment (DONE Jul-14-2025)** – `workflow_execution:{id}` channel + `NodeState`, `ExecutionFinished`, `NodeLog` messages added to `asyncapi/chat.yml`; SDKs regenerated & Pact contracts updated.

1. Front-end: Execution **UX polish** (button states, connection glow, log drawer).
2. Back-end: Real node execution (Tool / Agent / Trigger) + retry policy.
3. Front-end: Workflow tab context-menu (rename / delete) & error toasts.
4. Persist canvas layout to backend and remove LocalStorage fallback.
5. Template gallery & onboarding docs.
6. Clean-up tech-debt & multi-device edge-cases.
