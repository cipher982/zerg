# Rust File Analysis
Here's a list of the Rust files in your src directory and their apparent purpose:

src/mod.rs: Root module file for the src directory. Based on the provided content (empty), it's not currently declaring sub-modules or re-exporting anything significant. It might be a placeholder or leftover.
src/renderer.rs: (Top-level) Seems like an older version for drawing nodes and connections on the canvas. Imports shapes.
src/shapes.rs: (Top-level) Seems like an older version containing functions to draw specific geometric shapes (rectangles, arrows, etc.) for the canvas.
src/canvas_editor.rs: (Top-level) Looks like an older or misplaced file for handling canvas setup, events (mouse, resize), and dispatching canvas-related messages (dragging, zooming).
src/components/canvas_editor.rs: Handles setup of the HTML Canvas, event listeners (mouse down/move/up/wheel, resize), dispatches canvas-related messages (StartDragging, UpdateNodePosition, ZoomCanvas, etc.), and interacts with AppState for canvas state. This seems like the active canvas interaction logic file based on its location.
src/components/chat/mod.rs: Module file for the chat component directory. Exports the WebSocket manager.
src/components/chat/ws_manager.rs: Manages WebSocket subscriptions and message handling specifically for the Chat View, interacting with the global TopicManager. It handles thread:* topics and dispatches messages like ReceiveThreadHistory, ReceiveNewMessage, ReceiveThreadUpdate.
src/components/chat_view.rs: Responsible for setting up the DOM structure for the chat view, handling its specific UI events (back button, send message, new thread), and providing functions to update the chat UI (agent info, thread list, conversation messages).
src/components/dashboard.rs.bak: A backup file, clearly not in active use.
src/components/dashboard/mod.rs: Defines the dashboard view. It includes rendering the agent table, handling dashboard-specific actions (create agent, run, edit, chat, reset DB), interacting with the API client, and managing the dashboard's WebSocket connection via its ws_manager. Defines a local Agent struct likely for display purposes.
src/components/dashboard/ws_manager.rs: Manages WebSocket subscriptions for the Dashboard view, specifically listening for agent events (agent:* topic) and triggering refreshes. Interacts with the global TopicManager.
src/components/dashboard/ws_manager_test.rs: Contains unit/integration tests for the DashboardWsManager, using mock implementations for dependencies.
src/components/mod.rs: Module file for the components directory, likely declaring the sub-modules (chat, dashboard, etc.).
src/components/model_selector.rs: Handles the UI and logic for the AI model selection dropdown, including fetching available models from the backend and updating the application state.
src/constants.rs: Defines shared constant values used across the application (default names, instructions, colors, dimensions, etc.).
src/lib.rs: The main entry point of the WASM library (#[wasm_bindgen(start)]). Initializes the application, sets up the base UI, connects the WebSocket (V2), initializes state, sets up navigation, and starts timers (like auto-save).
src/messages.rs: Defines the central Message enum representing all possible actions/events that can modify the application state, and the Command enum for handling side effects.
src/models.rs: Defines the core data structures used throughout the application, including Node, NodeType, ApiAgent, ApiThread, ApiThreadMessage, Workflow, etc. Distinguishes between backend API models and frontend visual models.
src/network/api_client.rs: Implements the ApiClient struct for making HTTP REST requests to the backend API (fetching/creating/updating/deleting agents, threads, messages). Includes helper functions like load_agents.
src/network/event_types.rs: Defines enums (MessageType, EventType) and helper functions (topics) related to WebSocket message/event types and topic naming conventions.
src/network/messages.rs: Defines the specific structures for various WebSocket message payloads (e.g., PingMessage, SubscribeMessage, ThreadHistoryMessage, StreamChunkMessage, event data payloads).
src/network/mod.rs: Module file for the network directory. Re-exports key network components and provides helper functions like get_api_base_url.
src/network/topic_manager.rs: Implements the TopicManager and ITopicManager trait. Manages WebSocket topic subscriptions (subscribe/unsubscribe) and routes incoming WebSocket messages to the correct handlers based on topic. Works with IWsClient.
src/network/ui_updates.rs: Contains utility functions specifically for updating small parts of the UI related to network activity (connection status indicator, packet flash).
src/network/ws_client.rs: An older WebSocket client implementation. Appears to handle connection, reconnection, and message processing directly, tightly coupled with AppState.
src/network/ws_client_v2.rs: The newer WebSocket client (WsClientV2) implementing the IWsClient trait. Focuses on connection management, pinging, and reconnection logic, delegating message routing/handling via callbacks (used by TopicManager).
src/state.rs: Defines the global AppState struct, the APP_STATE thread-local static variable, methods for modifying state (like adding nodes, drawing, fitting view), saving/loading helpers, and the dispatch_global_message function.
src/storage.rs: Handles persistence logic. Includes functions to save/load state to/from localStorage (legacy nodes, viewport, workflows) and interacts with the ApiClient to save/load state (agents, messages) from the backend API. Defines ActiveView.
src/thread_handlers.rs: Contains specific logic for handling chat message sending, including optimistic UI updates and processing server responses/failures for thread messages.
src/ui/events.rs: Sets up various UI event listeners (buttons, toggles, modals) that dispatch Messages to the state management system.
src/ui/main.rs: Responsible for creating the main UI elements within the app-container, like the input panel, canvas container, buttons, and dropdowns.
src/ui/mod.rs: Module file for the ui directory. Also includes the setup_animation_loop function using requestAnimationFrame for canvas drawing updates.
src/ui/modals.rs: Contains functions specifically for controlling the agent modal dialog (opening, closing, populating data).
src/ui/setup.rs: Creates the initial, static parts of the UI structure like the header, status bar, and the basic modal structure.
src/update.rs: Contains the core update function, which acts as the state reducer, handling incoming Messages, modifying the AppState, and returning Commands for side effects.
src/views.rs: Provides functions (render_active_view, render_dashboard_view, render_canvas_view) to manage which major UI view (Dashboard, Canvas, Chat) is currently displayed by showing/hiding the relevant containers.