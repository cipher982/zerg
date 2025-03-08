# Message / Update / View Refactoring Roadmap

Below is a detailed roadmap for implementing the "Message / Update / View" pattern in our Rust+WASM frontend, with progress updates and next steps.

## Core Principles
- ✅ Create a central "Message" enum to represent all UI events
- ✅ Use an "update" function to change AppState
- ✅ Let view code read from AppState but never mutate it
- ✅ Dispatch messages from event handlers rather than directly manipulating state

## Progress Summary
- ✅ Created messages.rs with Message enum
- ✅ Created update.rs with central update function
- ✅ Added dispatch method to AppState
- ✅ Created views.rs with view functions
- ✅ Refactored all UI event handlers in ui/events.rs to use the new pattern
- ✅ Fixed compiler warnings
- ❌ Still need to refactor canvas interaction code
- ❌ Still need to refactor dashboard code

## Detailed Progress & Next Steps

────────────────────────────────────────────────────────────────
1. ✅ CREATE A NEW "MESSAGES.RS" FILE  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've created the messages.rs file with a comprehensive Message enum that includes:
- View switching (ToggleView)
- Agent management (CreateAgent, EditAgent, DeleteAgent)
- Node manipulation (UpdateNodePosition, AddNode, AddResponseNode)
- Canvas controls (ToggleAutoFit, CenterView, ClearCanvas)
- Input handling (UpdateInputText)
- Dragging state (StartDragging, StopDragging)
- Canvas dragging (StartCanvasDrag, UpdateCanvasDrag, StopCanvasDrag)
- Modal operations (SaveAgentDetails, CloseAgentModal)
- Task operations (SendTaskToAgent)
- Tab switching (SwitchToMainTab, SwitchToHistoryTab)
- Auto-save operations (UpdateSystemInstructions, UpdateAgentName)

**Next Steps:**
- Add more message variants as needed for canvas interactions

────────────────────────────────────────────────────────────────
2. ✅ ADD AN "UPDATE.RS" FILE WITH A CENTRAL UPDATE FUNCTION  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've created update.rs with a comprehensive update function that handles all message variants defined in the Message enum.

**Next Steps:**
- Expand the update function as new message variants are added for canvas interactions

────────────────────────────────────────────────────────────────
3. ✅ CLEAN UP YOUR APPSTATE (STATE.RS)  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've added a dispatch method to AppState that:
- Takes a Message
- Calls the update function
- Handles state saving
- Triggers view rendering

All direct state manipulations have been moved to the update function.

────────────────────────────────────────────────────────────────
4. ✅ DISPATCH MESSAGES FROM EVENT HANDLERS (UI/EVENTS.RS)  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've refactored all event handlers in ui/events.rs to use the new pattern:
- Auto-fit toggle button
- Center view button
- Clear button
- Tab navigation (Dashboard/Canvas switching)
- Create agent button
- Modal close, save, and send buttons
- Modal tab switching
- System instructions auto-save
- Agent name auto-save

**Next Steps:**
- Refactor canvas mouse events in components/canvas_editor.rs
- Refactor dashboard event handlers in components/dashboard.rs

**Important Files to Focus On:**
- frontend/src/components/canvas_editor.rs (especially setup_canvas_mouse_events)
- frontend/src/components/dashboard.rs

────────────────────────────────────────────────────────────────
5. ✅ SEPARATE YOUR "VIEW" CODE FROM THE "UPDATE" CODE  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've created views.rs with functions to render different parts of the UI:
- render_active_view
- render_dashboard_view
- render_canvas_view
- hide_agent_modal
- show_agent_modal

**Next Steps:**
- Add more specialized view functions for canvas interactions

────────────────────────────────────────────────────────────────
6. ✅ DECIDE ON A RENDERING STRATEGY  
────────────────────────────────────────────────────────────────
**COMPLETED**

We've implemented a simple "update, then render" strategy in the AppState::dispatch method.

**Next Steps:**
- Consider optimizing rendering to only update what's changed in later iterations

────────────────────────────────────────────────────────────────
7. 🔄 MIGRATE FEATURES GRADUALLY  
────────────────────────────────────────────────────────────────
**IN PROGRESS**

We've successfully migrated all UI event handlers. Still need to:
1. Refactor canvas mouse interactions
2. Refactor dashboard event handlers, particularly the agent run button

**Next Steps:**
- Focus on canvas_editor.rs next
- Test thoroughly after each feature is migrated

────────────────────────────────────────────────────────────────
8. ❌ OPTIONAL: INTRODUCE A "PROGRAM LOOP"  
────────────────────────────────────────────────────────────────
**NOT STARTED**

We haven't implemented a program loop yet, as we're focusing on migrating existing code first.

**Next Steps:**
- Decide if a program loop would benefit the application
- If yes, implement a simple program loop that handles the update-render cycle

## Known Issues & Considerations

1. **Compiler Warnings**: All compiler warnings have been addressed

2. **Canvas Rendering**: The canvas rendering code is complex and needs careful refactoring:
   - We have multiple draw functions (draw_nodes in state.rs, draw_node in canvas/renderer.rs)
   - We need to ensure consistent parameter ordering and function signatures

3. **Error Handling**: We've improved error handling in the dispatch function, but should review other areas.

4. **Testing**: We should add tests for the new message/update pattern to ensure it works correctly.

5. **Performance**: The current implementation re-renders the entire UI on any state change. We might want to optimize this for larger applications.

## Next Priorities

1. Refactor canvas mouse event handlers to use the Message/Update pattern
2. Refactor dashboard event handlers, particularly the agent run button
3. Address any remaining compiler warnings
4. Consider adding tests for the Message/Update pattern

## Summary of Files & Roles

• **messages.rs** ✅  
  Defines the enum of possible user or system "events."  
  Now includes all necessary message types for UI events.

• **update.rs** ✅  
  Contains the update(AppState, Message) function.  
  Calls small methods on AppState to mutate data.  
  Logic for each event is centralized here.

• **state.rs** ✅  
  Contains AppState itself—the single source of truth.  
  Also includes small helper methods for domain logic.  
  Added dispatch method to handle messages.

• **views.rs** ✅  
  Contains view functions that render the UI based on state.  
  Functions like render_dashboard_view and render_canvas_view.  
  They read from state to build HTML or render canvas.  
  They never mutate the global state directly.

• **ui/events.rs** ✅  
  Fully refactored to dispatch messages instead of mutating state directly.  

• **components/canvas_editor.rs** 🔄  
  Partially refactored, still need to update canvas mouse event handlers.

• **components/dashboard.rs** 🔄  
  Partially refactored, still need to update agent interaction handlers.

This unidirectional approach simplifies debugging, reduces confusion about who's mutating state, and helps reason about concurrency and performance. Whenever something happens in the UI, it goes through a clearly defined path:

(1) Construct Message  
(2) update(state, message)  
(3) Re-render views reading from state