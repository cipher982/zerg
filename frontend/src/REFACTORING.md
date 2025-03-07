# Code Refactoring Plan

## Current Structure
We're gradually refactoring the monolithic `ui.rs` file (1300+ lines) into a more modular structure. The goal is to make the codebase more maintainable and easier to extend with new features like the dashboard view.

## New Directory Structure

```
frontend/src/
├── components/              # UI components
│   ├── mod.rs               # Module exports
│   ├── canvas_editor.rs     # Canvas editing functionality
│   ├── model_selector.rs    # Model selection UI
│   └── dashboard.rs         # (Future) Dashboard view
├── ui/                      # UI utilities
│   ├── mod.rs               # Module exports
│   ├── setup.rs             # UI setup functions
│   └── events.rs            # Event handlers
```

## Refactoring Strategy

1. **Phase 1: Extract Components (Current)**
   - Move related functions to appropriate files
   - Keep original ui.rs intact during transition
   - Use re-exports in mod.rs files for compatibility

2. **Phase 2: Complete the Migration**
   - Move all functions from ui.rs to their proper modules
   - Remove redundant code
   - Update imports throughout the codebase

3. **Phase 3: Add Dashboard**
   - Implement a dashboard view
   - Add view switching functionality
   - Implement agent cards

## Components Documentation

### Canvas Editor
Responsible for setting up and managing the canvas where nodes are displayed and edited.

### Model Selector
Handles the model selection dropdown and related functionality.

### UI Setup
Contains functions for creating the base UI elements like headers, status bars, and modals.

### Events
Will contain event handlers for user interactions.

## How to Extend

When adding new functionality, follow these patterns:
1. For new UI elements, create a component in the components/ directory
2. For utility functions, add them to an appropriate module in ui/
3. Update the module exports in the mod.rs files 