# Agent Platform

Agent Platform is a full-stack application combining a Rust + WebAssembly (WASM) frontend, a FastAPI/OpenAI backend, and an optional pre‑rendering system for improved SEO. It allows you to create and manage AI-driven “agents” that can handle user requests in real-time, streaming AI-generated responses directly into the browser.

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
8. [How It Works](#how-it-works)  
9. [Extending the Project & Future Plans](#extending-the-project--future-plans)  
10. [Testing & Verification](#testing--verification)  
11. [License](#license)  

--------------------------------------------------------------------------------
## Overview
Agent Platform is designed for users who need to create, manage, and orchestrate conversational or task-based AI “agents.” At a high level:  
• Agents can be created via a “Dashboard” view, which lists all existing agents, their statuses, and quick actions (run, pause, schedule, etc.).  
• When creating or editing agents, an optional “Canvas Editor” (implemented in Rust/WASM) provides a node-based interface for configuring complex inputs, system instructions, or chain-of-thought flows.  
• The backend (FastAPI) handles requests/responses using OpenAI’s API, providing real-time streaming to the UI.  
• A Node/Playwright-based prerendering system can generate static snapshots for improved SEO, serving them conditionally to crawlers.  

--------------------------------------------------------------------------------
## Quick Start
Here’s the minimal set of commands to get Agent Platform running on your local machine:

1. Clone the repository.  
2. Make sure you have Python 3.12+, Rust with wasm-pack, and Node.js installed.

• Install and run the backend:
  » cd backend  
  » cp .env.example .env   # add your OPENAI_API_KEY  
  » pip install -r requirements.txt  
  » uvicorn main:app --host 0.0.0.0 --port 8001  

• Build and run the frontend:
  » cd frontend  
  » ./build.sh (or build-debug.sh)  
  »   # This compiles Rust -> WASM and starts a local server at http://localhost:8002  

• (Optional) Run the prerender server:
  » cd prerender  
  » npm install  
  » npm run prerender   # Generates a static index.html and screenshot  
  » node server.js      # Serves prerendered content to bots on port 8003  

Open your browser at http://localhost:8002 to see the interactive UI.  

--------------------------------------------------------------------------------
## Architecture Overview
This repository is divided into three main parts:

1. **Frontend (Rust + WASM)**  
   - Uses wasm-bindgen, web-sys, and js-sys for DOM interactions in the browser.  
   - The “Dashboard” manages high-level agent cards, while the “Canvas Editor” offers node-based UI for advanced flows.  
   - Local state is primarily in state.rs, with event listeners and UI logic in src/ui/ and src/components/.  

2. **Backend (Python + FastAPI)**  
   - Provides RESTful APIs and WebSocket endpoints to handle real-time streaming from the OpenAI API.  
   - Contains environment-based configuration for the OpenAI key.  

3. **Pre‑Rendering (Node/Playwright)**  
   - Runs a headless browser to capture the fully rendered HTML for SEO.  
   - Offers an Express-based server (server.js) that detects bot user agents and serves the pre-rendered snapshot.  

Data Flow Summary:
1. The user interacts with the Rust/WASM UI to create or manage agents.  
2. The UI sends requests to the FastAPI server.  
3. OpenAI’s streaming API is called, chunked responses are piped back.  
4. The prerender system optionally serves static HTML for bot traffic.  

--------------------------------------------------------------------------------
## Key Features
• **Dashboard for Agent Management**  
  A dedicated view listing all agents, each with status, quick actions (run, stop, schedule), and a link to detailed logs.  

• **Canvas Editor for Flows**  
  For more complex interactions, the WASM-based node editor (originally the main UI) can be used in a modal to visually define agents’ system instructions or multi-step flows.  

• **Real‑time AI Streaming**  
  The backend uses OpenAI’s streaming API to deliver incremental tokens to connected browsers via WebSockets.  

• **State Persistence & LocalStorage**  
  Key parameters, node graphs, and viewport settings persist automatically.  

• **SEO-Friendly Pre‑Rendering (Optional)**  
  A Node-based system captures prerendered HTML, serving it to crawlers while giving humans the live WASM experience.  

• **Modular & Extensible**  
  The codebase is organized into discrete modules in both Rust (frontend) and Python (backend) for easier maintenance.  

--------------------------------------------------------------------------------
## Directory Structure
A simplified overview of notable top-level files and folders:

• backend/  
   ├── main.py (FastAPI & streaming logic)  
   ├── requirements.txt  
   └── pyproject.toml (Project config/linting)  

• frontend/  
   ├── Cargo.toml, build.sh, build-debug.sh  
   ├── src/ (Rust code for UI, including components & canvas)  
   ├── target/ (Build artifacts)  
   └── www/ (Compiled WASM output & static files)  

• prerender/  
   ├── prerender.js, server.js (Playwright & Express)  
   ├── dist/ (Generated static HTML snapshots)  
   └── package.json  

In addition, there is a .gitignore, this README.md, and an Apache 2.0 LICENSE.  

--------------------------------------------------------------------------------
## Dependencies
1. **Frontend**  
   - Rust (edition 2021) & wasm-pack  
   - wasm-bindgen, web-sys, js-sys, serde, serde-wasm-bindgen  
   - console_error_panic_hook (for better debugging)  

2. **Backend**  
   - Python 3.12+  
   - FastAPI, uvicorn, websockets, openai, python-dotenv  

3. **Pre-Rendering (Optional)**  
   - Node.js and npm  
   - playwright-chromium, express  

--------------------------------------------------------------------------------
## Setup & Running

### Backend Setup
1. Navigate to backend/.  
2. Copy .env.example to .env; insert your OPENAI_API_KEY.  
3. pip install -r requirements.txt  
4. Run with:  
   uvicorn main:app --host 0.0.0.0 --port 8001  
   (Backend is accessible at http://localhost:8001)  

### Frontend Setup
1. Ensure you have Rust, cargo, and wasm-pack installed:  
   cargo install wasm-pack  
2. In frontend/, build the WASM:  
   ./build.sh (for production) or ./build-debug.sh (for debug)  
3. This spins up a local Python server at http://localhost:8002.  

### Pre-rendering Setup (Optional)
1. In prerender/, npm install.  
2. npm run prerender to create dist/index.html.  
3. node server.js to serve the pre-rendered snapshot to bots at http://localhost:8003.  

--------------------------------------------------------------------------------
## How It Works
1. **Dashboard Flow**  
   - You load http://localhost:8002.  
   - The Rust/WASM code initializes a “Dashboard” view with agent cards.  
   - Each card has quick actions (create new agent, run, etc.).  

2. **Canvas Editor (Optional)**  
   - When you want a more complex setup, you open the “Canvas Editor” in a modal.  
   - The node-based approach lets you define system instructions, dynamic prompts, or multiple child nodes.  

3. **Backend Processing**  
   - The FastAPI backend receives your input, sets up an OpenAI streaming call, and sends chunked tokens over WebSocket.  

4. **Pre‑rendering**  
   - For crawlers, server.js in prerender/ checks user agents and returns the static snapshot from dist/index.html if it’s a recognized bot.  

--------------------------------------------------------------------------------
## Extending the Project & Future Plans
- **Additional Agent Features**  
  Plan to add scheduling widgets, external API integrations, and agent-to-agent chaining.  

- **Dashboard Enhancements**  
  Introducing advanced filtering, grouping, or searching of agents. Possibly support drag-and-drop reordering of agent cards.  

- **Canvas Refactors**  
  Turn the node graph editor into a fully modular system, letting you create reusable logic blocks or plugin-based expansions.  

- **Performance Metrics**  
  Display usage cost or token usage in the Agent Dashboard, pulling data from OpenAI’s usage endpoints.  

- **Roadmap**  
  - GPU-accelerated inference (when local LLMs are feasible)  
  - Additional language bindings (e.g., Go, Node) for the backend  
  - Multi-agent orchestration logic  

--------------------------------------------------------------------------------
## Testing & Verification
• Run the included script in prerender/ (test-crawler.sh) to see if pre-rendering works for different user agents:  
  ./test-crawler.sh  
• Check logs in your terminal for any warnings or errors from uvicorn, the prerender server, or the Rust WASM console output.  
• For Python-based tests, use pytest or define them in backend/tests.  