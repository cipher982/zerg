# ZERG

## Overview

Agent Platform Frontend (ZERG) is a WebAssembly (WASM) project built in Rust that provides an interactive, node-based interface for communicating with an AI backend. Users can enter text, which is rendered as nodes on a canvas, and receive corresponding AI responses in real time. Each node is graphically represented either as a rounded rectangle (for user inputs) or a thought bubble (for AI responses). Nodes are connected by dynamically drawn curved lines with arrowheads.

## Features

- **Interactive Canvas UI:**  
  Displays nodes for user inputs and AI responses. Nodes can be visually connected with animated curves.

- **Dynamic Node Creation:**  
  User-submitted text is automatically added as a node on the canvas. When a response is received from the backend over a WebSocket, a connected response node is generated.

- **Drag & Drop:**  
  Nodes on the canvas can be repositioned using mouse drag events for customized layout.

- **Responsive and Adaptive Rendering:**  
  Automatically handles window resizing and scales correctly on high-DPI displays.

- **WebSocket & HTTP Communication:**  
  Utilizes a WebSocket connection to receive real-time AI responses and the Fetch API for sending user input to the backend.

- **Modular Codebase:**  
  Designed with clear separation of concerns:
  - **models.rs:** Data structures for node representation.
  - **state.rs:** Global application state management.
  - **canvas/renderer.rs & canvas/shapes.rs:** Drawing logic for nodes and connections.
  - **ui.rs:** UI element creation and event handling.
  - **network.rs:** Setup of WebSocket and Fetch communication.

## Directory Structure

```
.
├── Cargo.toml               # Rust project manifest
├── build.sh                 # Shell script to build the WASM module using wasm-pack
├── src
│   ├── canvas               # Contains canvas drawing modules
│   │   ├── mod.rs           # Module declaration for canvas
│   │   ├── renderer.rs      # High-level rendering logic for nodes/connections
│   │   └── shapes.rs        # Helper functions to draw different shapes
│   ├── lib.rs               # Main WASM entry point
│   ├── models.rs            # Data models (Node and NodeType definitions)
│   ├── network.rs           # Networking code (WebSocket and HTTP requests)
│   ├── state.rs             # Application state management
│   └── ui.rs                # Dynamic UI setup and event handlers
├── target                   # Build output including compiled WASM files
└── www                      # Directory for generated web assets (HTML, JS, WASM)
```

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- A modern web browser with WebAssembly support.

### Building and Running

1. **Install wasm-pack** (if not already installed):

   ```bash
   cargo install wasm-pack
   ```

2. **Build the WASM Module:**

   Run the included build script in your terminal:

   ```bash
   ./build.sh
   ```

   This command will compile the project to WebAssembly, output necessary assets in the `www` folder, and launch a simple HTTP server (default on port 8002).

3. **Open in Browser:**

   Navigate to [http://localhost:8002](http://localhost:8002) in your web browser to access the application.

## How It Works

- **User Input:**  
  An input field and a “Send to AI” button allow the user to enter text. When the text is submitted, a new node is created on the canvas.
  
- **Drawing Nodes:**  
  The canvas is used as the visual playground where each node is rendered using custom drawing functions. The drawing logic handles shapes, text wrapping, and connections between nodes.

- **Networking:**  
  The project establishes a WebSocket connection to receive responses asynchronously. Additionally, text is sent to the backend via an HTTP POST request.
  
- **Interaction:**  
  Nodes can be repositioned via mouse dragging. The state is updated and the canvas is re-rendered accordingly.

## Customization & Future Enhancements

- **Additional Node Types:**  
  Currently, the app supports UserInput, AgentResponse, and a placeholder for AgentIdentity. Future enhancements could include more distinctive shapes or additional node types.
  
- **Improved Responsiveness:**  
  Enhance text measurement and dynamic sizing for better UI scaling.
  
- **Backend Integration:**  
  This front-end is designed to work with an AI backend service that sends responses via WebSocket and processes input via HTTP requests.