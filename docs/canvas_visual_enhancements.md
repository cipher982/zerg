# Canvas Visual Enhancement Plan

## 🎯 **Primary Goal**

Transform the agent canvas from a static utility into a visually rich, dynamic, and intuitive environment that aligns with the project's futuristic branding. The new design should make the canvas more engaging and provide immediate, at-a-glance insights into the status of the agent workflows.

## 💡 **Inspiration**

The design direction is heavily inspired by the existing branding materials, focusing on a dark, glowing, and data-driven aesthetic. Key references include:

-   **Component Styling & Colors:** `branding/agent_platform_design_system.html`
-   **Status Indicators:** `branding/logo_design_showcase.html`
-   **Animated Backgrounds:** `branding/index.html` & `logo_brand_identity.html`
-   **Connection Line Effects:** `branding/logo_design_advanced.html`

---

## ✅ **Task Checklist & Implementation Plan**

### 1. Implement Dynamic Canvas Background

-   [x] **Task:** Replace the solid background with a subtle, animated particle field.
-   [x] **Implementation:**
    -   Create a new module at `frontend/src/canvas/background.rs` to manage the particle animation logic (structs, updates, and drawing).
    -   In `frontend/src/canvas/renderer.rs`, modify the `draw_nodes` function to call the new background animation on each frame.
    -   Update `frontend/src/canvas/mod.rs` to declare the new `background` module.

### 2. Redesign Agent Nodes

-   [x] **Task:** Update the agent node styling to match the `card` component from the design system, including a glowing border on hover/selection.
-   [x] **Implementation:**
    -   Modify the `NodeType::AgentIdentity` match arm within the `draw_node` function in `frontend/src/canvas/renderer.rs`.
    -   Update the corresponding drawing functions in `frontend/src/canvas/shapes.rs` to render the new card-based design.

### 3. Integrate Rich Status Indicators

-   [x] **Task:** Replace the plain "IDLE" text with rich, visual status indicators (Idle, Active, Success, Error).
-   [x] **Implementation:**
    -   The logic for status-specific colors and icons already exists within the `draw_node` function in `renderer.rs`.
    -   This logic will be enhanced to draw the new visual styles (e.g., pulsing rings, checkmarks) instead of the simple text and pill.

### 4. Animate Connection Lines

-   [ ] **Task:** Upgrade the static connection lines to be animated, showing the direction of data flow.
-   [ ] **Implementation:**
    -   Modify the `draw_connections` function in `frontend/src/canvas/renderer.rs` to use an animated gradient for the stroke style.
    -   **Note:** This was attempted but reverted due to `web-sys` API issues. This will be revisited in the future.

### 5. Continuous Rendering

-   [x] **Task:** Ensure the canvas continuously re-renders to prevent animation artifacts.
-   [x] **Implementation:**
    -   Modified the `AnimationTick` handler in `frontend/src/reducers/canvas.rs` to always mark the canvas as dirty, ensuring a continuous render loop.
