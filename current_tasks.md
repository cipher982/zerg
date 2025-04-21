 Below is a “state‑of‑the‑union” for the repo followed by an opinionated, step‑by‑step roadmap.  I read all of backend/zerg and the Rust/WASM frontend so the comments are concrete, not hand‑wavy.

    ────────────────────────────────────────

        1. Where the codebase is TODAY
           ────────────────────────────────────────
           Backend (FastAPI + SQLAlchemy)
           • Stable CRUD for Agents, Threads, Messages.
           • LangGraph‑based Agent runtime with streaming ChatCompletion → EventBus → WebSocket.
           • APScheduler service exists and correctly runs cron‑style schedules (tests cover it).
           • Topic‑based WS hub is solid; decorators publish DB events automatically.
           • **Webhook Triggers implemented:** Trigger table, `/api/triggers` router, `EventType.TRIGGER_FIRED`, SchedulerService hook & passing tests.
           • Tests are extensive (>180) and green.

    Frontend (Rust / Yew / WASM)
    • Three views that mount from the same global AppState:

        1. Dashboard – table of agents with “Run / Pause / Edit”.
        2. Chat – real‑time conversation per agent/thread.
        3. Canvas – node editor (agents are nodes; edges are “flow”).
           • Networking layer (api_client.rs + ws_client_v2.rs) already supports:
             – REST CRUD, WS subscribe, topic routing.
           • Scheduling UI is half–wired: the Agent modal shows cron fields but they are not surfaced on Dashboard cards nor editable from Canvas.

    What’s NOT in place
    • Webhook Triggers exist server‑side but **UI still lacks** trigger management/visualisation.
    • No persistent history when an agent is executed from Dashboard (a temp thread is created but never shown).
    • Canvas nodes = agents only; no first‑class “Tool”, “Input/Output”, “Condition”, etc.
    • Multistep workflows (a chain of nodes) are not executed—Canvas is only an editor.
    • Auth, multi‑tenant, usage metering, cost tracking all missing.

    ────────────────────────────────────────
    2.  Terminology we should settle on
    ────────────────────────────────────────
    Thread        = chronological log of messages (always exists, even for scheduled runs).
    Run           = one execution of an agent (against a thread).
    Trigger node  = produces an event (webhook, cron, Kafka, email, Slack, Docker‑alert…).
    Agent node    = consumes thread context, calls LLM, yields messages.
    Tool node     = deterministic function (send Slack msg, create Jira ticket, read email IMAP).
    Condition     = tiny JS/Python predicate to branch.
    Edge          = data/control flow from node → node.

    ────────────────────────────────────────
    3.  Road‑map (six incremental milestones)
    ────────────────────────────────────────
    M0  “Low‑hanging UX polish”  (1–2 weeks)
    • Finish scheduling in UI
      – Dashboard card shows next‑run / last‑run.
      – In Agent modal & Canvas side‑panel allow cron expr + “Enable schedule” toggle.
      – Wire REST PATCH /agents/{id} to update schedule.
    • ALWAYS create a Thread row for any run (manual or cron) and surface it in Chat view (read‑only).
      Rationale: uniform data model simplifies analytics & debugging.

    M1  “Triggers v1 – Inbound WebHooks”  (1 week)
    Backend  ✅  (delivered)
      – Trigger DB table, router & event flow shipped; tests green.
    DB       ✅
    Frontend
      – Canvas gets “Webhook Trigger” node (auto‑generates URL + secret).
      – Dashboard > Agent details lists its triggers with copy‑URL button.
    Value  – real users can wire Zapier, GitHub, Terraform Cloud, etc.

    M2  “Workflow Execution Engine”  (3–4 weeks)
    Backend
      – Replace current LangGraph single‑node graph with multi‑node DAG built from Canvas JSON:
          Trigger → (zero⁺ Tool | Agent | Condition)* → (Action Tool)
      – Persist Canvas JSON per agent (already partly there in Agent.config).
      – Each node type implements an async execute(context) -> context.
      – Streaming still only originates from Agent nodes.
    Frontend
      – Canvas editor must let users add the new node types and connect edges; export to JSON schema.
      – Minimal run‑time visualisation: highlight node that’s currently executing.
    Tests
      – Add end‑to‑end test that fires webhook and sees Slack‑mock message.

    M3  “Toolbox Expansion”  (ongoing)
    Ship small deterministic helpers as separate nodes/packages:
      • Slack send_message, Slack fetch_channel_history
      • Email (IMAP fetch, SMTP send)
      • GitHub create_issue / comment
      • DockerHub alert listener (maps to Trigger v1)
    Guideline: Tools never call LLM, must finish <10 s, return JSON.

    M4  “Multi‑tenant & Auth”  (2–3 weeks)
    • Auth0 / Clerk / your‑choice JWT guard on all routes.
    • DB “workspace_id” column on Agent/Thread/Message/Trigger.
    • Frontend login screen; WS handshake passes token.
    This unlocks inviting alpha users safely.

    M5  “Observability & Cost”  (nice‑to‑have)
    • per‑Run metrics table: token_in/out, OpenAI cost, duration, errors.
    • Simple Grafana or /admin/metrics JSON.
    • Surface usage graphs on Dashboard.

    ────────────────────────────────────────
    4.  Suggested priorities (why this order)
    ────────────────────────────────────────

        1. Finish scheduling UX because it shows immediate progress and exercises EventBus → WS → UI loop.
        2. Triggers (webhooks) deliver the first “real‑world” integration without Canvas changes.
        3. Only after 1 & 2 is solid, invest in full workflow engine—otherwise it’s plumbing nobody yet uses.
        4. Expand toolbox continuously; each new integration is user‑visible value and good marketing.
        5. Auth once you need external testers; earlier if security is mandatory.

    ────────────────────────────────────────
    5.  Implementation hints
    ────────────────────────────────────────
    • Keep the EventBus central—Triggers just publish new events, SchedulerService already listens.
    • Store Canvas JSON as a dedicated Agent.workflow column; version it for migrations.
    • Use Pydantic v2 BaseModel for node schemas and graph validation.
    • In Rust Canvas editor, each new node type is an enum variant; serde_wasm_bindgen already in tree can serialise directly to the backend schema.
    • Treat every Agent run as immutable; derive analytics offline rather than mutating rows.

    ────────────────────────────────────────
    6.  Immediate next steps (actionable)  — April 2025 status
    ────────────────────────────────────────

        1. Frontend: show `next_run/last_run` on Dashboard & Agent modal; allow editing `schedule` & `run_on_schedule` (PATCH /agents/{id}).
        2. Frontend: Cron‑UI (simple text field for now) + validation.
        3. Frontend: list existing Triggers (read‑only) with copy‑URL button; later add creation flow in Canvas.
        4. (Backend follow‑up): optional security hardening for trigger secrets & HMAC signing.

    Let me know which milestone you’d like to dive into first, and I can sketch the detailed technical tasks or start sending PRs.


----
NOTES FROM ONGOING WORK


    0. TL;DR
    Backend = FastAPI + SQLAlchemy + LangGraph, WS via EventBus.
    Frontend = Rust/Yew WASM SPA (Dashboard, Chat, Canvas).
    You are picking up after milestone 0 (schedule metadata shipped).

    1. Skeleton you should open first
    backend/
      • zerg/app/models/models.py  – DB schema (Agents, Threads, Messages).
      • zerg/app/services/scheduler_service.py  – APScheduler + event listeners.
      • zerg/app/events/  – tiny pub/sub bus.
      • zerg/app/routers/agents.py  – CRUD + run‑task endpoint.
      • tests/ (good reading; >180 tests show expected behaviour).

    frontend/
      • src/network/ws_client_v2.rs + topic_manager.rs – WS client.
      • src/components/dashboard/ – Agent cards UI.
      • src/components/canvas_editor.rs – node editor entry point.
      • src/update.rs & messages.rs – Elm‑style state update.

    docs & road‑map
      • README.md – full project overview.
      • project_goals.md – agreed roadmap (M0‑M5).
      • DATABASE.md – table cheat‑sheet.

    2. Current backend state (after M0)
    • Agent now has next_run_at / last_run_at.
    • scheduler_service persists those on schedule & after each run.
    • Session maker uses expire_on_commit=False to avoid detached errors in tests.
    • All tests pass via: cd backend && uv run pytest tests.

    3. Immediate tasks in backlog (see project_goals.md)
    Frontend
      ‑ show next/last run on Dashboard card & Agent modal.
      ‑ add cron editing UI and PATCH /agents/{id} call (schedule + run_on_schedule).

    Backend
      ‑ Trigger table + router skeleton (M1 start).
      ‑ POST /api/triggers/{id}/events publishes EventType.TRIGGER_FIRED.
      ‑ SchedulerService listens and enqueues run_agent_task.

    Testing
      ‑ new pytest for trigger flow (create trigger row → POST event → assert run called).
      ‑ Add assertions in existing scheduler_service tests for next_run_at/last_run_at.

    4. Gotchas / conventions
    • DB sessions via default_session_factory; tests override with in‑memory session.
    • EventBus callbacks are async; always await publish().
    • Any REST route that mutates state is decorated with @publish_event to fan out WS messages.
    • Frontend never mutates state directly – dispatch Message → update.rs → Command executor.
    • LangGraph integration currently single‑node; you’ll extend to DAG later.
    • Use uv run pytest instead of pip installs – uv handles virtual env + deps.
    • sessionmaker(expire_on_commit=False) means objects stay live after commit—handy but remember to refresh() if you need latest DB values.

    5. Environment
    Set DATABASE_URL (sqlite path ok) and OPENAI_API_KEY (can be dummy during tests).
    Run backend: uv run python -m uvicorn zerg.main:app --reload --port 8001
    Run frontend: ./frontend/build-debug.sh (spawns simple http server on :8002).

    6. Where to start coding

        1. frontend/src/components/dashboard/agent_card.rs (doesn’t exist yet – add).
        2. backend/zerg/app/routers/triggers.py (new).
        3. tests/test_triggers.py (new).

    Ping project_goals.md after each PR so roadmap stays current.

    Welcome aboard & happy hacking!


## BEGIN TASK LIST


    Proposed work‑plan (what I will do)

        1. Front‑end ‑ Surface scheduling metadata  (completes M0‑part‑1)
           a. Extend ApiAgent (+ AppState, serde) with:
              • schedule :String?  • run_on_schedule :bool
              • next_run_at :String?  • last_run_at :String?
           b. Dashboard: new AgentCard fields “Next run / Last run” (friendly utc→local).
           c. Agent‑modal / Canvas side‑panel:
              • Cron expression text‑box.
              • “Enable schedule” toggle.
           d. Message / update.rs additions:
              • SetSchedule(String)  • ToggleRunOnSchedule(bool)
              • Command::SaveAgentSchedule(agent_id, ApiAgentUpdate).
           e. Command executor issues PUT /api/agents/{id}.
           f. TopicManager already receives AGENT_UPDATED → ensure reducer patches next_run_at/last_run_at live.
        2. Front‑end – Cron editing PATCH call (M0‑part‑2)
           • Re‑use agent PUT endpoint; success path closes modal & shows toast.
           • Validation: lightweight “YYYY* * *…” regex; let backend reject invalid cron for now.
        3. Backend – Trigger MVP ✅ (completed in previous PR)
        4. Front‑end – expose triggers (read‑only for now)
           • Dashboard agent‑details panel lists existing webhook URLs with copy‑button.

    Order of execution

        1. Implement & test front‑end model changes + display fields.
        2. Wire edit UI + PUT flow.
        3. Land Trigger DB + router + tests.
        4. Basic read‑only trigger list in UI.

    Deliverables per PR
       • Code + passing pytest & wasm build.
       • Short update in project_goals.md marking M0 complete and M1 started.

    Estimated time
       – Scheduling UX: 1–2 sessions.
       – Trigger MVP: 1 session backend, ½ session tests, ½ session UI list.