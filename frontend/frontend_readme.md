# AI Agent Platform Frontend Overview

This document provides a high-level explanation of how the frontend of the “AI Agent Platform” is structured, how it communicates with the backend Python services, and how a user typically creates and interacts with agents through the web interface.

## Table of Contents
1. Introduction
2. Application Structure
3. Navigation and Views
   - Dashboard View
   - Canvas View
   - Chat/Thread View
4. Creating and Managing Agents
5. Interacting with Agents (Running Tasks)
6. Threads and Chat Interface
7. API Routes (High-Level) and Frontend Communication
8. Summary of Workflow

---

## 1. Introduction

The “AI Agent Platform” is a web-based interface that allows users to create, manage, and interact with AI-driven agents. These agents can be configured with system instructions, task instructions, and can maintain multiple “threads” for conversation-like interactions.

This frontend is written in Rust (compiled to WebAssembly). It communicates with a Python FastAPI backend, which handles agent management, database storage, and AI processing.

---

## 2. Application Structure

The frontend consists of several logical modules:

• A “Dashboard” module that displays a list of agents.  
• A “Canvas” module that visually represents nodes (agents and other nodes).  
• A “Chat” (aka “Thread”) module that shows chat threads and allows message-based communication with an agent.

Key directories and files:

• src/  
  ├─ components/ (UI modules such as dashboard, chat_view, etc.)  
  ├─ network/ (HTTP & WebSocket code)  
  ├─ state.rs (Central application state)  
  ├─ storage.rs (Local & remote state persistence)  
  ├─ update.rs (Core “message” reducer function)  
  └─ views.rs (Functions that render the different “views”: Dashboard, Canvas, Chat)

• www/  
  ├─ index.html (Main HTML entry point)  
  ├─ styles.css, chat.css (Styling)  
  └─ index.js (Generated JavaScript that bootstraps the WASM)

---

## 3. Navigation and Views

### 3.1 Dashboard View

When the user first loads the app, they typically see the “Dashboard” containing:

• A search bar to look up agents.  
• A “Create Agent” button.  
• A table of existing agents, showing status, last run, success rate, etc.  
• Action buttons beside each agent (Run, Edit, Chat, etc.).  

This view is managed by the “dashboard.rs” module under src/components/.

### 3.2 Canvas View

The “Canvas Editor” is a visual environment for adding, deleting, and connecting “nodes.” Nodes can represent agents, user input boxes, or other content. These nodes are rendered with HTML canvas (in src/canvas/).

Navigation to the Canvas happens via a top tab labeled “Canvas Editor.” The user can manipulate nodes, pan around, and zoom in/out while the system stores (and automatically saves) node data in both local storage and the backend.

### 3.3 Chat/Thread View

When a user selects “Chat with Agent,” the UI navigates to the “Chat View,” displaying:

• A sidebar of threads for that agent.  
• A large message area showing back-and-forth conversation.  
• An input field to type new messages.  

In the background, a WebSocket connection may be created for real-time messaging events.

---

## 4. Creating and Managing Agents

Below is the standard user flow for adding a new agent:

1. The user navigates to the Dashboard.
2. Clicks the “Create Agent” button (or “+ Create Agent” in the UI).
3. A new agent record is created in the backend via an HTTP POST to /api/agents. The frontend obtains an ID from the backend.
4. The agent is shown in the list, and if desired, a corresponding “node” is created in the Canvas.

Editing an existing agent:

• The user clicks the “Edit” button from the dashboard list or “Edit Agent” button.  
• A modal dialog appears with fields (Agent Name, System Instructions, etc.).  
• Upon clicking “Save,” the agent is updated in the backend (PUT /api/agents/{id}).

Deleting an agent:

• The user can click the “More” button (⋮) or “Delete” on the dashboard.  
• This triggers a DELETE /api/agents/{id} call to remove it from the backend.  
• The dashboard then refreshes the list of agents.

---

## 5. Interacting with Agents (Running Tasks)

To “run” an agent:

1. On the dashboard, each agent row has a “Run” button (▶).  
2. Clicking Run calls POST /api/agents/{id}/run on the backend, which schedules or immediately executes the agent’s task.  
3. The agent’s status changes to “processing” (or similar). WebSockets or periodic refresh can update the UI.

Alternatively, in the Canvas view, you may see a context menu or action button on an agent’s node that sends tasks to be processed.

---

## 6. Threads and Chat Interface

• Each agent can have one or more “threads,” representing separate conversation contexts.  
• The user navigates to the chat view by clicking “Chat” on the dashboard’s agent row.  
• A sidebar lists threads for that agent. The user may:
  - Create new threads.
  - Switch between existing threads.
• Messages are loaded from /api/threads/{id}/messages.  
• Sending a new message triggers a POST /api/threads/{id}/messages or a WebSocket flow.  
• The message is displayed optimistically in the UI, then updated when the server responds.

---

## 7. API Routes (High-Level) and Frontend Communication

Below is a quick summary of the main routes the frontend calls:

1. GET /api/agents  
   - Loads the list of agents for the dashboard.

2. POST /api/agents  
   - Creates a new agent with name, instructions, etc.

3. PUT /api/agents/{id}  
   - Updates an agent’s attributes.

4. DELETE /api/agents/{id}  
   - Removes an agent from the system.

5. POST /api/agents/{id}/run  
   - Triggers an agent to run a background task.

6. GET /api/threads?agent_id={id}  
   - Lists all threads for a particular agent.

7. GET /api/threads/{id}/messages  
   - Loads messages from a single thread for chat.

8. POST /api/threads/{id}/messages  
   - Appends a new message to a thread.

Additionally, there is a WebSocket endpoint /api/ws (or /api/ws?thread_id=...) that pushes agent/threads updates to the client in real time.

---

## 8. Summary of Workflow

1. The user opens the app and sees the Dashboard (ActiveView::Dashboard).  
2. From the Dashboard, they either:
   - Create a new agent, or  
   - View an existing agent’s data, or  
   - Click “Canvas Editor” to switch to a visual node-based layout.  
3. Agents can be run (triggering the backend to do some processing).  
4. Users can open Chat to handle multi-message threads between user and AI.  
5. Under the hood, the frontend uses a unified message-based state management system:
   - dispatch(Message::X) → update.rs → modifies AppState → triggers UI refresh.  
6. The backend is updated via fetch calls (API client in src/network/api_client.rs) or WebSocket messages (src/network/ws_client.rs).  
7. The UI updates automatically to reflect agent states, new threads, and messages.

That concludes the high-level overview of how the frontend operates and interacts with the backend.