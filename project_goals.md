 Below is a “state‑of‑the‑union” for the repo followed by an opinionated, step‑by‑step roadmap.  I read all of backend/zerg and the Rust/WASM frontend so the comments are concrete, not hand‑wavy.

    ────────────────────────────────────────

        1. Where the codebase is TODAY
           ────────────────────────────────────────
           Backend (FastAPI + SQLAlchemy)
           • Stable CRUD for Agents, Threads, Messages.
           • LangGraph‑based Agent runtime with streaming ChatCompletion → EventBus → WebSocket.
           • APScheduler service exists and correctly runs cron‑style schedules (tests cover it).
           • Topic‑based WS hub is solid; decorators publish DB events automatically.
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
    • No concept of “Trigger” (external event that fires an agent).
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
    Backend
      – POST /api/triggers/{trigger_id}/events that publishes EventType.TRIGGER_FIRED with JSON payload.
      – SchedulerService subscribes and enqueues run_agent_task(trigger.agent_id, payload).
    DB
      – new table Trigger (id, agent_id, type='webhook', secret, created_at…).
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
    6.  Immediate next steps (actionable)
    ────────────────────────────────────────

        1. Backend: expose `next_run` and `last_run` fields in Agent schema & tests. DONE
        2. Frontend: add them to AgentCard component and update.rs message flow.
        3. Decide cron‑UI (simple text‑field vs presets).
        4. Draft Trigger table + router skeleton; write a pytest that POSTs event and asserts Agent executed.

    Let me know which milestone you’d like to dive into first, and I can sketch the detailed technical tasks or start sending PRs.