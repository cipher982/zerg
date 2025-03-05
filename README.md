# ZERG

## Overview

Agent Platform Frontend (ZERG) is a WebAssembly (WASM) project built in Rust that provides an interactive, node-based interface for communicating with an AI backend. Users enter text, which is rendered as nodes on a canvas, and receive corresponding AI responses in real time. Each node is graphically represented – user inputs are shown as rounded rectangles and AI responses as thought bubbles with dynamic, curved connections.

ZERG combines modern WASM technologies on the frontend and a Python FastAPI backend that leverages the OpenAI API. The result is an engaging, real-time interface where graphical nodes illustrate a conversation between a user and an AI.

## Key Features

- **Interactive Canvas UI:**  
  Display nodes that represent user input and AI responses. The nodes are drawn on a canvas with rounded shapes and are connected using animated, curved lines with arrowheads.

- **Dynamic Node Creation and Connection:**  
  When a user submits text, a new node is created on the canvas. Once the backend processes the text and returns an AI response, a connected response node is generated automatically.

- **Real-Time Updates:**  
  Utilizes both HTTP (Fetch API) and WebSocket communications. The user text is sent over HTTP while the AI responses are pushed in real time via a WebSocket connection.

- **Drag & Drop Support:**  
  Users can reposition nodes on the canvas using mouse drag events for a customized visual layout.

- **Responsive and Adaptive Rendering:**  
  The canvas resizes dynamically based on browser window adjustments and high-DPI displays. Auto-fit functionality ensures that all nodes stay visible.

- **Model Selection:**  
  A dropdown enables choosing among different AI models. The default model is “GPT-4o” but additional models can be fetched from the backend.

- **Modular Code Structure:**  
  The codebase is clearly divided into backend and frontend:
  - **Backend (Python with FastAPI):**
    - Processes incoming text requests.
    - Uses the OpenAI API to create chat completions.
    - Maintains WebSocket connections to broadcast AI responses.
    - Provides endpoints for health checking and model listing.
  - **Frontend (Rust & WASM):**
    - **models.rs:** Defines core data structures such as Node and NodeType.
    - **state.rs:** Maintains application state (nodes, viewport, etc.).
    - **canvas/renderer.rs & canvas/shapes.rs:** Implements the drawing logic for nodes, curves, and arrows.
    - **ui.rs:** Sets up HTML elements, event handlers, and interaction logic.
    - **network.rs:** Manages WebSocket connections and asynchronous HTTP requests.
    - **favicon.rs:** Dynamically generates a favicon using the canvas.
    - A helper script (`build.sh`) compiles the Rust code to WASM and launches a local web server.

## Directory Structure

```
.
├── LICENSE
├── README.md
├── backend
│   ├── main.py                 # FastAPI backend using OpenAI API
│   ├── pyproject.toml
│   ├── requirements.txt
│   └── src
│       └── __init__.py
└── frontend
    ├── Cargo.toml              # Rust project manifest for WASM build
    ├── build.sh                # Build script that uses wasm-pack and launches a server
    ├── src
    │   ├── canvas              # Modules for drawing nodes and connections
    │   │   ├── mod.rs
    │   │   ├── renderer.rs
    │   │   └── shapes.rs
    │   ├── favicon.rs          # Dynamic favicon generation using canvas
    │   ├── lib.rs              # Entry point for the WASM application
    │   ├── models.rs           # Data models for node structures
    │   ├── network.rs          # WebSocket and HTTP networking code
    │   ├── state.rs            # Global state management for nodes and viewport
    │   └── ui.rs               # UI setup and event handling for user interactions
    ├── target                  # Build output including compiled WASM files
    └── www                     # Generated web assets (HTML, JS, WASM)
```

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- A modern web browser that supports WebAssembly
- Python 3.12 or later (for the backend)

### Building and Running the Project

1. **Frontend Setup:**

   - Install wasm-pack using:
     ```bash
     cargo install wasm-pack
     ```
   - Build the WASM module by running the build script:
     ```bash
     ./build.sh
     ```
   - This script compiles your Rust code to WASM, outputs assets in the `www` directory, and starts a web server (default port 8002).

2. **Backend Setup:**

   - Create a `.env` file in the `backend` directory based on the provided `.env.example` and set your OpenAI API key.
   - Install Python dependencies using:
     ```bash
     pip install -r backend/requirements.txt
     ```
   - Run the FastAPI backend:
     ```bash
     uvicorn backend.main:app --host 0.0.0.0 --port 8001
     ```

3. **Open in Browser:**

   - Open your web browser and navigate to [http://localhost:8002](http://localhost:8002) to interact with the application.

## How It Works

- **User Input and Node Creation:**  
  The UI provides an input field and a “Send to AI” button. When text is entered, a user input node is created on the canvas and simultaneously sent to the backend for processing.

- **Backend Processing and AI Responses:**  
  The backend receives the text, processes it using the OpenAI API, and returns a response. The response is broadcast to all connected clients via WebSocket. In the UI, the appropriate response node is created and linked to the initial user node.

- **Canvas Rendering and Interaction:**  
  Custom drawing functions render nodes as dynamic, interactive elements. Nodes can be dragged to reposition and the canvas viewport auto-adjusts using an auto-fit feature.

- **Networking and Model Selection:**  
  The application supports dynamic AI model selection via a dropdown that fetches available models from the backend. HTTP requests and WebSocket messages handle data transfer between the frontend and backend seamlessly.