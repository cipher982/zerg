# Agent Platform

Agent Platform is a full-stack reference application that lets you **create &
orchestrate AI agents** in real-time.  It couples a FastAPI backend (Python
3.12) with a purely-Rust + WASM front-end, and ships an optional Node +
Playwright pre-rendering layer for SEO.

The project acts as both a production-ready starter kit *and* a playground for
modern WebSocket streaming patterns, LangGraph-based agents, and a
canvas-style visual editor – all under one repo.

NEVER CALL 'python' or 'pytest' directly – use the provided helper scripts OR go through UV.

-------------------------------------------------------------------------------
## Table of Contents

1. [Overview](#overview)  – what it does, at a glance  
2. [Key Features](#key-features)  – why it is interesting  
3. [Quick Start](#quick-start)  – run everything locally in minutes  
4. [Architecture](#architecture)  – BE / FE / Pre-render  
5. [Authentication](#authentication)  – Google Sign-In + JWT + WS  
6. [WebSocket & Event Flow](#websocket--event-flow)  
7. [Frontend State Management](#frontend-state-management)  
8. [Directory Layout](#directory-layout)  
9. [Testing](#testing)  
10. [Runtime Configuration & Feature Flags](#runtime-configuration--feature-flags)  
11. [WebSocket Close Codes](#websocket-close-codes)  
12. [MCP Tool Integration](#mcp-tool-integration)  
13. [Frontend Dev Tips](#frontend-dev-tips)  
14. [Development Environment & Sandboxing](#development-environment--sandboxing)  
15. [Extending / Road-map](#extending--road-map)

-------------------------------------------------------------------------------
## Overview

*   **Dashboard** – spreadsheet-like table of agents with run/pause, stats,
    quick debugging links.  
*   **Canvas Editor** – node-based UI (Rust + WebGL-free renderer) to design
    multi-step or multi-tool workflows visually.  
*   **Chat view** – chat with any agent, token-level streaming, message
    history.

Agents live in a SQL database, can be triggered on-demand or via CRON, and are
implemented as *fully functional* LangGraph runnables.

-------------------------------------------------------------------------------
## Key Features

• **Strict WebSocket authentication (Jul 2025)** – every browser opens
  `wss://…/api/ws?token=<jwt>`.  The backend validates the token *before*
  `accept()`; bad or missing tokens close with code **4401**.  Local-dev keeps
  `AUTH_DISABLED=1` so no token is needed.

• **Token-level AI streaming** – enable `LLM_TOKEN_STREAM=1` and the backend
  emits `assistant_token` chunks for real-time typing effect; still sends full
  `assistant_message` for non-live clients.

• **Elm-style front-end architecture** – global `AppState`, `Message` enum,
  pure `update()` reducer -> predictable, no borrow-checker fights.  A tiny
  `mut_borrow!` macro encapsulates RefCell borrows.

• **Google Sign-In production auth, zero-friction local dev** – toggle via
  `AUTH_DISABLED`.

• **ReAct-style LangGraph agents** – cached runnable compilation, tool calls
  executed concurrently with `asyncio.gather`.

• **SEO snapshotting** – Node + Playwright captures static HTML and serves it
  to crawlers; humans get the live WASM SPA.

-------------------------------------------------------------------------------
## Quick Start

Prereqs: Python 3.12, [uv](https://github.com/astral-sh/uv), Rust (+ wasm-pack),
Node 16+, and GNU Make (installed by default on macOS/Linux; Windows users can
install via `choco install make`).

### 1. One-liner local dev (recommended)

```bash
make dev
# starts backend on :8001 and frontend on :8002
# …open http://localhost:8002 in your browser
```

`make dev` simply wraps the exact commands you may have run manually; it just
avoids the port-clash & “what was that command again?” issues.

### 2. Manual steps (if you prefer)

```bash
# backend
cd backend && cp .env.example .env   # add OPENAI_API_KEY here
uv run python -m uvicorn zerg.main:app --reload --port 8001

# frontend (new terminal tab)
cd frontend && ./build-debug.sh      # dev build + server on :8002

# visit http://localhost:8002
```

### 3. Running tests

```bash
# fast unit / service tests
make test

# full browser end-to-end suite
make e2e
```

Under the hood those targets call the existing helper scripts
(`backend/run_backend_tests.sh`, `frontend/run_frontend_tests.sh`,
`e2e/run_e2e_tests.sh`) so nothing about the individual test runners has
changed – there is now just one canonical entry-point.

-------------------------------------------------------------------------------
## Architecture

### Backend (FastAPI)

*   **Routers** under `backend/zerg/routers/` – CRUD, auth, WebSocket.  
*   **EventBus** – in-process pub/sub; decorators publish automatically.  
*   **TopicConnectionManager** – holds per-socket topic subscriptions and
    broadcasts EventBus packets.  Uses a single `asyncio.Lock` and a 30-s
    heartbeat to drop zombies.

*   **AgentRunner** compiles + runs LangGraph runnables, streams tokens over
    WS, persists messages.

### Front-end (Rust + WASM)

*   `WsClientV2` – reconnects with exp-back-off, appends `?token=…`, shows
    auth-error banner on 4401, has packet ping.
*   `TopicManager` – subscribe/unsubscribe, dispatches messages to closures.
*   `state.rs` – single RefCell holding `AppState`; message reducer pattern.

### Pre-render (optional)

Node + Playwright snapshot → `prerender/dist/index.html`; Express server serves
it to bots based on UA sniffing.

-------------------------------------------------------------------------------
## Authentication

Production: Google Sign-In → backend verifies Google ID-token → issues
30-minute HS256 JWT → stored in `localStorage[zerg_jwt]` and sent:

* as `Authorization: Bearer …` on every REST call, and  
* appended as `?token=…` when the front-end opens the WebSocket.

Local development: leave `AUTH_DISABLED=1` (default).  Backend injects a
deterministic `dev@local` user; WS accepts even without token.

Secrets required for prod:

```bash
GOOGLE_CLIENT_ID="…apps.googleusercontent.com"
JWT_SECRET="change-me"
TRIGGER_SIGNING_SECRET="hex-string"   # only if you use triggers
```

-------------------------------------------------------------------------------
## Runtime Configuration & Feature Flags

The backend loads configuration via `zerg.config.get_settings()` – a tiny
dataclass-based helper that replaces the heavier Pydantic dependency.

Search order (first match wins):

1. Variables already in the process environment (Docker, CI, etc.)
2. `<repo-root>/.env`   (current canonical location)
3. `<repo-root>/backend/.env`   (legacy path kept for backwards-compat)

### Required secrets (production)

| Variable | Purpose |
|----------|---------|
| `OPENAI_API_KEY` | Mandatory for *all* LLM calls – startup aborts if absent (unless `TESTING=1`). |
| `JWT_SECRET` | HS256 signing key for authentication tokens |
| `TRIGGER_SIGNING_SECRET` | HMAC secret validated by `/api/triggers/{id}/events` |

### Feature flags (truthy = `1`, `true`, `yes`, `on`)

| Flag | Behaviour |
|------|-----------|
| `AUTH_DISABLED` | Bypass Google/JWT auth (local dev & unit tests) |
| `LLM_TOKEN_STREAM` | Stream per-token `assistant_token` WS chunks for live typing effect |
| `E2E_LOG_SUPPRESS` | Mute debug logs during Playwright runs |
| `DEV_ADMIN` | Spawn an implicit *admin* user (do **not** use in prod) |

Tips:

* Always read flags via `get_settings()` *inside* functions – tests mutate
  `os.environ` at runtime.
* Import API path constants from `zerg.constants` instead of hard-coding
  strings like `"/api/agents"`.

-------------------------------------------------------------------------------
## WebSocket Close Codes

| Code | Meaning | Suggested client action |
|------|---------|-------------------------|
| **4401** | Auth/JWT invalid | Show login overlay, clear token |
| **4408** | Heart-beat timeout (>30 s without pong) | Reconnect with back-off |

`WsClientV2` responds to pings automatically; external clients must echo a
`pong` within 30 seconds to avoid a 4408 close.

-------------------------------------------------------------------------------
## MCP Tool Integration

Agents can load tools from external **Model-Context-Protocol (MCP)** servers.

Add to the agent’s JSON `config` field:

```jsonc
{
  "mcp_servers": [
    {
      "name": "corp-tools",
      "url":  "https://tools.acme.com",
      "auth_token": "${ACME_TOKEN}",
      "allowed_tools": ["db_*", "send_email"]
    }
  ]
}
```

`AgentRunner` fetches the manifest lazily; per-process cache prevents
duplicate requests. Wild-cards in `allowed_tools` do simple prefix matching.

-------------------------------------------------------------------------------
## Frontend Dev Tips

• Run `./frontend/build-debug.sh` to enable the translucent **debug overlay** –
  every `debug_log!(…)` call appears in-page (mobile friendly).

• Useful macros:
  * `mut_borrow!(cell)` – ergonomic `RefCell` mutable borrow.
  * `css_var!(primary)` – returns `"var(--primary)"` at compile-time.

• Dashboard shortcuts: `Ctrl/⌘+K` search, `N` new agent, `R` run row, arrow
  keys to move selection, `Enter` to expand.

• Toggle token streaming quickly:

```bash
export LLM_TOKEN_STREAM=1
uv run python -m uvicorn zerg.main:app --reload
```

The UI gracefully falls back to message-level streaming when the flag is off.

-------------------------------------------------------------------------------

## Development Environment & Sandboxing

Working inside this repository feels like a normal local-dev setup, **but AI
agents (and human contributors!) must respect a few hard rules to avoid
shooting themselves in the foot**:

1. **Always execute Python through `uv` or the helper scripts.**  The host
   machine ships with system Python 3.9 which *does not* have our
   dependencies compiled for it.  Running `python foo.py` directly will place
   you in that wrong interpreter, the code will error out, and the agent will
   waste cycles trying to “fix” nonexistent compatibility issues.  Correct
   invocations:

   ```bash
   uv run python -m uvicorn zerg.main:app --reload    
   backend/run_backend_tests.sh                      
   make dev # (be careful, this will start the backend/frontend services, and they dont exit)
   cargo check # (for the frontend)
   ```

2. **Do NOT start the backend server inside long-running agent tasks.**  The
   dev server is meant for manual testing; in an automated context it never
   exits and the agent process will hang indefinitely.  Use the test runner
   scripts or targeted `'uv run pytest'` instead.

3. **No ad-hoc fall-backs, shims or version hacks.**  We control the entire
   stack (Python 3.12, Rust 1.76+, Node 18).  If something appears broken,
   assume a genuine bug in the code rather than missing dependencies and fix
   it at the source.

4. **Never call raw `pytest`.**  Use `backend/run_backend_tests.sh` (it
   pre-creates isolated temp DBs, sets env vars and loads plugins).  Same for
   front-end and e2e scripts.

5. **File-system is sandboxed** – the workspace is a git checkout that can be
   rolled back.  Feel free to `apply_patch`, run tests, even nuke the local
   DB files; nothing touches the host system.

Keeping these guard-rails in mind prevents an entire class of misleading error
messages and infinite-loop behaviours when AI agents interact with the repo.

-------------------------------------------------------------------------------

### Avoiding Borrowing Issues

The message-passing architecture prevents Rust borrowing conflicts by:

1. **Single Point of Mutation**: Only the `update()` function should mutably borrow `APP_STATE`
2. **Automatic UI Refresh**: The dispatch function handles UI refreshes after state changes
3. **Clean Separation**: Components only dispatch messages, they don't access state directly

### For New Developers

When extending or modifying the frontend:

1. **Look at Existing Patterns**: See how similar functionality is implemented
2. **Add New Message Types**: Create specific message types for new state changes
3. **Update the Handler**: Add handlers for new messages in the `update()` function
4. **Use `dispatch_global_message`**: Always use the dispatch function for state changes

This architecture makes the codebase more maintainable, prevents bugs related to borrowing, and creates a predictable data flow.

--------------------------------------------------------------------------------
## WebSocket & Event Communication

The application uses a sophisticated event-based architecture for real-time communication:

### Backend Event System

1. **Event Bus Pattern**
   - The backend implements a central `EventBus` that manages event publishing and subscription.
   - Events are typed (e.g., `THREAD_CREATED`, `AGENT_UPDATED`) and carry payload data.
   - Components can subscribe to specific event types without knowing about the publisher.

2. **Publish-Event Decorator**
   - REST endpoints use the `@publish_event(EventType.XYZ)` decorator to broadcast state changes.
   - After an endpoint successfully completes (e.g., creates a thread), the decorator:
     • Extracts data from the function result
     • Publishes an event to the EventBus
     • All subscribers for that event type are notified

3. **WebSocket Integration**
   - The `TopicConnectionManager` subscribes to relevant EventBus events.
   - When an event occurs (e.g., thread created), the manager broadcasts to all WebSocket clients subscribed to that topic.

**Summary:** When a REST endpoint changes state, the decorator publishes an event, the event bus notifies subscribers, and the WebSocket manager pushes updates to all subscribed clients in real time.

### Frontend WebSocket Integration

1. **Topic-Based Subscription Model** (`frontend/src/network/ws_client_v2.rs`)
   - The frontend WebSocket client connects to `/api/ws/v2` and can subscribe to specific topics:
     • `thread:{id}` for thread updates and messages
     • `agent:{id}` for agent status updates

2. **Topic Manager** (`frontend/src/network/topic_manager.rs`)
   - The frontend implements a `TopicManager` that handles:
     • WebSocket connection management
     • Topic subscription
     • Message routing to appropriate handlers

3. **Real-Time UI Updates**
   - When WebSocket messages arrive, they are converted to appropriate Messages:
     • `ReceiveThreadUpdate` for thread metadata changes
     • `ReceiveStreamChunk` for incoming streaming responses – covers three `chunk_type`s:
       `assistant_message` (final message), `tool_output`, and the new `assistant_token` (token-by-token stream)
     • `ThreadMessagesLoaded` for thread history

This architecture creates a seamless real-time experience: when an agent is created, a thread is created, or a message is sent, the UI updates instantly across all connected clients without polling.

--------------------------------------------------------------------------------
## Key Features

• **Two UI Approaches (Dashboard & Canvas)**  
  - The Dashboard is a table-like view of agent cards (showing status, quick actions, logs).  
  - The Canvas Editor (in Rust/WASM) is used for more advanced flows or multi-step instructions.  

• **Strict WebSocket authentication (Jul 2025)** – when `AUTH_DISABLED=0` the
  frontend appends the JWT as `?token=` to `/api/ws`, the backend validates it
  before accepting the handshake and closes with **4401** on failure.  Local
  dev mode remains token-less.  
• **Real-Time AI Streaming (token-level)**  
  - With the `LLM_TOKEN_STREAM` feature flag the backend forwards **each individual token** as it is generated (`chunk_type="assistant_token"`).  
  - Full message chunks (`assistant_message`, `tool_output`) are still emitted so clients without token mode remain compatible.

• **Debug Ring-Buffer Overlay (dev builds)**  
  - Build the frontend with `./build-debug.sh` and every `debug_log!()` call appears in a translucent overlay on the canvas – invaluable when testing on devices without a JS console.

• **ReAct-style Functional Agents**  
  - Pure agent definitions live in `backend/zerg/agents_def/*` and are composed with LangGraph's Functional API.  
  - `AgentRunner` compiles the definition at runtime and handles DB persistence plus WebSocket streaming.
  - Each agent still stores system instructions, thread history and status, and can be triggered manually or on schedule.

• **SEO-Friendly Pre‑Rendering**  
  - A Node + Playwright system captures static HTML snapshots, serving them to web crawlers.  

• **UX polish (June 2025)**  
  - Built-in toast notification system (`toast.rs`) surfaces success / error / info without blocking modals.  
  - Power-user keyboard shortcuts in the Dashboard:  
    • `Ctrl/⌘ + K` focus search  
    • `N` create a new agent  
    • `R` run the focused row  
    • `↑ / ↓ / Enter` full keyboard navigation & expand/collapse rows  
  - Fully responsive Dashboard – below 768 px the table transforms into mobile-friendly cards.  

• **Canvas Editor written in Rust/wgpu‑free 2‑D renderer**  
  - Custom rendering for efficient performance and fluid user interactions.

• **Google Sign-In Authentication (May 2025)**  
  - Production deployments require users to authenticate with their Google account.  
  - The backend issues short-lived HS256 JWTs; the SPA stores them in `localStorage` and attaches `Authorization: Bearer …` to every fetch/WebSocket request.  
  - For local development you can bypass the login overlay by setting `AUTH_DISABLED=1`.

• **HMAC-Secured Webhook Triggers**  
  - `/api/triggers/{id}/events` now validates `X-Zerg-Timestamp` and `X-Zerg-Signature` headers (HMAC-SHA256 using `TRIGGER_SIGNING_SECRET`).  
  - Blocks replay attacks beyond ±5 minutes.

• **Cron-style scheduling via SchedulerService**  
  - ✅ Shipped – APScheduler now triggers `AgentRunner` on the defined CRON schedule.

• **Run History**  
  - Track agent executions with detailed metrics including token counts and costs

--------------------------------------------------------------------------------
## Directory Structure

A simplified overview of notable top-level files and folders:

• backend/  
   ├── zerg/main.py (FastAPI & app bootstrap)  
   ├── zerg/agents_def/ (pure functional agent definitions; ReAct example lives here)  
   ├── zerg/managers/agent_runner.py (orchestration + DB persistence + streaming)  
   ├── zerg/callbacks/token_stream.py (WebSocket token streaming handler)  
   ├── zerg/legacy_agent_manager.py (deprecated – kept for backwards compatibility)  
   ├── run_backend_tests.sh (Test runner script)  
   └── pyproject.toml (Python project config & linting)  

• frontend/  
   ├── Cargo.toml, build.sh, build-debug.sh  
   ├── run_frontend_tests.sh (Test runner script)
   ├── src/  
   │    ├── canvas/           (shapes, rendering logic)  
   │    ├── components/       (dashboard, chat, canvas UI)  
   │    ├── network/         (API client, WebSocket, topics)  
   │    ├── state.rs         (global AppState)  
   │    ├── messages.rs      (frontend events/actions)  
   │    ├── update.rs        (state mutation logic)  
   │    ├── command_executors.rs (side effects)  
   │    └── lib.rs          (WASM entry point)  
   └── www/                 (WASM output & static files)

• prerender/  
   ├── prerender.js, server.js (Playwright & Express)  
   ├── dist/ (Generated static HTML snapshots)  
   └── package.json  

• scripts/
   └── (Various helper scripts)

--------------------------------------------------------------------------------
## Dependencies

1. **Frontend**  
   - Rust (edition 2021) & wasm-pack  
   - wasm-bindgen, web-sys, js-sys, serde, console_error_panic_hook  
   - Python 3 for the local dev server in build.sh (if using the script's local server)  

2. **Backend**  
   - Python 3.12+  
   - FastAPI, uvicorn, websockets, openai, python-dotenv  
   - langgraph, langchain-core, langchain-openai, apscheduler, sqlalchemy
   - uv (for dependency management and running scripts)

3. **Pre-Rendering (Optional)**  
   - Node.js and npm  
   - playwright-chromium, express  

--------------------------------------------------------------------------------
## Setup & Running

### Backend Setup
1. cd backend  
2. Copy the environment file and edit it:  
   » cp .env.example .env  
   (Add your `OPENAI_API_KEY` so the backend can call OpenAI.)  
4. Run the server:  
   » uv run python -m uvicorn zerg.main:app --reload --port 8001 

### Frontend Setup
1. Ensure you have Rust, cargo, and wasm-pack installed.  
2. cd frontend  
3. Build the WASM module:  
   » ./build.sh (production)  
     or  
   » ./build-debug.sh (debug)  
4. A local server at http://localhost:8002 is automatically started by the build script, hosting the UI.

### Pre-rendering Setup (Optional)
1. cd prerender  
2. npm install  
3. npm run prerender   (Captures a static HTML snapshot in dist/)  
4. node server.js      (Serves snapshots on http://localhost:8003 to bots)

--------------------------------------------------------------------------------
## Using the Dashboard & Canvas Editor

After launching the frontend (http://localhost:8002):

1. **Dashboard**  
   - The default tab shows "Agent Dashboard," with existing agents in a table.  
   - Each card shows agent name, status, quick actions (run, pause, edit), logs if available.
   - Live-search (top-right) filters as you type – `ESC` clears the query.  
   - Click any column header to sort; indicator ▲/▼ reflects direction.  
   - Keyboard shortcuts: `N` new agent, `R` run selected row, `↑/↓` move focus, `Enter` expand row.  
   - On small screens (< 768 px) the table auto-converts to a card list for seamless mobile use.  

2. **Canvas Editor**  
   - Switch to "Canvas Editor" from the top tabs.  
   - This view shows a node-based interface for advanced flows.
   - **Phase 1 limitation:** only the following node types are implemented – `AgentIdentity`, `UserInput`, `ResponseOutput`, and `GenericNode`.  Trigger / Tool / Condition nodes are on the roadmap.
   - You can drag nodes around, connect them visually, and edit agent system instructions via the sidebar.

3. **Agent Modal**  
   - In the canvas, clicking on an "Agent Identity" node opens a modal for system instructions, scheduling, advanced settings.

4. **Agent Debug Modal**  
  - From the Dashboard, choose "Details" on any agent to open a read-only modal that surfaces raw JSON returned by `/api/agents/{id}/details` (powered by the new `AgentDetails` schema).  
  - Tabs for *Threads*, *Runs* and *Stats* are stubbed out – they will populate once those include payloads are implemented on the backend.  

--------------------------------------------------------------------------------
## Pre‑Rendering & SEO Details

• Why Pre-rendering: Single-page apps or WASM apps can be difficult for search engines to crawl. By using Node+Playwright, we generate a static snapshot that is served to bot user-agents, boosting SEO.  

• How to Generate Snapshots:  
  1) Ensure your backend (port 8001) and frontend dev server (port 8002) are running.  
  2) cd prerender && npm run prerender.  
  3) The script loads http://localhost:8002 in a headless browser, waits for WASM, then saves dist/index.html.  

• Serving the Snapshot:  
  1) node server.js starts an Express server on http://localhost:8003.  
  2) When a recognized bot user-agent hits "/", it serves the dist/index.html snapshot.  
  3) Regular users see the live WASM app from the frontend.  

• Testing Bot Detection:  
  - The test-crawler.sh script simulates requests from normal and "Googlebot" user agents, saving the results to human.html or googlebot.html for comparison.  

--------------------------------------------------------------------------------
## How It Works

1. **User Action**: A user interacts with the Dashboard or Canvas, e.g., creating or editing an agent.  
2. **Backend**: A FastAPI route handles the request, setting up an OpenAI streaming call for live responses.  
3. **Streaming**: The backend broadcasts tokens over WebSockets to the relevant agent node in the UI.  
4. **Pre‑Rendering**: For bots, server.js in /prerender returns a pre-rendered HTML snapshot; humans get the interactive WASM page.

--------------------------------------------------------------------------------
## Testing & Verification

1. **Basic Functionality**  
   - Start the backend (uvicorn) and frontend (./build.sh).  
   - Create a new agent in the dashboard or canvas, send a prompt, and confirm a streaming response in the UI.  

2. **Logs & Console**  
   - Check the console logs in your browser dev tools, as well as uvicorn logs in your terminal.  
   - For pre-rendering, inspect prerender/dist/index.html and test-crawler.sh results to verify bot detection.  

3. **Running Tests**  
   - Backend: `cd backend && ./run_backend_tests.sh` (spins up in-memory SQLite, no extra services needed)
   - Frontend (wasm-bindgen tests): `cd frontend && ./run_frontend_tests.sh`
   - Current test coverage exceeds 95% for the backend

--------------------------------------------------------------------------------
## Extending the Project & Future Plans

• **Multi-Agent Orchestration**  
  - LangGraph foundation enables agents to chain operations and share state
  - Agents can be composed into larger workflows

• **Advanced Agent Plugins**  
  - Agents bridging to external services, e.g. sending emails or reading logs.  

• **Data Analytics & Usage Metrics**  
  - Track token usage, cost, and latency.  

• **Refinement of Canvas Editor**  
  - Enhanced node library for more complex flows or branching logic.

--------------------------------------------------------------------------------
## Testing & Verification

1. **Basic Functionality**  
   - Start the backend (uvicorn) and frontend (./build.sh).  
   - Create a new agent in the dashboard or canvas, send a prompt, and confirm a streaming response in the UI.  

2. **Logs & Console**  
   - Check the console logs in your browser dev tools, as well as uvicorn logs in your terminal.  
   - For pre-rendering, inspect prerender/dist/index.html and test-crawler.sh results to verify bot detection.  

3. **Running Tests**  
   - Backend: `cd backend && ./run_backend_tests.sh` (spins up in-memory SQLite, no extra services needed)
   - Frontend (wasm-bindgen tests): `cd frontend && ./run_frontend_tests.sh`
   - Current test coverage exceeds 95% for the backend


# Developer Notes - Detailed Architecture Analysis

Below is a practical "big‑picture" walkthrough of the repository.
I read every top‑level source file and the entire test‑suite so that what follows is accurate, but I'm only summarizing – no large blocks of code are reproduced.

## 1. High‑level architecture

* **"backend/"** — Python 3.12 FastAPI service (codename "zerg").
  - Relational persistence through SQLAlchemy → SQLite (app.db)
  - Domain: Agents, Threads, Messages.
  - LangChain + LangGraph used to run LLM workflows for each Agent.
  - Real‑time layers:
    * in‑process EventBus (pub/sub enum EventType)
    * topic‑based WebSocket hub that relays EventBus messages to browsers.
  - APScheduler drives cron‑like runs of Agents.
  - Almost 200 Pytest tests give >95% coverage.

* **"frontend/"** — Rust (edition 2021) + wasm‑bindgen SPA.
  - Compiles to WASM; starter html in www/ mounts the app.
  - Mirrors back‑end models in src/models.rs and keeps global app‑state in src/state.rs.
  - Two big UI surfaces:
    * Canvas editor (graphical editor built on <canvas>)
    * Chat/dashboard (DOM components with split‑pane layout)
  - A single WebSocket client (src/network/ws_client_v2.rs) plus TopicManager
    handles subscribe / publish just like the Python side.
  - Async/await throughout via wasm‑bindgen‑futures.

## 2. Back‑end walkthrough

### Main entry‑point
- backend/zerg/main.py
  * boots FastAPI, installs CORS & OPTIONS middleware.
  * creates tables, mounts routers, starts/stops SchedulerService.

### 2.1 Persistence layer
- backend/zerg/database.py
  - Standard SQLAlchemy engine/session helpers.
- backend/zerg/models/models.py
  - Three tables: Agent, Thread, *Message (two subclasses for agent‑ and thread‑scope but share columns).
  - All models expose JSON columns for flexible config.

### 2.2 CRUD helpers (thin)
- backend/zerg/crud/crud.py
  - Pure SQLAlchemy operations, no business logic.

### 2.3 Event bus
- backend/zerg/events/event_bus.py
  - Very small async pub/sub written from scratch.
  - Decorator publish_event(event_type) lets router functions emit automatically.

### 2.4 Routers
- agents.py – full CRUD, nested /messages sub‑endpoints.
- threads.py – manage conversation threads & messages.
- websocket.py – HTTP handshake that upgrades to WS and delegates to TopicConnectionManager.
- models.py – surfaces list of OpenAI models (one simple GET).

All routers are version‑less but live under prefix "/api".

### 2.5 Agent runtime (new)
- backend/zerg/agents_def/zerg_react_agent.py – holds the **pure functional ReAct agent** built with `langgraph.func`.
- backend/zerg/managers/agent_runner.py – orchestration layer that:
  1. Compiles the runnable from the agent definition (first call is cached).
  2. Persists messages to the DB and marks user messages as processed.
  3. Pushes per-token chunks over WebSocket when the `LLM_TOKEN_STREAM` flag is enabled (thanks to `WsTokenCallback`).
  4. Returns the newly created assistant / tool message rows.

### 2.6 SchedulerService
- backend/zerg/services/scheduler_service.py
  AsyncIOScheduler.
  On startup, loads all agents with a non-null schedule (CRON string) and converts them to CronTrigger. Agents are considered scheduled if their schedule field is set; there is no separate boolean flag.

### 2.7 WebSocket layer
- backend/zerg/websocket/manager.py
  - Keeps {client_id → websocket}, {topic → set(client_id)}.
  - Topics are plain strings: "agent:{id}", "thread:{id}", etc.
  - EventBus handlers turn internal events into outbound JSON.
  - HTTP upgrade endpoint (routers.websocket) glues it together.

### 2.8 Tests
- to run tests:
  - » ./backend/run_backend_tests.sh

## 3. Front‑end walkthrough

### 3.1 Build & entry
- Cargo.toml pulls wasm‑bindgen, web‑sys, console_error_panic_hook, etc.
- start() in src/lib.rs is #[wasm_bindgen(start)] → sets up panic hook, constructs base DOM stub, connects WebSocket, triggers first API fetches.

### 3.2 Global state
- src/state.rs
  - OnceCell‑style APP_STATE, holds Rc<RefCell<AppState>> with:
    * ws_client (single connection)
    * topic_manager (maps topic → Vec<Callback>)
    * active models, agent list, threads, UI flags …

### 3.3 Network layer
- src/network/ws_client_v2.rs
  - wraps WebSocket; reconnection logic with exponential back‑off.
- src/network/topic_manager.rs
  - mirrors Python manager: subscribe/unsubscribe & callback dispatch.
- src/network/api_client.rs
  - fetch wrapper for standard REST routes, uses serde_json.

### 3.4 Components
- Chat UI (components/chat/) – message list + textarea.
- Dashboard (components/dashboard/) – lists agents/threads; gets live updates by subscribing to topics right after mount.
- Canvas editor – vector shapes, renderer, selection, etc.

### 3.5 Update / Msg enum
- Elm‑style msg handling (src/update.rs) drives state updates.

### 3.6 Storage
- Local‑storage persistence of "last open thread", auth token, window layout.

## 4. Cross‑cutting concerns

### Logging
- Python: std logging; root logger set in main.py.
- Rust: web_sys::console directly, but TODO: swap to gloo‑console for levels.

### Error handling
- Back‑end wraps every router in exception traps and still adds CORS headers.
- Front‑end shows banner in UI_updates when WS disconnects or API fetch fails.

### Security
- OPENAI_API_KEY comes from .env on server; never sent to client.
- CORS is wildcard during dev – tighten for prod.
- No authentication/authorisation yet; every route is public.

### Performance
- Chat streaming is efficient (chunk generator).
- APScheduler runs in same event loop; long LLM calls could block – consider moving heavy inference to worker queues.

## 5. Suggested improvements / risks

### Backend
1. Database isolation: most crud functions open external Session; consider using dependency‑injected scoped session per request to avoid leaks.
2. process_message streams chunks but accumulates them in a Python string — O(n²) copy for long responses; collect list + "".join at the end.
3. Ensure Cron strings validated (currently pass‑through to CronTrigger; bad strings raise at runtime).
4. websocket.manager.TopicConnectionManager uses Dict[str, WebSocket] but never timeouts stale entries if client crashes mid‑handshake.

### Frontend
1. start() directly manipulates DOM; consider a more structured approach to DOM manipulation for better maintainability.
2. TopicManager keeps callbacks in HashMap<String, Vec<_>> without pruning duplicates; add Weak<Callback> or dedup.

### Dev‑ops / CI
- Tests run with sqlite in cwd; parallel CI might race → parametrize db‑file path.
- Provide justfile or Makefile that builds both wasm & backend.

## 6. Mental model cheat‑sheet
- REST → FastAPI routers → CRUD → SQLAlchemy
- WS → /api/ws/{client_id} → TopicConnectionManager
- LLM → AgentRunner.run_thread() → ReAct functional agent (zerg_react_agent.get_runnable) → OpenAI
- Cron → APScheduler → SchedulerService → AgentRunner (scheduled job)
- Browser subscribes to topics → receives JSON deltas → DOM updates via wasm-bindgen.

This should give you a solid grasp of "everything" without drowning in code. Let me know if you want to zoom into any specific file, execution path or test!
-------------------------------------------------------------------------------
## WebSocket & Event Flow

```text
LLM / CRUD change ──▶ EventBus ──▶ TopicConnectionManager ──▶ browser
                                      │                       ▲
                                      └──── heartbeat / ping──┘
```

1. Browser opens `wss://<host>/api/ws?token=<jwt>`.  
2. `validate_ws_jwt()` runs **before** `websocket.accept()`.  
3. Success → socket auto-subscribed to `user:{id}` personal topic.  
4. Failure → close code 4401; front-end banner + logout.

Message schema (v1):

```jsonc
{
  "v": 1,
  "id": "uuid4",
  "topic": "thread:42",
  "type": "thread_message_created",
  "ts": 1719958453,
  "data": { … }
}
```

-------------------------------------------------------------------------------
## Frontend State Management

Same Elm-style description as before (Message enum, pure reducer, command
executors) – see `frontend/src/update.rs` for details.

-------------------------------------------------------------------------------
## Directory Layout (excerpt)

```
backend/
  zerg/
    routers/…     FastAPI routes incl. websocket.py
    websocket/…   TopicConnectionManager + helpers
    agents_def/   Pure LangGraph agent templates

frontend/
  src/
    network/      ws_client_v2.rs, topic_manager.rs, …
    canvas/       Rust renderer for the visual editor
    macros.rs     mut_borrow! convenience macro

prerender/        Node snapshotper (optional)
docs/             Deep-dives & task docs
```

-------------------------------------------------------------------------------
## Testing

* **Backend** – `./backend/run_backend_tests.sh` (200+ pytest tests, >95 % cov).  
* **Frontend** – `./frontend/run_frontend_tests.sh` (wasm-bindgen + headless
  browser).  
* **E2E** – Playwright specs under `e2e/`, run via `./e2e/run_e2e_tests.sh`.

The CI checks also run `cargo clippy -D warnings`, `ruff`, and the pre-commit
hooks.

-------------------------------------------------------------------------------
## Extending / Road-map

*   **Milestone WS-02** – move broadcast to per-socket queue, add trace-id &
    Prometheus counters (see `docs/task_websocket_hardening.md`).  
*   Agent plugins for external services (email, db, http).  
*   Graphical editor: condition/branch nodes, tool nodes.  
*   PostgreSQL migration & async SQLAlchemy once multi-worker fan-out lands.

-------------------------------------------------------------------------------
### Design-System playground

Running `trunk serve` exposes **http://localhost:8002/dev_components.html** – a gallery that previews every shared component (buttons, inputs, toasts …) with live CSS.  Use it while iterating on styles to catch visual bugs quickly.

-------------------------------------------------------------------------------
#EOF
