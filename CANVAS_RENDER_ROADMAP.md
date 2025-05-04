# Canvas Refactor & Performance Roadmap

Audience: **new frontend engineer** joining the project to finish the canvas-performance
refactor and related quality-of-life tasks.  Everything you need – context,
file paths, call-stacks – is collected here so you can ramp up in minutes.

> Status @ 2025-05-04: the initial “single requestAnimationFrame renderer” is
> merged.  The canvas now repaints only when `AppState.dirty == true`.  All
> *direct* calls to `draw_nodes()` were removed from reducers, and the RAF
> loop performs the paint + clears the flag.

---

## 1 · Quick mental model

```
                         (pointer events)
<canvas> ──► canvas_editor.rs  ──► Message enum  ──► update.rs reducer
                                           │                       │
                                           │ mark_dirty()          │
                                           ▼                       │
                                  AppState { dirty: bool, … }      │
                                           │                       │
  requestAnimationFrame loop ◄─────────────┘                       │
                         │                                        │
                         ▼                                        ▼
                    draw_nodes()                         Command / Effects
```

Important invariants:

* Only the reducer mutates `AppState`.
* Rendering happens **exclusively** inside `ui/mod.rs::setup_animation_loop()`.

---

## 1.1 · Progress snapshot (updated 2025-05-04)

P1 (debounced save-state) and P3 (dirty-flag cleanup) are merged.  The RAF
loop now performs both rendering _and_ the debounce check that dispatches the
new `Command::SaveState`.  All helper functions outside the reducer trigger
repaints via the `MarkCanvasDirty` message.


## 2 · Remaining pain-points  (value ✓ / effort ⧗)

| ID | Title | Value | Effort | Status |
|----|-------|-------|--------|--------|
| P1 | Debounce `save_state_to_api()` | ★★★★★ | ★★☆☆☆ | ✅ **shipped (2025-05-04)** |
| P2 | Replace view mount/unmount with CSS toggling | ★★★★☆ | ★★☆☆☆ | ⏩ **next** |
| P3 | Remove stray `mark_dirty()` calls in helpers | ★★☆☆☆ | ☆☆☆☆☆ | ✅ **shipped (2025-05-04)** |
| P4 | `debug_log!` macro + on-screen ring-buffer | ★★☆☆☆ | ★☆☆☆☆ | 🔜 week-3 |
| P5 | Batch “layout” PATCH endpoint (frontend + backend) | ★★★★☆ | ★★★☆☆ | 🔜 week-4 |


## 3 · File guide for each task

### P1 · Debounced persistence

* **frontend/src/storage.rs** – `save_state_to_api()` currently writes
  localStorage immediately and sets `state_modified`.
* **frontend/src/update.rs** – many reducers flip `state.state_modified = true;`.

Implementation sketch:
1. Add `last_modified_ms: u64` to `AppState`.
2. After each reducer sets `state_modified = true` also store `now_ms()`.
3. In RAF loop (after drawing) check:
   ```rust
   if state.state_modified && now_ms() - state.last_modified_ms > 400 {
       Command::SaveState // already exists
   }
   ```

### P2 · Visibility toggle

* **frontend/src/views.rs::render_active_view_by_type** – heavy DOM rebuild.
* **frontend/www/index.html** – ensure each view has its root `<div>`.

Steps:
1. Add `id="view-dashboard"` etc. to HTML.
2. Reducer for `ToggleView` sets CSS `display:none` instead of re-mounting.

### P3 · Stray dirty flags

* Search codebase for `mark_dirty()` outside `update.rs`.  Replace with
  `dispatch_global_message(Message::MarkCanvasDirty)`.  (Some already done.)

### P4 · Debug helper

* New file **frontend/src/utils/debug.rs** with macro:
  ```rust
  #[macro_export]
  macro_rules! debug_log { ($($t:tt)*) => {
      #[cfg(debug_assertions)]
      web_sys::console::log_1(&format!($($t)*).into());
  }}
  ```
* Optionally push entries into `AppState.debug_ring: VecDeque<String>` and
  draw overlay in RAF.

### P5 · Layout PATCH batching

* **backend/zerg/routers/graph_layout.py** (to be created).
* **frontend/src/network/api_client.rs** – new `patch_layout(payload)`.
* **frontend/src/storage.rs** – when debounce fires, send one payload with
  `{ nodes: {id: {x,y}}, viewport: {x,y,zoom} }`.


## 4 · Important structs & functions

```
// AppState (frontend/src/state.rs)
dirty: bool                // repaint flag
state_modified: bool       // persistence flag

// Entry point: ui/mod.rs::setup_animation_loop()
if state.dirty { draw_nodes(); state.dirty=false; }

// Reducer helper
fn mark_dirty(&mut self)

// Storage
save_state(app_state)              // wraps save_state_to_api()

// Canvas event handler
components/canvas_editor.rs        // converts DOM events → Messages
```


## 5 · Suggested week-by-week schedule

| Week | Goals |
|------|-------|
| 1 | **DONE** – P1 (debounced save) + P3 (dirty-flag cleanup) |
| 2 | P2 (CSS visibility toggle for views) |
| 3 | P4 (`debug_log!` macro + overlay UI) + create backend layout endpoint stub |
| 4 | P5 Batch-save payload: frontend caller, backend PATCH handler, WS broadcast |


---

### Onboarding checklist for new dev

1. `./build-debug.sh`, open http://localhost:8002 and verify you can drag, pan & zoom.  Console should show RAF debug msg (`debug_log!`) only in debug build.
2. Run `cargo check` after each change – no warnings allowed for new code.
3. Follow the Message/Command architecture: *never* mutate `AppState` from a component.

Welcome aboard – happy hacking!
