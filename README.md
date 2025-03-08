# Agent Platform

Agent Platform is a full-stack application combining a Rust + WebAssembly (WASM) frontend, a FastAPI/OpenAI backend, and an optional Node/Playwright pre‑rendering layer for improved SEO. It enables you to create and manage AI-driven “agents” to handle user requests in real-time, streaming AI-generated responses into the browser.

--------------------------------------------------------------------------------
## Table of Contents

1. [Overview](#overview)  
2. [Quick Start](#quick-start)  
3. [Architecture Overview](#architecture-overview)  
4. [Key Features](#key-features)  
5. [Directory Structure](#directory-structure)  
6. [Dependencies](#dependencies)  
7. [Setup & Running](#setup--running)  
   - [Backend Setup](#backend-setup)  
   - [Frontend Setup](#frontend-setup)  
   - [Pre-rendering Setup (Optional)](#pre-rendering-setup-optional)  
8. [Using the Dashboard & Canvas Editor](#using-the-dashboard--canvas-editor)  
9. [How It Works](#how-it-works)  
10. [Extending the Project & Future Plans](#extending-the-project--future-plans)  
11. [Testing & Verification](#testing--verification)  
12. [License](#license)  

--------------------------------------------------------------------------------
## Overview

Agent Platform is designed for users who need to create, manage, and orchestrate conversational or task-based AI “agents.” At a high level:  

• Agents can be created or edited via either:  
  1. The Dashboard view (for a structured, spreadsheet-like experience of agent cards).  
  2. A Node-based “Canvas Editor” (built in Rust/WASM) for visually configuring complex instructions or multi-step flows.  

• Agents can be paused, scheduled, or run on-demand. Each agent can maintain a conversation history and system instructions.  

• The backend (FastAPI) streams real-time responses from the OpenAI API to connected browsers over WebSockets.  

• An optional Node/Playwright-based pre‑rendering system can generate static snapshots for SEO, serving them to crawlers while humans see the interactive WASM interface.

--------------------------------------------------------------------------------
## Quick Start

Here’s the minimal set of commands to get Agent Platform running locally:

1. Clone the repository:
   » git clone https://github.com/your-username/agent-platform.git
2. Install prerequisites: Python 3.12+, Rust (with wasm-pack), and Node.js.

• Backend:  
   1. cd backend  
   2. cp .env.example .env  # Insert your OPENAI_API_KEY  
   3. pip install -r requirements.txt  
   4. uvicorn main:app --host 0.0.0.0 --port 8001  

• Frontend:  
   1. cd frontend  
   2. ./build.sh (or ./build-debug.sh for a debug build)  
   3. This launches a local server at http://localhost:8002  

• (Optional) Pre‑rendering:  
   1. cd prerender  
   2. npm install  
   3. npm run prerender     # Generates dist/index.html  
   4. node server.js        # Serves content on http://localhost:8003  

Visit http://localhost:8002 to see the UI.  

--------------------------------------------------------------------------------
## Architecture Overview

This repo is divided into three main areas:

1. **Frontend (Rust + WebAssembly)**  
   • Uses wasm-bindgen, web-sys, and js-sys for DOM and event handling.  
   • Renders a “Dashboard” for quick agent management and a “Canvas Editor” for visual flows.  
   • State, events, and UI logic are organized in modules (e.g., src/state.rs, src/ui/, src/components/).

2. **Backend (Python + FastAPI)**  
   • Provides both REST and WebSocket endpoints to handle streaming from OpenAI.  
   • Environment variables are read from a .env file for your OPENAI_API_KEY (and any future keys).  
   • uvicorn or gunicorn can host the app in production.

3. **Pre-Rendering (Node/Playwright)**  
   • Uses a headless browser to capture HTML snapshots for SEO.  
   • An Express-based server (server.js) detects bot user agents and returns the pre-rendered snapshot.  

--------------------------------------------------------------------------------
## Key Features

• **Two UI Approaches (Dashboard & Canvas)**  
  - The Dashboard is a table-like view of agent cards (status, quick actions, etc.).  
  - The Canvas Editor (in Rust/WASM) is used for more advanced flows or multi-step instructions.  

• **Real‑Time AI Streaming**  
  - The backend streams incremental tokens from OpenAI’s API to connected browsers via websockets.  

• **Extensible “Agent” Model**  
  - Each agent can store system instructions, conversation history, and status.  
  - Agents can be triggered manually or scheduled.  

• **SEO-Friendly Pre‑Rendering**  
  - A Node + Playwright system captures a static HTML snapshot, serving it to web crawlers.

• **Rust + WASM Performance**  
  - The Canvas Editor uses Rust for efficient rendering and fluid user interactions.

--------------------------------------------------------------------------------
## Directory Structure

A simplified overview of notable top-level files and folders:

• backend/  
   ├── main.py (FastAPI & streaming logic)  
   ├── requirements.txt  
   └── pyproject.toml (Python project config & linting)  

• frontend/  
   ├── Cargo.toml, build.sh, build-debug.sh  
   ├── src/ (Rust code: components, ui, state, canvas)  
   ├── target/ (Build artifacts)  
   └── www/ (Final WASM output & static files)

• prerender/  
   ├── prerender.js, server.js (Playwright & Express)  
   ├── dist/ (Generated static HTML snapshots)  
   └── package.json  

--------------------------------------------------------------------------------
## Dependencies

1. **Frontend**  
   - Rust (edition 2021) & wasm-pack  
   - wasm-bindgen, web-sys, serde, console_error_panic_hook  
   - Python 3 for the local dev server in build.sh  

2. **Backend**  
   - Python 3.12+  
   - FastAPI, uvicorn, websockets, openai, python-dotenv  

3. **Pre-Rendering (Optional)**  
   - Node.js and npm  
   - playwright-chromium, express  

--------------------------------------------------------------------------------
## Setup & Running

### Backend Setup
1. Navigate to backend/:  
   » cd backend  
2. Copy the environment file and edit it:  
   » cp .env.example .env  
   (Add your OPENAI_API_KEY)  
3. Install dependencies:  
   » pip install -r requirements.txt  
4. Run:  
   » uvicorn main:app --host 0.0.0.0 --port 8001  

### Frontend Setup
1. Ensure you have Rust, cargo, and wasm-pack installed.  
2. Navigate to frontend/:  
   » cd frontend  
3. Build the WASM module:  
   » ./build.sh       (production)  
     or  
   » ./build-debug.sh (debug with source maps)  
4. A local server at http://localhost:8002 is started, hosting the UI.

### Pre-rendering Setup (Optional)
1. cd prerender  
2. npm install  
3. npm run prerender  # Renders dist/index.html  
4. node server.js     # Serves snapshots on http://localhost:8003  

--------------------------------------------------------------------------------
## Using the Dashboard & Canvas Editor

After launching the frontend (http://localhost:8002):

1. **Dashboard**  
   - The default tab is the “Agent Dashboard,” showing existing agents.  
   - Each card shows agent name, status, quick actions (run, pause, edit), and logs if available.  
   - Clicking “Create Agent” adds a new agent node.

2. **Canvas Editor**  
   - Switch to “Canvas Editor” from the top tabs.  
   - This view shows a node-based interface for advanced flows:  
     • Create “User Input” nodes and “Agent Response” nodes.  
     • Connect them via edges to define multi-turn logic.  
     • Drag nodes to reposition; the system auto-fits if enabled.

3. **Agent Modal**  
   - In the canvas, clicking on an “Agent Identity” node opens a modal to configure system instructions, schedule, or advanced settings.

--------------------------------------------------------------------------------
## How It Works

1. **User Action**: A user interacts with the Dashboard or Canvas, e.g., creating or editing an agent.  
2. **Backend**: A FastAPI route handles the request, setting up OpenAI’s streaming call for live response.  
3. **Streaming**: The backend broadcasts tokens over WebSockets to the relevant agent node in the UI.  
4. **Pre‑Rendering**: For bots, server.js in /prerender returns a pre-rendered HTML snapshot instead of the live WASM page.

--------------------------------------------------------------------------------
## Extending the Project & Future Plans

• **Additional Scheduling Features**  
  - Full CRON-like scheduling or external triggers.  

• **Advanced Agent Plugins**  
  - Agents bridging to external services, e.g., sending emails or reading logs.  

• **Data Analytics & Usage Metrics**  
  - Track token usage, cost, performance times.  

• **Multi-Agent Orchestration**  
  - Let agents share data or call each other.  

• **Refinement of Canvas Editor**  
  - Enhanced node library for more complex flows or branching logic.

--------------------------------------------------------------------------------
## Testing & Verification

1. **Basic Functionality**  
   - Start the backend (uvicorn) and frontend (./build.sh).  
   - Create a new agent in the dashboard or canvas, send a test prompt, and confirm a streaming response in the UI.  

2. **Logs & Console**  
   - Check the console logs in your browser dev tools, as well as the uvicorn logs in your terminal.  
   - For prerendering, check prerender/dist/index.html and test-crawler.sh to verify the bot detection.  

3. **Optional Tests**  
   - Python-based tests can be placed in backend/tests, with pytest or coverage.  
   - Additional front-end tests would involve wasm-pack test or Cypress integration (if set up).