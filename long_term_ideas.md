# Project Context & Roadmap: Zerg Automation Platform

**Document Purpose:** Provides essential context, current status, roadmap, development guidelines, and future ideas for the Zerg project, intended for both human developers and AI coding assistants.

**Last Updated:** 2025-05-03 *(Remember to update this date regularly!)*

---

## 1. Project Overview & Goals

*   **Core Purpose:** Zerg is an automation platform enabling users to connect event triggers (like webhooks or schedules) to AI agents and deterministic tools using a visual canvas interface, facilitating the creation of complex, event-driven workflows.
*   **Technology Stack:**
    *   **Backend:** Python, FastAPI, SQLAlchemy (async with `asyncpg`), LangGraph, APScheduler
    *   **Frontend:** Rust (compiled to WASM), Yew Framework, WebSockets
    *   **Key Libraries:** Pydantic (V2), `serde`/`serde_wasm_bindgen`, `uv` (for env/pkg management)
*   **High-Level Architecture:**
    *   A central `EventBus` facilitates decoupled communication between services.
    *   A WebSocket Hub (`ws_manager`) pushes real-time updates (DB changes, agent streaming) to connected frontends based on topic subscriptions.
    *   The `AgentService` uses LangGraph to execute agent logic (currently single-node, planned for multi-node DAGs).
    *   The `SchedulerService` manages cron-based job scheduling via APScheduler. *(Note: For future scaling beyond a single process, consider task queues like Celery or RQ - see Sec 5).*

---

## 2. Current System State (Snapshot as of 2025-05-03)

*Key: ✅ = Shipped & Verified | 🟡 = Partially Done | ❌ = Not Started*

**Backend:**
*   Agents / Threads / Messages CRUD: ✅
*   LangGraph Agent Runtime (Single Node): ✅
    *   Streaming ChatCompletion → EventBus → WebSocket: ✅
*   APScheduler Service (Cron Runs, `next/last_run_at` persistence): ✅
*   Topic-based WebSocket Hub (Auto DB event publishing via decorators): ✅
*   Webhook Triggers (DB Table, `POST /api/triggers`, `POST /api/triggers/{id}/events`, `EventType.TRIGGER_FIRED`, Scheduler hook): ✅
*   Webhook Triggers (`DELETE /api/triggers/{id}` endpoint): ❌
*   Webhook Triggers (HMAC Secret Verification on fire event): ❌
*   Workflow DAG Execution Engine (Multi-node LangGraph from Canvas JSON): ❌
*   Testing: >180 tests, covering CRUD, services, events, triggers (create/fire). Green ✅

**Frontend:**
*   Core Views (Dashboard, Chat, Canvas): ✅
*   Networking Layer (`api_client.rs`, `ws_client_v2.rs` - REST CRUD, WS Subscribe/Topics): ✅
*   Scheduling UX (Display `next/last_run` badges, Cron Input, Enable Toggle, PATCH `/agents/{id}`): ✅
*   Dashboard: Agent Table (with run/pause/edit): ✅
*   Dashboard: Agent Details Drawer (Overview Tab, Raw JSON Tab): ✅
*   Dashboard: List Agent Triggers (Read-only, Copy URL button): 🟡 (Create/Delete UI missing)
*   Chat: Real-time conversation view per agent/thread: ✅
*   Chat: Surface Threads from *all* runs (manual, scheduled, trigger-fired): 🟡 (Data exists; UI currently filters/hides some)
*   Canvas: Node Editor foundational component: ✅
*   Canvas: Only "Agent" nodes currently placeable/editable: ✅
*   Canvas: Add/Connect other node types (Webhook Trigger, Tool, Condition, Input/Output): ❌
*   Canvas: Export workflow definition to JSON: ❌
*   Canvas: Runtime Execution Highlighting (visualizing active node): ❌
*   Canvas: Specific UI for "Webhook Trigger" node type (displaying URL/secret): ❌

**Cross-Cutting / Ops:**
*   Auth & Multi-tenancy (JWT, `workspace_id`, Login): ❌
*   Usage Metering / Cost Tracking (Tokens, Duration): ❌
*   Observability (Per-Run Metrics, Grafana/Admin UI): ❌

---

## 3. Roadmap & Milestones

**(Overall Goal Reminder):** To enable users to visually build, execute, and monitor event-driven workflows combining AI agents and tools.

**Milestone Definitions:**

*   **M0: “Low‑hanging UX polish”** (Status: ✅ Completed ~2025-04-21)
    *   *Summary:* Implemented frontend UI for viewing and editing agent schedules (cron expression, enable/disable toggle). Wired up PATCH API. Ensured `next_run_at`/`last_run_at` display correctly. Ensured all runs create a persistent `Thread`.
*   **M1: “Triggers v1 – Inbound WebHooks”** (Status: 🟡 In Progress)
    *   *Summary:* Allow external systems (Zapier, GitHub Actions, etc.) to trigger agent runs via unique webhook URLs.
    *   *Status Breakdown:* Backend API (Create/Fire ✅), Frontend UI (List ✅, Create/Delete ❌), Backend Polish (Delete API ❌, HMAC ❌), Canvas Node ❌.
*   **M2: “Workflow Execution Engine”** (Status: ❌ Not Started)
    *   *Summary:* Execute multi-step workflows defined on the Canvas (DAGs of Trigger, Agent, Tool, Condition nodes). Persist Canvas JSON. Basic runtime visualization. *Scalability Note: May require evaluating dedicated task queues (Celery/RQ) if APScheduler proves insufficient for high concurrency.*
*   **M3: “Toolbox Expansion”** (Status: ❌ Not Started)
    *   *Summary:* Ship initial set of deterministic "Tool" nodes (e.g., Slack message, Email send/fetch, GitHub issue, basic HTTP Request). Define Tool node contract (input/output schema, execution guarantees). *Future Enhancement: Consider tool-specific access controls.*
*   **M4: “Multi‑tenant & Auth”** (Status: ❌ Not Started)
    *   *Summary:* Implement user authentication (e.g., Auth0/Clerk JWT) and data isolation (`workspace_id`) to support multiple users securely. *This may include managing permissions for accessing specific tools or agents.*
*   **M5: “Observability & Cost”** (Status: ❌ Not Started)
    *   *Summary:* Track key metrics per agent run (token usage, cost, duration, errors) and expose them via API and/or dashboard analytics panel.

**Prioritization Rationale:**

1.  **Scheduling UX (M0 - Done):** Delivered immediate user value and exercised the core backend->frontend eventing loop.
2.  **Triggers (M1 - Current):** Provides the first real-world integration point, unlocking practical use cases without requiring the full workflow engine.
3.  **Workflow Engine (M2):** Foundational for complex automations, but requires M1 (Triggers) and M3 (Tools) to be truly useful. Build the plumbing only once there's something to plumb.
4.  **Toolbox (M3):** Each tool adds concrete, marketable value. Can be developed incrementally alongside/after M2.
5.  **Auth/Multi-tenancy (M4):** Necessary for onboarding external alpha users or offering a SaaS product. Timing depends on user rollout plan.
6.  **Observability (M5):** Crucial for production use and optimization, but less critical than core execution functionality for early stages.

---

## 4. Current Focus & Action Items (Targeting: M1 Completion)

**Current Goal:** Finish all remaining M1 tasks (Webhook Trigger functionality) and perform initial design for M2 (Workflow Engine).

**Backend Tasks (M1 Polish):**
*   `[ ]` Implement `DELETE /api/triggers/{id}` endpoint allowing users to remove webhook triggers.
*   `[ ]` Add `pytest` tests covering the trigger deletion flow (create, verify exists, delete, verify gone, check permissions).
*   `[ ]` **(Hardening/Optional):** Implement HMAC secret generation on trigger creation and verification logic within the `POST /api/triggers/{id}/events` endpoint. Requires storing the secret hash/salt.
*   `[ ]` Add tests for HMAC verification (valid signature, invalid signature, missing signature header).

**Frontend Tasks (M1 Features & Polish):**
*   `[ ]` **Dashboard:** Add a "Create New Trigger" button within the Agent Details drawer (likely near the existing trigger list). This should likely open a small modal or inline form.
*   `[ ]` **Dashboard:** Add a "Delete" icon/button next to each trigger listed in the Agent Details drawer. Add confirmation dialog.
*   `[ ]` **Dashboard:** Wire up the necessary `api_client.rs` calls (`create_trigger`, `delete_trigger`) and corresponding `Message`/`Command` flows in `update.rs` for the Create/Delete UI elements. Refresh trigger list on success.
*   `[ ]` **Canvas:** Add a new node type "Webhook Trigger" to the node palette in `canvas_editor.rs`.
*   `[ ]` **Canvas:** When a "Webhook Trigger" node is selected, its side-panel should display the associated (read-only) webhook URL and potentially the secret (with copy-to-clipboard buttons). If a trigger hasn't been created for this node yet, prompt to create one.
*   `[ ]` **Canvas:** Ensure the unique ID of the associated Trigger DB row is stored as part of the Webhook Trigger node's configuration in the (future) Canvas JSON export structure (preparation for M2).
*   `[ ]` **Chat/Dashboard:** Modify data fetching or filtering logic to ensure *all* `Thread` rows associated with an agent (regardless of how the run was initiated - manual, schedule, trigger) are potentially visible in the Chat view's thread list or a dedicated run history view.

**QA & Documentation:**
*   `[ ]` Refine/add end-to-end `pytest`: Create Agent -> Use API to Create Trigger -> `POST` valid data to trigger URL -> Verify `AgentRun` record created / `Thread` updated / `last_run_at` timestamp updated / expected `Message` appears. Test invalid trigger post (e.g., wrong ID, bad secret if HMAC enabled).
*   `[ ]` Update `README.md` or create `docs/triggers.md` explaining how to create, manage, and use Webhook Triggers (including `curl` examples).

**Planning for M2 (Workflow Engine):**
*   `[ ]` **Schema Design:** Draft the initial Pydantic models (backend) and corresponding Rust structs (frontend) for representing the Canvas workflow DAG as JSON (e.g., `nodes: List[Union[TriggerNode, AgentNode, ToolNode...]]`, `edges: List[Edge]`). Consider versioning. Store in `Agent.workflow_definition` column (needs migration).
*   `[ ]` **Execution Interface:** Outline a common async `execute(context: WorkflowContext) -> WorkflowContext` method signature or interface that each node type (Agent, Tool, Condition) will need to implement in the backend. Define the structure of `WorkflowContext` (carrying state between nodes).

---

## 5. Development Guide

**Key Code Locations:**
*   **Backend:**
    *   Models: `backend/zerg/app/models/models.py`
    *   Services: `backend/zerg/app/services/` (esp. `scheduler_service.py`, `agent_service.py`)
    *   Events: `backend/zerg/app/events/`
    *   API Routers: `backend/zerg/app/routers/` (esp. `agents.py`, `triggers.py`)
    *   Main App: `backend/zerg/main.py`
    *   Tests: `backend/tests/`
*   **Frontend:**
    *   Networking: `frontend/src/network/` (`api_client.rs`, `ws_client_v2.rs`, `topic_manager.rs`)
    *   Components: `frontend/src/components/` (`dashboard/`, `chat/`, `canvas_editor.rs`)
    *   State Management: `frontend/src/` (`update.rs`, `messages.rs`, `state.rs`)
    *   Main App: `frontend/src/app.rs`
*   **Documentation:**
    *   `README.md` (Project Overview)
    *   `DATABASE.md` (DB Schema Details)
    *   This document (`project_context.md` or similar)

**Environment Setup:**
*   **Dependencies:** Uses `uv` for Python environment and package management. Ensure `uv` is installed.
*   **Environment Variables:**
    *   `DATABASE_URL`: Connection string for PostgreSQL (e.g., `postgresql+asyncpg://user:pass@host:port/db`) or SQLite (e.g., `sqlite+aiosqlite:///./zerg_dev.db`)
    *   `OPENAI_API_KEY`: Required for agent runs. Can be a dummy value like `sk-dummy` if only testing non-LLM parts or using mocks.
*   **Running Backend:**
    ```bash
    cd backend
    uv sync # Installs/updates dependencies from pyproject.toml
    uv run python -m uvicorn zerg.main:app --reload --port 8001
    ```
*   **Running Frontend:**
    ```bash
    cd frontend
    ./build-debug.sh # Runs trunk build and serves on http://localhost:8002
    ```
*   **Running Tests:**
    ```bash
    cd backend
    uv run pytest tests
    ```

**Core Conventions & Patterns:**
*   **Database:** Uses SQLAlchemy 2.0 async (`asyncio`, `asyncpg`/`aiosqlite`). Session management via `default_session_factory`. Tests override with an in-memory SQLite DB. `expire_on_commit=False` is set on the sessionmaker; be mindful that objects are not automatically refreshed after commit – use `session.refresh(obj)` if needed.
*   **Backend Events:** The `EventBus` (`zerg/app/events/bus.py`) is central. Use `await event_bus.publish(Event(type=EventType.XYZ, data={...}))`. Callbacks registered via `event_bus.subscribe()` are async. REST routes that mutate state should use the `@publish_event()` decorator to automatically broadcast changes over WebSockets.
*   **Frontend State:** Follows the Elm Architecture: User interaction -> `Message` enum -> `update(msg, state)` function -> updates `AppState` -> potentially returns `Command` enum -> Command executed (e.g., API call) -> Result yields new `Message` -> cycle repeats. Never mutate `AppState` directly outside `update.rs`. `TopicManager` handles incoming WebSocket messages and translates them into `Message`s.
*   **API Schemas:** Use Pydantic V2 `BaseModel` for defining API request/response bodies and for data validation.
*   **Workflow Definition:** Canvas graph structure will be stored as JSON in a dedicated `Agent.workflow_definition` column (planned). Schema definitions should exist in both Python (Pydantic) and Rust (Serde) for consistency, ideally generated or kept closely in sync. Plan for schema versioning early.
*   **Immutability:** Treat `AgentRun` and `Message` records as immutable logs of past activity. Derive analytics or summaries offline/on-demand rather than modifying historical records.
*   **Rust/WASM:** Use `serde` for serialization and `serde_wasm_bindgen` to pass complex types between Rust frontend and JS/backend (via JSON). Frontend node types for the Canvas should map clearly to backend execution logic, likely using enums in Rust.
*   **Task Execution & Scaling:** Currently uses APScheduler for cron jobs and direct execution for triggered/manual runs. For high-volume or long-running tasks in the future (esp. post-M2), consider integrating a distributed task queue like [Celery](https://docs.celeryproject.org/) or [Redis Queue (RQ)](https://python-rq.org/) with dedicated worker processes.

**Implementation Hints:**
*   Keep the `EventBus` central for decoupling. New features (like Triggers) should publish relevant events. Services (like `SchedulerService`) listen for events they care about.
*   Use Pydantic models rigorously on the backend for validating the structure of the incoming Canvas JSON before saving/executing.
*   In the Rust Canvas editor, represent each distinct node type (Agent, Tool, Trigger, Condition) as a variant of a Rust `enum`. This makes pattern matching and serialization clean.
*   When implementing Tools (M3), consider simple, common examples first like sending email (`smtplib`), making HTTP requests (`httpx`), or posting to Slack.

---

## 6. Terminology

*   **Thread:** A chronological log of messages associated with a specific context or conversation. A thread always exists, even for scheduled or triggered runs (though it might be initially empty). It's the primary input/output context for an Agent.
*   **Run:** A single, discrete execution instance of an Agent or a Workflow, operating against a specific Thread. Can be triggered manually, by schedule, or by an external event (webhook).
*   **Trigger node:** A node type on the Canvas that initiates a workflow run based on an external event (e.g., incoming Webhook, new email matching criteria, Kafka message, cron schedule). It produces an initial data payload for the workflow.
*   **Agent node:** A node type on the Canvas that typically consumes context from the associated Thread, interacts with a Language Model (LLM) based on its configuration and input, and produces new messages back to the Thread. Streaming output originates here.
*   **Tool node:** A node type on the Canvas representing a deterministic function or API call (e.g., send a Slack message, create a Jira ticket, fetch data from a database, read an email via IMAP, simple HTTP GET/POST). Tools generally do not call LLMs directly, should execute quickly (<10s), and return structured data (usually JSON).
*   **Condition node:** A node type on the Canvas that allows branching in the workflow based on evaluating data from the preceding node (e.g., using a simple JS/Python predicate: `input.value > 10`).
*   **Edge:** A connection on the Canvas representing the flow of data and/or control between two nodes.

---

## 7. Future Considerations & Idea Backlog

*(This section captures valuable ideas beyond the current M0-M5 roadmap for potential future implementation.)*

*   **Enhanced Output Presentation:**
    *   **Structured Output Panels:** Allow nodes (Agents, Tools) to output structured data (e.g., JSON, potentially simple HTML) and render it appropriately in the UI (e.g., interactive tables, formatted lists, simple charts) instead of just raw text.
    *   **"Result Panel" Concept:** Visually distinguish the final, summarized result of a workflow run from the detailed, step-by-step execution log (Thread Log) in the UI.
*   **Data Handling & Usability:**
    *   **Export Features:** Allow users to export Thread logs or final results (e.g., as JSON, CSV, plain text).
    *   **Automatic Summaries:** Add an optional feature (perhaps a dedicated "Summarize" tool node or button) to generate a concise summary of a long thread using an LLM call.
*   **Tooling & Orchestration:**
    *   **Notifications:** Implement a dedicated "Notification" tool (or use Email/Slack tools) to alert users on workflow completion, failure, or specific events.
    *   **Multi-Agent Orchestration:** Explore enabling one agent/workflow to trigger another, allowing for complex chaining (requires careful design regarding context passing, error handling, and cycle detection).
*   **Advanced Configuration:**
    *   **Fine-Tuning Support:** Allow users to specify custom fine-tuned model identifiers for Agent nodes (if using platforms like OpenAI that support this).
*   **UI/UX Polish:**
    *   Add tooltips for icons and controls.
    *   Implement smoother animations or transitions in the Canvas editor.
    *   Improve responsive design for different screen sizes.

---

## 8. Status History / Log

*(Older status updates and detailed completion notes are archived here)*

**2025-04-21 - M0 Completed / M1 Started**
*   *(Details of work completed for M0 - Scheduling UX & Thread Persistence - remain the same as in the previous version)*
    *   **Frontend Models:** `ApiAgent` structs updated...
    *   **Frontend UI (Display):** Dashboard cards show next/last run...
    *   **Frontend UI (Edit):** Agent modal / Canvas panel have cron input & toggle...
    *   **Frontend State & Actions:** `Message`/`Command`/`update.rs` logic added...
    *   **Frontend Real-time Updates:** `TopicManager` merges `AGENT_UPDATED` events...
    *   **Backend:** `AgentService.run_agent_task` ensures Thread creation...
    *   **Testing:** Relevant frontend/backend tests added/updated...

**2025-05-02 - Status Review Snapshot**
*   *(Content of original Section 7 "Status review – 02 May 2025" was integrated into Section 2 "Current System State" above)*