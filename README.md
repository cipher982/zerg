# AI Agent Platform

A proof-of-concept platform for visualizing and interacting with AI agents using Python/FastAPI and Rust/WebAssembly.

## Features

- Text input field to send prompts to an AI model
- Canvas-based visualization of prompts and responses
- WebSocket for real-time updates
- Draggable nodes for organizing your agent interactions

## Project Structure

- **Backend**: Python FastAPI server with OpenAI integration
- **Frontend**: Rust WebAssembly application for interactive visualization

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) and [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- Python 3.8+ and [pip](https://pip.pypa.io/en/stable/installation/)
- An [OpenAI API key](https://platform.openai.com/api-keys)

## Setup and Running

### Backend Setup

1. Create a `.env` file in the `backend` directory by copying the example:
   ```bash
   cd backend
   cp .env.example .env
   ```

2. Edit the `.env` file to add your OpenAI API key

3. Install the Python dependencies:
   ```bash
   pip install -r requirements.txt
   ```

4. Start the backend server:
   ```bash
   python main.py
   ```

### Frontend Setup

1. Build the WebAssembly module:
   ```bash
   cd frontend
   ./build.sh
   ```

2. Start a simple HTTP server to serve the frontend:
   ```bash
   cd frontend/www
   python -m http.server
   ```

3. Open your browser and navigate to http://localhost:8000

## How It Works

1. Type a message in the input field and click "Send to AI"
2. Your message is displayed as a node on the canvas
3. The message is sent to the backend, which forwards it to OpenAI
4. The AI's response is sent back via WebSocket
5. The response appears as a connected node on the canvas

## Future Enhancements

- Save/load agent configurations
- Multiple agent types with different capabilities
- Visual flow builder for connecting agents
- Historical data viewing and analysis 