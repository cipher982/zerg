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

### 1.2. Workflow Execution Engine *(v1 COMPLETE — production ready)*
The production workflow engine in `backend/zerg/services/workflow_engine.py` now
supports full DAG execution with real node types and parallel processing.
All major backend execution features are now complete:

- [x] Linear execution engine + DB persistence
- [x] **Real execution for Tool / Trigger / Agent nodes** *(COMPLETED)*
- [x] **MCP tool integration & trigger firing** *(COMPLETED)*
- [x] **Error handling & configurable retries** *(COMPLETED)*
- [x] WebSocket broadcast (`node_state` envelope)
- [x] WebSocket broadcast (`execution_finished`, `node_log`) events
    - [x] Front-end subscription & visualisation (see section 2.3)
- [x] **DAG traversal & parallel branches** *(COMPLETED)*
- [x] **Manual vs scheduled runs (APScheduler hook via Trigger nodes)** *(COMPLETED)*

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
- [x] **Connect frontend to backend workflow CRUD API** *(COMPLETED Jan 2025)*
    - [x] Fetch workflows from backend on app load (tab-bar reflects server data).
    - [x] **Create – plus-tab (`＋`) hooks to `POST /api/workflows`**
        - [x] `create_workflow()` helper added to `network/api_client.rs`.
        - [x] `workflow_switcher` now dispatches `Message::CreateWorkflow` which
              triggers a `Command::CreateWorkflowApi` side-effect.
        - [/] Optimistic UI (spinner/tab) & toast rollback still TBD.

    - [x] **Rename – context-menu → `PATCH /api/workflows/{id}`**
        - [x] *Rename* action accessible via toolbar dropdown (⋮) – prompts for name & description.
        - [x] For *Rename*: context-menu action prompts for *Name* & *Description*, dispatches API call (Jul-14-2025).
        - [x] `rename_workflow()` helper implemented in `ApiClient`; command &
              reducer wiring merged.
        - [x] UI finished – item lives in ⋯ dropdown; optimistic UI handled by reducer.

    - [x] **Delete – context-menu → `DELETE /api/workflows/{id}`**
        - [x] Confirmation dialog added; calls `DELETE /api/workflows/{id}`. (Jul-14-2025)
        - [x] `delete_workflow()` helper + command/reducer integration done.
        - [x] UI context menu wired.

    - [x] **LocalStorage migration** – legacy key `zerg_workflows_v1` has been
          fully removed from the code-base; no import flow required for new
          users.  Focus shifts to persisting the remaining *layout* data:
          - [x] Add `/api/workflows/{id}/layout` (GET/PUT) – viewport & node positions *(backend merged Aug-14-2025)*
          - [x] Front-end save/load helpers; dropped LS fallback now that backend endpoint is live (Aug-2025)

    - [x] **Error handling & UX** *(COMPLETED Jan 2025)*
        - [x] Normalize backend error toasts (409 duplicate, 422 validation,
              network error).
        - [x] Disable *Save* buttons while request in-flight.

    - [x] Ensure all workflow state is synced across devices and sessions *(COMPLETED - backend persistence active)*

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
- [x] **Polish and extend:** *(COMPLETED Jan 2025)*
    - [x] Complete trigger configuration modal (UI, parameter mapping, validation).
    - [x] Advanced tool configuration: input mapping from node outputs, validation, test logic.
    - [ ] Node resizing and layout improvements for complex workflows.
    - [ ] Accessibility and keyboard navigation for all canvas features.

### 2.3. Workflow Execution UI & UX polish
- [ ] **Add rich execution feedback**
    - [x] Highlight nodes on **running / success / failed** (colour-coded).
    - [x] Spinner / pulse states on the ▶︎ *Run* button (start → running → ok/error) – **implemented Jul-14-2025**.
    - [x] Glow animation on *connections* while a node is running – implemented Jul-14-2025.  Animated dashed line with soft blue glow indicates active execution.
    - [x] **Log drawer** (collapsible, 25 vh) streaming live `node_log` frames – backend & initial UI merged (toggle via 📜 button).
        * Backend emitter implemented (Jul-14-2025) – front-end still needs UI.*
    - [x] Display execution history – 🕒 sidebar lists last runs, click selects (Aug-2025).  
      [ ] Detailed node-state replay still pending.
    - [ ] Manual & scheduled execution triggers from the UI.

### 2.4. Templates & Examples
- [x] **Template gallery and onboarding** *(COMPLETED Jan 2025)*
    - [x] Build a template gallery with categories and previews.
    - [x] One-click template deployment and customization wizard.
    - [x] Backend API for template CRUD operations with category filtering.
    - [ ] Provide starter templates for common use cases (Gmail → Summarize → Slack, etc.).
    - [ ] Document how to create and share templates.

---

## 3. General / Cross-Cutting

### 3.1. Testing & Quality
- [x] **Add e2e and integration tests** *(COMPLETED Jan 2025)*
    - [x] Playwright canvas editor smoke tests added (`e2e/tests/canvas_workflows.spec.ts`).
    - [x] Test workflow CRUD and execution flows end-to-end (backend + front-end).
    - [x] Comprehensive workflow execution E2E tests (`e2e/tests/workflow_execution.spec.ts`).
    - [x] Template gallery E2E tests (`e2e/tests/template_gallery.spec.ts`).
    - [ ] Test error handling, edge cases, and UI feedback.
    - [ ] Accessibility and performance tests for large workflows.

### 3.2. Documentation & Migration
- [ ] **Document all new APIs and frontend usage.**
- [ ] **Migration plan:** If any users have real data in localStorage, provide a migration/import tool and clear instructions.
- [ ] **Update onboarding and help docs** to reflect new workflow persistence and execution features.

### 3.3. Remove Technical Debt
- [x] **Purge legacy LocalStorage workflow code** (`zerg_workflows_v1`) – helpers and key removed.
- [x] Remove LocalStorage fallback for **canvas layout** now that `/api/graph/layout` is live.
- [ ] Refactor any code that assumes single-user or single-device usage (e.g. cached JWT, hard-coded `user_id=1`).
- [x] Remove `unreachable-pattern` warnings in `update.rs` – cleaned up duplicate match arms (Jul-14-2025).

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

### MAJOR MILESTONES COMPLETED (Jan 2025):

1. ✅ **Backend Workflow Engine (v1.0)**: Complete DAG execution with real node types
   - Real Tool/Agent/Trigger node execution replacing all mock implementations
   - Full MCP tool integration with unified tool registry
   - Comprehensive retry policies with configurable backoff strategies  
   - Parallel node execution with dependency resolution
   - Error handling, cancellation support, and real-time WebSocket updates
   - Manual vs scheduled workflow execution with APScheduler integration

2. ✅ **Frontend Stability & UX**: Production-ready canvas interface  
   - Fixed all critical panic errors in workflow_switcher.rs
   - Resolved layout positioning issues (workflow bar placement)
   - Execution status visualization (button states, connection glow, log drawer)
   - Robust error handling throughout the frontend stack

3. ✅ **Integration & Polish**: End-to-end workflow execution
   - Live WebSocket updates for node states and execution progress
   - Complete workflow CRUD operations with backend persistence
   - Real-time log streaming and execution history tracking

4. ✅ **UX & Quality Improvements** *(Jan 2025)*: Production-ready polish
   - Enhanced error handling with normalized HTTP error toasts (409, 422, 5xx)
   - Button state management to prevent double-submission during API calls
   - Comprehensive template gallery system with backend API and frontend UI
   - Template categories, deployment wizard, and one-click workflow creation
   - Complete E2E test coverage for workflow execution and template gallery
   - Enhanced trigger and tool configuration modals with validation and parameter mapping

### REMAINING WORK (Lower Priority):
5. Canvas layout persistence to backend (LocalStorage fallback removal)
6. Node resizing and layout improvements for complex workflows
7. Accessibility and keyboard navigation for all canvas features  
8. Create starter templates for common use cases (Gmail → Summarize → Slack, etc.)
9. Documentation: API docs, onboarding guides, migration plans
10. Clean-up tech-debt & multi-device edge-cases

---

## 6. Deeper-Dive Task Break-downs *(added Aug-2025)*

The high-level checklist above has served well for sprint-level planning, but
as the project moves toward **Workflow GA (v1.0)** we need more granular
engineering tasks so individual contributors can pick up work without a large
handover.  The sections below expand on the remaining unticked items ‑ each
bullet is intended to become its own GitHub issue (labelled **`workflow-v1`**)
once refined in triage.

### 6.1 Backend – Real Node Execution

> Goal: replace stubbed/mock node outputs with real functional logic and make
> the engine production-ready for diverse node types.

1. **Agent Node**
   • Invoke **`AgentRunner`** directly with the node's `agent_id`.  
   • Map upstream outputs → `user_message` for the agent.  
   • Persist the resulting assistant/tool messages back onto the **current
     workflow execution** rather than the canonical chat thread (avoid
     pollution).

2. **Tool Node**
   • Resolve tool reference: first search **MCP servers** in the workflow’s
     `mcp_servers[]`, then fall back to built-in tools.  
   • Stream tool output via `node_log` frames in addition to emitting the
     final `NodeState` (status=success, output=…).  
   • Add timeout setting (config param, default 120 s) – engine marks node as
     failed if exceeded and moves to error policy handler.

3. **Trigger Node**
   • Distinguish between *event* triggers (Gmail, HTTP webhook) and *time*
     triggers (CRON).  
   • For event-based triggers: verify HMAC signature exactly like
     `/api/triggers/{id}/events` endpoint.  
   • For time-based: schedule an APScheduler job that creates a new
     **WorkflowExecution** with `triggered_by="schedule"`.

4. **Shared Concerns**
   • **Retry policy** (see 6.2) applies to all failing nodes.  
   • **Idempotency** – engine must detect duplicate trigger events by
     `event_id` dedupe column.

Deliverables: green pytest suite for
`tests/test_workflow_execution_engine.py::test_real_node_execution` (currently
`xfail`), open-API docs updated, two new Pact interactions (success & error).

### 6.2 Backend – Error Handling & Retry Policy

1. **Policy DSL**
   • JSON field on Workflow: `{ "retries": {"default": 2, "backoff": "exp" } }`
     (backoff=`linear|exp|none`).  
   • Override per-node via node config panel.

2. **Engine Implementation**
   • Wrap each node `run()` in `try/except` catching `Exception` *and*
     asyncio-cancellation.  
   • Increment attempt counter, sleep according to back-off, then retry.
   • Emit `node_log` line `RETRY n/…` and update `NodeState` → `retrying` – ✅ **Implemented Aug-2025**

3. **User Cancellation**  ✅ **DONE (Aug-2025)**
   • `/api/workflow_executions/{id}/cancel` (PATCH + body `{reason}`) implemented.  
   • Engine now cooperatively checks a cancellation flag before each node and just before
     marking success, exiting early when requested.

4. **Tests**
   • Unit tests for back-off timing (monkeypatch `asyncio.sleep`).  
   • Integration test with three-failure tool that succeeds on 3rd attempt.

### 6.3 Front-end – Execution History & Inspection

1. **Run Sidebar**
   ✅ Implemented Aug-2025 – *Execution Sidebar*:  
   • 🕒 toolbar button toggles drawer.  
   • Fetches `/api/workflow-executions/history/{workflow_id}?limit=20`.  
   • Lists status pill + id; click selects (replay TBD).  
   • Real-time refresh manual trigger; auto-refresh backlog planned.

2. **Diff Viewer** (stretch)
   • Visual diff between two executions: highlights nodes with differing
     outputs or statuses.

3. **API Wiring**
   • `GET /api/workflows/{id}/executions?limit=20`  
   • `GET /api/workflow_executions/{id}` for detailed state.

### 6.4 Front-end – Node Config Polishing *(COMPLETED Jan 2025)*

1. **Trigger Modal**
   • Form schema auto-generated from backend JSON schema fetched via
     `/api/triggers/schemas`.  
   • Live validation errors, “Test Trigger” button that fires a dummy event.

2. **Tool Modal**
   • Parameter mapping UI: dropdowns populated by upstream node outputs.  
   • Autocomplete for JSONPath expressions (`$.body.subject`).

3. **Common**
   • Internationalization stub (extract all hard-coded strings).

### 6.5 Performance – Large Workflow Optimisations

1. **Front-end**
   • Virtualised canvas layer: only render nodes inside viewport ±margin.  
   • Debounce WS `NodeState` updates – coalesce updates arriving <16 ms apart.

2. **Back-end**
   • Chunk `node_log` frames >2 kB into 512 B pieces to avoid websocket
     congestion.

### 6.6 Collaboration *(post-v1 but specced now)*

1. **Presence Service** – simple Redis pub/sub broadcasting `{workflow_id,
   user_id, cursor}` packets.
2. **Front-end cursors** – coloured ghost cursors with user initials.

---

## 7. Open Questions / Parking Lot

• **Node Versioning** – do we snapshot node code at execution start or always
  run latest?  Affects reproducibility.
• **Secrets Management** – workflow may reference secrets; plan for masked UI
  fields and secure storage.
• **Cost Tracking** – attribute OpenAI / tool costs per node; backend already
  stores `metrics` JSON but UI is TBD.

