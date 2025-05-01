# ✨ Feature Request: “Run History” Accordion on Dashboard

    Bring real insight to each Agent by surfacing a concise, real-time history of past executions directly inside the Dashboard accordion.

-----------
---

## 1. Problem / Motivation

At the moment, expanding an Agent row on the Dashboard only shows the last error message.
Yet every manual, scheduled, or triggered execution is already persisted as a new Thread.
Using Threads alone feels wrong for three reasons:

    1. They lack lifecycle & metric fields (status, duration, token count, cost, trigger source).
    2. Future retries or overlapping executions would require multiple runs pointing to the same Thread.
    3. Analytics queries (fail-rate, avg duration) are clumsy when embedded in the chat data-model.

Hence we want a lightweight AgentRun entity that references the Thread but captures execution-level telemetry.

-----------
---

## 2. Goals & Non-Goals

### Goals

    1. Show **N (=20) most-recent runs** when a Dashboard row is expanded.
    2. Each row displays:
       • status icon ▶/✔/✖
       • started_at (localised tooltip)
       • duration (live ticker for running)
       • tokens (grey until finished)
       • cost (grey until finished)
       • trigger source (Manual / Schedule / API)
       • kebab-menu with “View details”, “Retry”, “Stop”.
    3. Realtime updates via existing WebSocket topic `agent:{id}`.
    4. Preserve current UI behaviour (accordion expand/collapse, error display if needed).
    5. Zero-breaking change to the chat experience.

### Non-Goals (phase-1)

    * No pagination / filtering UI.
    * No CSV export.
    * No cost calculation for non-OpenAI models.
    * No “Run details” deep dive (stubbed link only).

-----------
---

## 3. Deliverables

┌───────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
│ Area              │ Deliverable                                                                                                             │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Data-model        │ backend/zerg/models/models.py → new AgentRun table & SQLAlchemy relationship                                            │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ API               │ GET /api/agents/{id}/runs?limit=20  GET /api/runs/{run_id}                                                              │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Events            │ RUN_CREATED, RUN_UPDATED added to EventType <br>TopicConnectionManager pushes {type:'run_update', data:…} on agent:{id} │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Runner            │ execute_agent_task() / AgentRunner create & update AgentRun rows                                                        │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Frontend state    │ ApiAgentRun struct; agent_runs: HashMap<u32, Vec<ApiAgentRun>>                                                          │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Frontend messages │ LoadAgentRuns, ReceiveAgentRuns, ReceiveRunUpdate + matching Commands                                                   │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ UI                │ Rewrite create_agent_detail_row() → mini-table of runs                                                                  │
├───────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
│ Tests             │ • backend CRUD & endpoint tests<br>• WebSocket run_update integration test<br>• wasm-bindgen state-update unit test     │
└───────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

-----------
---

## 4. Implementation Checklist  ✅/🔲

    Copy into the PR description and tick as we go.

### 4.1 Backend

    * [x]  Add `AgentRun` SQLAlchemy model
      `id, agent_id, thread_id, status, trigger, started_at, finished_at, duration_ms, total_tokens, total_cost_usd, error`
    * [x]  Extend `schemas/schemas.py` with `RunStatus`, `RunTrigger`, `AgentRunOut`
    * [x]  CRUD helpers (`create_run`, `mark_running`, `mark_finished`, `mark_failed`, `list_runs`)
    * [x]  Add `RUN_CREATED`, `RUN_UPDATED` to `events/event_bus.py`
    * [~]  Hook run lifecycle in `task_runner.execute_agent_task()` (**done**) and `AgentRunner` (**pending**)
    * [x]  REST routes (`routers/runs.py`) + mount under `/api`
    * [ ]  Include `runs` in `AgentDetails` when `include=runs`
    * [x]  Broadcast run_update via `websocket/manager.TopicConnectionManager`
    * [ ]  Unit / integration tests (`tests/test_runs.py`)

### 4.2 Frontend

    * [ ]  `models.rs` – `ApiAgentRun`
    * [ ]  `messages.rs`
        * [ ]  new Messages

        * [ ]  new Commands
    * [ ]  `command_executors.rs` – handle `FetchAgentRuns`
    * [ ]  `network/api_client.rs` – `get_agent_runs()`
    * [ ]  `network/ws_handlers` – route `"run_update"`
    * [ ]  `state.rs` – add `agent_runs`
    * [ ]  `update.rs` – reducers for new messages
    * [ ]  `components/dashboard/mod.rs`
        * [ ]  dispatch `LoadAgentRuns` on expand

        * [ ]  replace details row with run-table

        * [ ]  live prepend on `ReceiveRunUpdate`
    * [ ]  CSS tweaks (`frontend/www/styles.css`)
    * [ ]  wasm-bindgen tests

### 4.3 Docs / Ops

    * [ ]  `README.md` → update “Key Features”, API docs
    * [ ]  CHANGLELOG entry
    * [ ]  (optional) Alembic migration script for prod DBs

-----------
---

## Progress Notes – 2025-05-01

### Implemented in Backend

1. **Data-model:** `AgentRun` table with relationships (`agent` & `thread`).
2. **Schemas:** `RunStatus`, `RunTrigger`, `AgentRunOut`.
3. **CRUD:** Helpers for run lifecycle + listing.
4. **Events:** `RUN_CREATED` / `RUN_UPDATED` – published from TaskRunner.
5. **WebSocket:** Topic manager now forwards `run_update` messages on `agent:{id}`.
6. **TaskRunner:** creates run row, updates status & duration, emits events.
7. **REST:** `GET /agents/{id}/runs?limit=n` and `GET /runs/{run_id}` mounted under `/api`.

### Key Context / Decisions

• Runtime is Python 3.9 – avoided pep604 union (`|`) in *function signatures*; use `Optional`.
• Only `task_runner.execute_agent_task()` currently tracks runs; chat `/threads/{id}/run` path still TODO.
• Token / cost columns left nullable until usage accounting lands.

### Next up

Backend:
• Add run tracking for chat `AgentRunner` code path.
• Return runs from `AgentDetails` when requested.
• Unit + integration tests.

Frontend:
• All checklist items still open (state, commands, UI, ws handler, CSS).

Docs / Ops:
• README & CHANGELOG updates; Alembic migration script for production.


-----------
---

## 5. File Guide & Hotspots

┌─────────────────────────────────────────────────────────┬───────────────────────────────────────────────────────────────────────────────────────────┐
│ Path                                                    │ Why it matters                                                                            │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ backend/zerg/services/task_runner.py                    │ Single entry for every run – easiest place to create_run() + mark_finished/failed()       │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ backend/zerg/managers/agent_runner.py                   │ When running from chat /threads/{id}/run we still need run tracking; same pattern applies │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ frontend/src/components/dashboard/mod.rs                │ Current accordion logic; replace create_agent_detail_row()                                │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ frontend/src/command_executors.rs                       │ All network calls live here; follow existing fetch pattern                                │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ frontend/src/network/ws_client_v2.rs + topic_manager.rs │ Already dispatch other event types; add handler for "run_update"                          │
├─────────────────────────────────────────────────────────┼───────────────────────────────────────────────────────────────────────────────────────────┤
│ frontend/src/update.rs                                  │ Central reducer; add handlers without borrowing conflicts                                 │
└─────────────────────────────────────────────────────────┴───────────────────────────────────────────────────────────────────────────────────────────┘

-----------
---

## 6. Data-flow After Feature

    1. User clicks ▶ or scheduler fires.
    2. `execute_agent_task()` → `AgentRun` row (status = queued → running).
    3. EventBus fires RUN_CREATED/RUN_UPDATED → WebSocket `agent:{id}` → browser dispatches `ReceiveRunUpdate`.
    4. Dashboard row is already expanded? It live-prepends run.
    5. Not expanded? When user expands later, UI dispatches `LoadAgentRuns` → REST list endpoint → `ReceiveAgentRuns`.

-----------
---

## 7. Risks & Mitigations

    1. **DB migration** – SQLite in dev auto-creates, but prod needs Alembic; ship stub migration.
    2. **Performance** – New insert per run is negligible; index `agent_id DESC, started_at DESC`.
    3. **WS fan-out** – Extra messages minimal (start + finish per run).
    4. **Frontend borrow panics** – Follow existing `Command::UpdateUI` pattern to avoid simultaneous borrows.

-----------
---

## 8. Future Enhancements

• Pagination & filters (“Last 24 h”, “Failed only”).
• Full Run-details modal (token/cost graph, tool call timelines).
• Retry / Stop actions (hook agent_runner.cancel_run() in future).
• Notify Slack on failed runs (integration plugin).

-----------
---

### Appendix A – Status & Trigger Enums

    class RunStatus(str, Enum):
        queued = "queued"
        running = "running"
        success = "success"
        failed = "failed"

    class RunTrigger(str, Enum):
        manual = "manual"
        schedule = "schedule"
        api = "api"