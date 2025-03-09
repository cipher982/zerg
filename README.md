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
   - [Pre‑Rendering Setup (Optional)](#pre-rendering-setup-optional)  
8. [Using the Dashboard & Canvas Editor](#using-the-dashboard--canvas-editor)  
9. [Pre‑Rendering & SEO Details](#pre-rendering--seo-details)  
10. [How It Works](#how-it-works)  
11. [Extending the Project & Future Plans](#extending-the-project--future-plans)  
12. [Testing & Verification](#testing--verification)  
13. [License](#license)  

--------------------------------------------------------------------------------
## Overview

Agent Platform is designed for users who need to create, manage, and orchestrate conversational or task-based AI “agents.” At a high level:  

• Agents can be created or edited via either:  
  1. The Dashboard view (for a structured, spreadsheet-like experience of agent cards).  
  2. A node-based “Canvas Editor” (built in Rust/WASM) for visually configuring complex instructions or multi-step flows.  

• Agents can be paused, scheduled, or run on-demand. Each agent can maintain a conversation history and system instructions.  

• The backend (FastAPI) streams real-time responses from the OpenAI API to connected browsers over WebSockets.  

• An optional Node/Playwright-based pre‑rendering system can generate static snapshots for SEO, serving them to crawlers while humans see the interactive WASM interface.

--------------------------------------------------------------------------------
## Quick Start

Here’s the minimal set of commands to get Agent Platform running locally:

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
3. Install dependencies:  
   » pip install -r requirements.txt  
4. Run backend server:  
   » uvicorn main:app --host 0.0.0.0 --port 8001  

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
   - Uses wasm-bindgen, web-sys, and js-sys for DOM/event handling.  
   - Renders a “Dashboard” for quick agent management and a “Canvas Editor” for more advanced flows.  
   - State, events, and UI logic are in modules (src/state.rs, src/ui/, src/components/).

2. **Backend (Python + FastAPI)**  
   - Provides both REST and WebSocket endpoints to handle streaming from OpenAI.  
   - Environment variables (OPENAI_API_KEY, etc.) loaded from a .env file.  
   - uvicorn or gunicorn can host the app in production.

3. **Pre-Rendering (Node/Playwright)**  
   - Uses a headless browser to capture HTML snapshots for SEO.  
   - An Express-based server (server.js) detects bot user agents and returns the pre-rendered snapshot.

--------------------------------------------------------------------------------
## Key Features

• **Two UI Approaches (Dashboard & Canvas)**  
  - The Dashboard is a table-like view of agent cards (showing status, quick actions, logs).  
  - The Canvas Editor (in Rust/WASM) is used for more advanced flows or multi-step instructions.  

• **Real‑Time AI Streaming**  
  - The backend streams incremental tokens from OpenAI’s API to connected browsers via websockets.  

• **Extensible “Agent” Model**  
  - Each agent stores system instructions, conversation history, and status.  
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
   - Python 3 for the local dev server in build.sh (if using the script’s local server)  

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
3. Install dependencies:  
   » pip install -r requirements.txt  
4. Run the server:  
   » uvicorn main:app --host 0.0.0.0 --port 8001  

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
   - The default tab shows “Agent Dashboard,” with existing agents in a table.  
   - Each card shows agent name, status, quick actions (run, pause, edit), logs if available.  
   - Clicking “Create Agent” adds a new agent node.

2. **Canvas Editor**  
   - Switch to “Canvas Editor” from the top tabs.  
   - This view shows a node-based interface for advanced flows.  
   - You can create “User Input” nodes and “Agent Response” nodes, connect them, drag them around, etc.  
   - The Canvas Editor is best for complex multi-step instructions or officially “chaining” steps.

3. **Agent Modal**  
   - In the canvas, clicking on an “Agent Identity” node opens a modal for system instructions, scheduling, advanced settings.

--------------------------------------------------------------------------------
## Pre‑Rendering & SEO Details

• Why Pre-rendering: Single-page apps or WASM apps can be difficult for search engines to crawl. By using Node+Playwright, we generate a static snapshot that is served to bot user-agents, boosting SEO.  

• How to Generate Snapshots:  
  1) Ensure your backend (port 8001) and frontend dev server (port 8002) are running.  
  2) cd prerender && npm run prerender.  
  3) The script loads http://localhost:8002 in a headless browser, waits for WASM, then saves dist/index.html.  

• Serving the Snapshot:  
  1) node server.js starts an Express server on http://localhost:8003.  
  2) When a recognized bot user-agent hits “/”, it serves the dist/index.html snapshot.  
  3) Regular users see the live WASM app from the frontend.  

• Testing Bot Detection:  
  - The test-crawler.sh script simulates requests from normal and “Googlebot” user agents, saving the results to human.html or googlebot.html for comparison.  

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