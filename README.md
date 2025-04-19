# Agent Platform

Agent Platform is a full-stack application combining a Rust + WebAssembly (WASM) frontend, a FastAPI/OpenAI backend, and an optional Node/Playwright pre‑rendering layer for improved SEO. It enables you to create and manage AI-driven "agents" to handle user requests in real-time, streaming AI-generated responses into the browser.

--------------------------------------------------------------------------------
## Table of Contents

1. [Overview](#overview)  
2. [Quick Start](#quick-start)  
3. [Architecture Overview](#architecture-overview)  
4. [Frontend State Management](#frontend-state-management)  
5. [WebSocket & Event Communication](#websocket--event-communication)  
6. [Key Features](#key-features)  
7. [Directory Structure](#directory-structure)  
8. [Dependencies](#dependencies)  
9. [Setup & Running](#setup--running)  
   - [Backend Setup](#backend-setup)  
   - [Frontend Setup](#frontend-setup)  
   - [Pre‑Rendering Setup (Optional)](#pre-rendering-setup-optional)  
10. [Using the Dashboard & Canvas Editor](#using-the-dashboard--canvas-editor)  
11. [Pre‑Rendering & SEO Details](#pre-rendering--seo-details)  
12. [How It Works](#how-it-works)  
13. [Extending the Project & Future Plans](#extending-the-project--future-plans)  
14. [Testing & Verification](#testing--verification)  
15. [License](#license)  

--------------------------------------------------------------------------------
## Overview

Agent Platform is designed for users who need to create, manage, and orchestrate threadal or task-based AI "agents." At a high level:  

• Agents can be created or edited via either:  
  1. The Dashboard view (for a structured, spreadsheet-like experience of agent cards).  
  2. A node-based "Canvas Editor" (built in Rust/WASM) for visually configuring complex instructions or multi-step flows.  

• Agents can be paused, scheduled, or run on-demand. Each agent can maintain a thread history and system instructions.  

• The backend (FastAPI) streams real-time responses from the OpenAI API to connected browsers over WebSockets.  

• An optional Node/Playwright-based pre‑rendering system can generate static snapshots for SEO, serving them to crawlers while humans see the interactive WASM interface.

--------------------------------------------------------------------------------
## Quick Start

Here's the minimal set of commands to get Agent Platform running locally:

1. Clone the repository:  
   » git clone https://github.com/your-username/agent-platform.git  

2. Install prerequisites:  
   - Python 3.12+  
   - Rust (with wasm-pack)  
   - Node.js and npm  

### Backend
1. cd backend  
2. Copy the example environment file:  
   » cp .env.example .env  
   (Insert your OPENAI_API_KEY in the .env file to enable AI calls.)   
3. Run backend server:  
   » uv run python -m uvicorn zerg.main:app --reload --port 8001

### Frontend
1. cd frontend  
2. Build the WASM module and start a local server:  
   » ./build.sh   (production build on http://localhost:8002)  
   or  
   » ./build-debug.sh (debug build with source maps)  

### (Optional) Pre‑rendering
1. cd prerender  
2. npm install  
3. npm run prerender     (Generates dist/index.html as a static snapshot)  
4. node server.js        (Serves content on http://localhost:8003 to bots)  

Visit http://localhost:8002 to see the UI.  

--------------------------------------------------------------------------------
## Architecture Overview

The repository is divided into three main areas:

1. **Frontend (Rust + WebAssembly)**  
   - Uses wasm-bindgen, web-sys, and js-sys for DOM/event handling  
   - Implements an "Elm-style" architecture where:
     • User actions produce Messages (defined in messages.rs)
     • An update function (update.rs) handles state changes
     • Commands handle side effects (API calls, WebSocket actions)
   - Renders three main views:
     • Dashboard for quick agent management
     • Canvas Editor for visual flow configuration
     • Chat interface for real-time agent interaction
   - State management via AppState (state.rs) as single source of truth
   - Communicates with backend via REST API calls and real-time WebSocket connection

2. **Backend (Python + FastAPI)**  
   - Provides both REST and WebSocket endpoints to handle streaming from OpenAI.
   - Uses an event-based architecture with custom decorators and an event bus.
   - REST endpoints handle CRUD operations while WebSocket connections provide real-time updates.
   - Environment variables (OPENAI_API_KEY, etc.) loaded from a .env file.
   - uvicorn or gunicorn can host the app in production.

3. **Pre-Rendering (Node/Playwright)**  
   - Uses a headless browser to capture HTML snapshots for SEO.  
   - An Express-based server (server.js) detects bot user agents and returns the pre-rendered snapshot.

--------------------------------------------------------------------------------
## Frontend State Management

The frontend uses an Elm-like message-passing architecture for state management:

### Message-Based State Updates

All state modifications follow these principles:

1. **Never Directly Mutate State**: Instead of directly modifying `APP_STATE`, dispatch a message that describes the change.

2. **Messages Define Intent**: Each state change has a corresponding message type in `messages.rs`.

3. **Pure State Updates**: The `update()` function in `update.rs` handles state mutations and returns Commands for side effects.

4. **Commands Handle Side Effects**: Network calls, WebSocket actions, and other side effects are handled by command executors.

5. **Use the Message/Command Pattern**:
   ```rust
   // Good: Message that may produce Commands
   crate::state::dispatch_global_message(Message::SendThreadMessage { 
       thread_id,
       content: "Hello agent".to_string(),
       client_id: Some("user-123".to_string())
   });
   // This will:
   // 1. Update thread state in update()
   // 2. Return a Command::SendThreadMessage
   // 3. command_executor handles the API call
   
   // Bad: Direct mutation or side effects
   APP_STATE.with(|state| {
       let mut state = state.borrow_mut();
       state.send_message_to_thread(thread_id, message);  // Don't mix state and side effects!
   });
   ```

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

1. **Topic-Based Subscription Model**
   - The frontend WebSocket client connects to `/api/ws/v2` and can subscribe to specific topics:
     • `thread:{id}` for thread updates and messages
     • `agent:{id}` for agent status updates

2. **Topic Manager**
   - The frontend implements a `TopicManager` that handles:
     • WebSocket connection management
     • Topic subscription
     • Message routing to appropriate handlers

3. **Real-Time UI Updates**
   - When WebSocket messages arrive, they are converted to appropriate Messages:
     • `ReceiveThreadUpdate` for thread metadata changes
     • `ReceiveStreamChunk` for incoming streaming responses
     • `ThreadMessagesLoaded` for thread history

This architecture creates a seamless real-time experience: when an agent is created, a thread is created, or a message is sent, the UI updates instantly across all connected clients without polling.

--------------------------------------------------------------------------------
## Key Features

• **Two UI Approaches (Dashboard & Canvas)**  
  - The Dashboard is a table-like view of agent cards (showing status, quick actions, logs).  
  - The Canvas Editor (in Rust/WASM) is used for more advanced flows or multi-step instructions.  

• **Real‑Time AI Streaming**  
  - The backend streams incremental tokens from OpenAI's API to connected browsers via websockets.  

• **Extensible "Agent" Model**  
  - Each agent stores system instructions, thread history, and status.  
  - Agents can be triggered manually or scheduled.  

• **SEO-Friendly Pre‑Rendering**  
  - A Node + Playwright system captures static HTML snapshots, serving them to web crawlers.  

• **Rust + WASM Performance**  
  - The Canvas Editor uses Rust for efficient rendering and fluid user interactions.

--------------------------------------------------------------------------------
## Directory Structure

A simplified overview of notable top-level files and folders:

• backend/  
   ├── main.py (FastAPI & streaming logic)  
   └── pyproject.toml (Python project config & linting)  

• frontend/  
   ├── Cargo.toml, build.sh, build-debug.sh  
   ├── src/  
   │    ├── canvas/           (shapes, rendering logic)  
   │    ├── components/       (dashboard, chat, canvas UI)  
   │    ├── network/         (API client, WebSocket, topics)  
   │    ├── state.rs         (global AppState)  
   │    ├── messages.rs      (frontend events/actions)  
   │    ├── update.rs        (state mutation logic)  
   │    ├── command_executors.rs (side effects)  
   │    └── lib.rs          (WASM entry point)  
   ├── target/              (build artifacts)  
   └── www/                 (WASM output & static files)

• prerender/  
   ├── prerender.js, server.js (Playwright & Express)  
   ├── dist/ (Generated static HTML snapshots)  
   └── package.json  

--------------------------------------------------------------------------------
## Dependencies

1. **Frontend**  
   - Rust (edition 2021) & wasm-pack  
   - wasm-bindgen, web-sys, serde, console_error_panic_hook  
   - Python 3 for the local dev server in build.sh (if using the script's local server)  

2. **Backend**  
   - Python 3.12+  
   - FastAPI, uvicorn, websockets, openai, python-dotenv  

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
4. A local server at http://localhost:8002 is started, hosting the UI.

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
   - Clicking "Create Agent" adds a new agent node.

2. **Canvas Editor**  
   - Switch to "Canvas Editor" from the top tabs.  
   - This view shows a node-based interface for advanced flows.  
   - You can create "User Input" nodes and "Agent Response" nodes, connect them, drag them around, etc.  
   - The Canvas Editor is best for complex multi-step instructions or officially "chaining" steps.

3. **Agent Modal**  
   - In the canvas, clicking on an "Agent Identity" node opens a modal for system instructions, scheduling, advanced settings.

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
## Extending the Project & Future Plans

• **Additional Scheduling Features**  
  - Full CRON-like scheduling or triggers from external events.  

• **Advanced Agent Plugins**  
  - Agents bridging to external services, e.g. sending emails or reading logs.  

• **Data Analytics & Usage Metrics**  
  - Track token usage, cost, and latency.  

• **Multi-Agent Orchestration**  
  - Let agents share data or call each other.  

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

3. **Additional Tests**  
   - Python-based tests can be placed in backend/tests (pytest).  
   - Additional front-end tests might use wasm-pack test or tools like Cypress (not yet set up).


---- V2 Analysis ----
Below is a practical “big‑picture” walkthrough of the repository.
    I read every top‑level source file (skipping compiler artefacts in target/) and the entire test‑suite so that what follows is accurate, but I’m only summarising – no large blocks of code are reproduced.

    ────────────────────────────────────────

        1. High‑level architecture
           ────────────────────────────────────────
           • “backend/”   — Python 3.12 FastAPI service (codename “zerg”).
             – Relational persistence through SQLAlchemy → SQLite (app.db)
             – Domain: Agents, Threads, Messages.
             – LangChain + LangGraph used to run LLM workflows for each Agent.
             – Real‑time layers:
           • in‑process EventBus (pub/sub enum EventType)
           • topic‑based WebSocket hub that relays EventBus messages to browsers.

      – APScheduler drives cron‑like runs of Agents.
      – Almost 200 Pytest tests give >95 % coverage.

    • “frontend/”  — Rust (edition 2021) Yew / wasm‑bindgen SPA.
      – Compiles to WASM; starter html in www/ mounts the app.
      – Mirrors back‑end models in src/models.rs and keeps global app‑state in src/state.rs.
      – Two big UI surfaces:
            • Canvas editor (graphical editor built on <canvas>)
            • Chat/dashboard (Yew components with split‑pane layout)
      – A single WebSocket client (src/network/ws_client_v2.rs) plus TopicManager
        handles subscribe / publish just like the Python side.
      – Async/await throughout via wasm‑bindgen‑futures.

    ────────────────────────────────────────
    2.  Back‑end walkthrough
    ────────────────────────────────────────
    main entry‑point
    └── backend/zerg/main.py
        • boots FastAPI, installs CORS & OPTIONS middleware.
        • creates tables, mounts routers, starts/stops SchedulerService.

    2.1 Persistence layer
        backend/zerg/app/database.py
            – Standard SQLAlchemy engine/session helpers.
        backend/zerg/app/models/models.py
            – Three tables: Agent, Thread, *Message (two subclasses for agent‑ and
              thread‑scope but share columns).
              All models expose JSON columns for flexible config.

    2.2 CRUD helpers (thin)
        backend/zerg/app/crud/crud.py
            – Pure SQLAlchemy operations, no business logic.

    2.3 Event bus
        backend/zerg/app/events/event_bus.py
            – Very small async pub/sub written from scratch.
            – Decorator publish_event(event_type) lets router functions
              emit automatically.

    2.4 Routers
        • agents.py – full CRUD, nested /messages sub‑endpoints.
        • threads.py – manage conversation threads & messages.
        • websocket.py – HTTP handshake that upgrades to WS and
          delegates to TopicConnectionManager.
        • models.py – surfaces list of OpenAI models (one simple GET).

          All routers are version‑less but live under prefix “/api”.

    2.5 Agent runtime
        backend/zerg/app/agents.py
            – Wraps an Agent row and hides LangGraph plumbing.
            – get_or_create_thread() lazily builds Thread.
            – process_message() builds a LangGraph state machine:
                  START ─► chatbot node ─► END
              The node calls OpenAI ChatCompletion (via langchain_openai.ChatOpenAI).
            – Supports streaming via generator: yields chunks to caller.

    2.6 SchedulerService
        backend/zerg/app/services/scheduler_service.py
            – AsyncIOScheduler.
            – On startup loads all agents where run_on_schedule=True and
              schedule ≠ NULL, converts cron strings to CronTrigger.
            – Subscribes to agent‑events so schedule stays in sync.
            – run_agent_task(): gets (or creates) a thread, injects system message, then
              calls process_message(stream=False).

    2.7 WebSocket layer
        backend/zerg/app/websocket/manager.py
            – Keeps {client_id → websocket}, {topic → set(client_id)}.
            – Topics are plain strings: “agent:{id}”, “thread:{id}”, etc.
            – EventBus handlers turn internal events into outbound JSON.
            – HTTP upgrade endpoint (routers.websocket) glues it together.

    2.8 Tests
        • tests/ directory is exhaustive: CRUD, scheduler timing, event‑bus semantics,
          streaming routes, WebSocket integration (async test‑client).
        • A tiny uvicorn wrapper plus run_tests.sh for CI.

    ────────────────────────────────────────
    3.  Front‑end walkthrough
    ────────────────────────────────────────
    3.1 Build & entry
        • Cargo.toml pulls yew 0.20, wasm‑bindgen, web‑sys, console_error_panic_hook, etc.
        • start() in src/lib.rs is #[wasm_bindgen(start)]  → sets up panic hook,
          constructs base DOM stub, connects WebSocket, triggers first API fetches.

    3.2 Global state
        src/state.rs
            – OnceCell‑style APP_STATE, holds Rc<RefCell<AppState>> with:
                  • ws_client (single connection)
                  • topic_manager (maps topic → Vec<Callback>)
                  • active models, agent list, threads, UI flags …

    3.3 Network layer
        src/network/ws_client_v2.rs
            – wraps WebSocket; reconnection logic with exponential back‑off.
        src/network/topic_manager.rs
            – mirrors Python manager: subscribe/unsubscribe & callback dispatch.
        src/network/api_client.rs
            – fetch wrapper for standard REST routes, uses serde_json.

    3.4 Components
        • Chat UI (components/chat/) – message list + textarea.
        • Dashboard (components/dashboard/) – lists agents/threads; gets live updates
          by subscribing to topics right after mount.
        • Canvas editor – vector shapes, renderer, selection, etc.

    3.5 Update / Msg enum
        Elm‑style msg handling (src/update.rs) drives state updates.

    3.6 Storage
        Local‑storage persistence of “last open thread”, auth token, window layout.

    ────────────────────────────────────────
    4.  Cross‑cutting concerns
    ────────────────────────────────────────
    Logging
        – Python: std logging; root logger set in main.py.
        – Rust: web_sys::console directly, but TODO: swap to gloo‑console for levels.

    Error handling
        – Back‑end wraps every router in exception traps and still adds CORS headers.
        – Front‑end shows banner in UI_updates when WS disconnects or API fetch fails.

    Security
        • OPENAI_API_KEY comes from .env on server; never sent to client.
        • CORS is wildcard during dev – tighten for prod.
        • No authentication/authorisation yet; every route is public.

    Performance
        • Chat streaming is efficient (chunk generator).
        • APScheduler runs in same event loop; long LLM calls could block – consider
          moving heavy inference to worker queues.

    ────────────────────────────────────────
    5.  Suggested improvements / risks
    ────────────────────────────────────────
    Backend

        1. Database isolation: most crud functions open external Session; consider
           using dependency‑injected scoped session per request to avoid leaks.
        2. process_message streams chunks but accumulates them in a Python string —
           O(n²) copy for long responses; collect list + \"\".join at the end.
        3. Ensure Cron strings validated (currently pass‑through to CronTrigger; bad
           strings raise at runtime).
        4. websocket.manager.TopicConnectionManager uses Dict[str, WebSocket] but
           never timeouts stale entries if client crashes mid‑handshake.

    Frontend

        1. start() directly manipulates DOM; Yew now supports portals — consider a pure
           component hierarchy to remove manual document queries.
        2. TopicManager keeps callbacks in HashMap<String, Vec<_>> without pruning
           duplicates; add Weak<Callback> or dedup.
        3. Large incremental compilation artefacts committed to git? add target/ to .gitignore.

    Dev‑ops / CI
    • Tests run with sqlite in cwd; parallel CI might race → parametrize db‑file path.
    • Provide justfile or Makefile that builds both wasm & backend.

    ────────────────────────────────────────
    6.  Mental model cheat‑sheet
    ────────────────────────────────────────
    REST  ➜ FastAPI routers ➜ CRUD ➜ SQLAlchemy
    WS    ➜ /api/ws/{client_id} ➜ TopicConnectionManager
    LLM   ➜ AgentManager.process_message() → LangGraph graph → OpenAI
    Cron  ➜ APScheduler → SchedulerService → AgentManager.run_agent_task
    Browser subscribes to topics → receives JSON deltas → Yew updates components.

    ────────────────────────────────────────
    That should give you a solid grasp of “everything” without drowning in code.
    Let me know if you want to zoom into any specific file, execution path or test!