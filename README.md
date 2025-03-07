# Agent Platform

Agent Platform is a full-stack application that combines a Rust + WebAssembly frontend with a FastAPI/OpenAI backend and a pre‑rendering solution for SEO. This system lets users build interactive conversation flows with AI responses; it features a dynamic canvas interface, local state persistence, real‑time streaming responses, and responsive design built from the ground up.

## Table of Contents
1. [Overview](#overview)
2. [Architecture Overview](#architecture-overview)
3. [Key Features](#key-features)
4. [Directory Structure](#directory-structure)
5. [Dependencies](#dependencies)
6. [Setup & Running](#setup--running)
   - [Backend Setup](#backend-setup)
   - [Frontend Setup](#frontend-setup)
   - [Pre-rendering Setup](#pre-rendering-setup)
7. [How It Works](#how-it-works)
8. [Extending the Project](#extending-the-project)
9. [Testing & Verification](#testing--verification)
10. [License](#license)

---

## Overview
Agent Platform lets you create and manage an interactive conversation tree where user inputs (blue nodes) trigger AI responses (purple nodes) that are displayed visually on a draggable, zoomable canvas. The frontend (written in Rust and compiled to WebAssembly) communicates with the backend (a FastAPI server) which interacts with the OpenAI API via streaming. To further improve SEO, a separate pre‑rendering system captures the application’s HTML state for bot crawlers.

---

## Architecture Overview
The repository is divided into three major parts:
- **Frontend (Rust + WASM):**  
  - Implements the UI including canvas drawing, node management, dynamic favicon generation, and user interaction.  
  - Uses wasm-bindgen, web-sys, and serde for efficient browser interactions.
- **Backend (Python/FastAPI):**  
  - Serves as the AI “brain” using the OpenAI API.  
  - Handles streaming responses via websockets and a POST request endpoint.
- **Pre-rendering (Node.js/Express):**  
  - Uses Playwright/Chromium to load your WASM app headlessly and generate static HTML snapshots for crawlers.
  
A typical flow starts when the user adds a message via the UI. The frontend creates a node and sends the text to the backend over HTTP and websockets. The backend uses OpenAI’s streaming API to generate a response, sending chunks in real time back to update the UI. Meanwhile, the pre‑rendering system periodically captures a snapshot of the rendered application for SEO.

---

## Key Features
- **Interactive Conversation Canvas:**  
  Drag-and-drop nodes, zoom and pan the view, and see live updates as nodes are created.
- **Real‑time AI Responses:**  
  Uses WebSocket streaming from the backend with chunked data updates.
- **State Persistence:**  
  Node graph, viewport settings, and selected AI model are saved in localStorage.
- **Dynamic Favicon Generation:**  
  A small canvas generates a fav icon on the fly.
- **Agent Configuration:**  
  Create “agent nodes” with custom system instructions and conversation history.
- **SEO Friendly:**  
  A pre‑rendering system serves static HTML snapshots to search engine crawlers.
- **Responsive & Adaptive:**  
  Fully responsive design with auto-fit/center view capabilities and manual override for layout control.

---

## Directory Structure
A simplified overview of the top-level files and folders:
- **backend/**  
  Contains the FastAPI server (main.py), API configuration (.env.example), and Python project files.
- **frontend/**  
  Contains the Rust crate (Cargo.toml), source code (src/), and build scripts (build.sh/build-debug.sh).  
  The UI is built with modules for canvas drawing (canvas/renderer.rs and canvas/shapes.rs), state management (state.rs), networking (network.rs), storage, and more.
- **prerender/**  
  Contains the Node.js-based pre‑rendering system (prerender.js, server.js, package.json, and test-crawler.sh).
- **Root Files:**  
  .gitignore, LICENSE, README.md, and pyproject.toml (for backend tooling).

---

## Dependencies
### For the Frontend:
- Rust (Edition 2021) with the WebAssembly toolchain
- wasm-pack for compiling Rust code to WASM
- Crates: wasm-bindgen, web-sys, js-sys, serde (with derive), serde_json, serde-wasm-bindgen, wasm-bindgen-futures, and console_error_panic_hook

### For the Backend:
- Python 3.12+  
- FastAPI, uvicorn, websockets, python-dotenv, and openai Python package  
- A valid OpenAI API key (configure via backend/.env)

### For the Pre‑rendering:
- Node.js and npm  
- Dependencies: playwright‑chromium and express

---

## Setup & Running

### Backend Setup
1. Navigate to the `backend` folder.
2. Copy `.env.example` to `.env` and set your OpenAI API key.
3. Install the dependencies (e.g., using pip or your preferred tool):
   • `pip install -r requirements.txt`
4. Run the FastAPI server:
   • `uvicorn main:app --host 0.0.0.0 --port 8001`
   
The backend will be available at http://localhost:8001 and will also serve the WebSocket endpoint at `/ws`.

### Frontend Setup
1. Ensure you have Rust and wasm-pack installed:
   • `cargo install wasm-pack`
2. In the `frontend` directory, run the build script:
   • For production:  
   `chmod +x build.sh && ./build.sh`
   • For debug with source maps:  
   `chmod +x build-debug.sh && ./build-debug.sh`
3. The build will compile the WASM module to the `www` folder and start a Python HTTP server on port 8002.
4. Open your browser at http://localhost:8002 to interact with the UI.

### Pre‑rendering Setup
1. Navigate to the `prerender` directory.
2. Install Node.js dependencies:
   `npm install`
3. Generate a pre‑rendered HTML snapshot:
   `npm run prerender`
  This creates a `dist` folder containing an `index.html` and a `screenshot.png`.
4. To serve content with crawler detection, run:
   `node server.js`
  The server listens (by default) on http://localhost:8003 and serves static frontend content to human users while serving the pre‑rendered HTML to bots.

---

## How It Works
- **Frontend (Rust/WASM):**  
  – On load, the `start()` function (in lib.rs) initializes the UI, sets up event handlers, the canvas, and WebSocket for live updates.  
  – It manages a global application state via the `AppState` structure (in state.rs) to track nodes, viewport settings, and current interactions.  
  – When a user creates a node or drags the canvas, the corresponding modules update the canvas rendering.
- **Networking:**  
  – The `network.rs` module establishes a WebSocket to “ws://localhost:8001/ws” that receives streamed text chunks from the backend.
  – It also uses the Fetch API to post text data (including message IDs, selected AI model, and optional system instructions) to “/api/process-text.”
- **Backend (Python/FastAPI):**  
  – The backend’s `/api/process-text` endpoint uses the OpenAI API (with streaming enabled) to process user input and sends response chunks to all connected clients.
  – The WebSocket endpoint in `main.py` maintains live connections and broadcasts the streaming data.
- **Pre‑rendering:**  
  – A Puppeteer/Playwright script (prerender.js) launches Chromium, loads the frontend, waits for WASM execution, and saves the fully rendered HTML for SEO.
  – The Express server (server.js) checks for bot user‑agents and serves the pre‑rendered HTML while human users continue to see the interactive version.

---

## Extending the Project
- **Adding New Node Types:**  
  Extend the `NodeType` enum in `models.rs` and add corresponding drawing logic in `canvas/shapes.rs`.
- **UI Improvements:**  
  Customize UI elements in `ui.rs` or add new interaction modules.
- **Backend Enhancements:**  
  Modify or add new API endpoints in the backend (main.py) to support additional AI models or functionality.
- **Pre‑rendering Enhancements:**  
  Integrate automated pre‑rendering in your CI/CD pipeline and add caching for faster responses.
- **Testing:**  
  Use the included `prerender/test-crawler.sh` script to simulate crawler requests and verify that different content is served.

---

## Testing & Verification
- To simulate bot traffic and verify SEO behavior, run the test script in the `prerender` folder:
   `./test-crawler.sh`
- This script uses `curl` with different user‑agents (e.g. Googlebot vs. a standard browser) and saves the outputs so you can compare them.