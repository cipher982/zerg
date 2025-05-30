# Agent Platform

Agent Platform is a full-stack reference application that lets you **create &
orchestrate AI agents** in real-time.  It couples a FastAPI backend (Python
3.12) with a purely-Rust + WASM front-end, and ships an optional Node +
Playwright pre-rendering layer for SEO.

The project acts as both a production-ready starter kit *and* a playground for
modern WebSocket streaming patterns, LangGraph-based agents, and a
canvas-style visual editor – all under one repo.

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
10. [Extending / Road-map](#extending--road-map)

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
Node 16+.

```bash
# 1. clone & install rust/wasm targets as usual

# 2. backend
cd backend
cp .env.example .env                 # add OPENAI_API_KEY here
uv run python -m uvicorn zerg.main:app --reload --port 8001

# 3. frontend (in another tab)
cd ../frontend
./build.sh                           # prod build + dev server on :8002

# 4. visit http://localhost:8002  – login overlay skipped in dev mode
```

Tests must be executed via helper scripts (they set env & flags):

```bash
cd backend   && ./run_backend_tests.sh   # not pytest directly!
cd frontend  && ./run_frontend_tests.sh
```

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
#EOF
