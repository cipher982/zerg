# Agent Debug Modal (Agent Info) – Design Document

> **Status:** Draft  |  Option B (Separate Modals) **selected**

This document captures the evolving design and implementation plan for a new **Agent Debug / Info modal** in the frontend.  It is intended to be _living documentation_: update it as design decisions change, requirements grow, or tasks are completed.

---

## 1  Background & Motivation

The current Dashboard shows only high-level agent metadata (name, schedule, status).  When troubleshooting or optimising an agent we need deeper visibility: system prompt, model parameters, recent runs, token/cost usage, error traces, etc.  Surfacing this information will:

* accelerate debugging and support
* prevent accidental edits when a user “just wants to look”
* pave the way for future observability (charts, per-run logs)

---

## 2  High-Level Solution

* Introduce a **read-only Agent Debug Modal** (a.k.a. _AgentInfoModal_).  
* Keep the existing **Agent Edit Modal** for mutating data.  
* Provide **cross-links** so a user can jump directly between the two modals for the same agent.

Why not a single combined modal?  Separate views keep state management simple (important in our Elm-style architecture) and reduce UX confusion between “observe” and “edit”.

---

## 3  Backend Changes

### 3.1  New REST endpoint

`GET /api/agents/{id}/details`

Query parameters (`include=threads,runs,stats`) allow the client to opt-in to heavier payloads.

```jsonc
{
  "agent": {
    "id": "uuid",
    "name": "…",
    "status": "running|paused|error",
    "schedule": "0 */3 * * *",
    "system_prompt": "You are …",
    "model": "gpt-4o-mini",
    "temperature": 0.7,
    "created_at": "…"
  },
  "threads": [ /* returned only if include=threads */ ],
  "runs":    [ /* returned only if include=runs    */ ],
  "stats":   { /* returned only if include=stats   */ }
}
```

### 3.2  WebSocket additions

Topic `agent:{id}` already emits status updates.  We will extend it to broadcast per-run log events:

```jsonc
{
  "type": "run_log",
  "data": {
    "run_id": "uuid",
    "started_at": "…",
    "duration_ms": 1234,
    "success": true,
    "total_tokens": 543,
    "cost_usd": 0.0123,
    "error": null
  }
}
```

---

## 4  Frontend Architecture

### 4.1  New Messages (frontend/src/messages.rs)

```rust
ShowAgentDebugModal  { agent_id: Uuid }
HideAgentDebugModal
ReceiveAgentDetails  { details: AgentDetails }
ReceiveAgentRunLog   { agent_id: Uuid, log: AgentRunLogEntry }
```

### 4.2  State additions (frontend/src/state.rs)

```rust
pub struct AgentDebugPane {
    pub agent_id: Uuid,
    pub loading: bool,
    pub details: Option<AgentDetails>,
    pub logs: Vec<AgentRunLogEntry>,
    pub active_tab: DebugTab,
}

pub enum DebugTab { Overview, Threads, Runs, RawJson }
```

### 4.3  Command executors (frontend/src/command_executors.rs)

1. `fetch_agent_details(agent_id)` – calls new REST endpoint.
2. `subscribe_agent_topic(agent_id)` / `unsubscribe_agent_topic` – WebSocket management.

### 4.4  Component tree

```
components/
  └─ agent_debug_modal.rs   (new)
```

*Initial tab set*

| Tab      | Notes                                                |
|----------|------------------------------------------------------|
| Overview | name, status badge, schedule, model, temperature     |
| Raw JSON | pretty-printed full payload                          |

Later phases add **Threads** and **Runs / Logs** tabs.

### 4.5  Dashboard integration

Add a 🐞 “Debug” icon/button beside the existing ✎ “Edit” button.

---

## 5  Implementation Phases & Task List

> Mark tasks with ✅ when merged into `main`.

### Phase 0  – Preparation

- [ ] Decide file names / folder placement (✔ agreed in this doc)

### Phase 1  – Minimal Vertical Slice

- [ ] **Backend**: implement `/details` endpoint (overview only)
- [ ] **Frontend**: introduce messages, state, command executor stubs
- [ ] **Component**: create `AgentDebugModal` with Overview & Raw JSON tabs
- [ ] **Dashboard**: add “Debug” button → ShowAgentDebugModal
- [ ] **Linking**: in Debug modal add “Edit Agent” button (deep-link)

### Phase 2  – Threads Tab

- [ ] Backend: add `threads` include + DTO
- [ ] Frontend: render Threads table

### Phase 3  – Runs / Logs Tab (live updating)

- [ ] Persist run results (#backend)
- [ ] Publish `run_log` WebSocket events
- [ ] Frontend: display log stream & basic charts

### Phase 4  – Cross-link from Edit modal

- [ ] Add “View Debug Info” button inside `AgentEditModal`

---

## 6  Open Questions

1. **Pagination / limits** — how many runs & threads to load by default?
2. **Access control** — will the debug view be restricted to certain roles later?
3. **Mobile UX** — should the modal be full-screen on small devices?

---

## 7  Changelog

| Date       | Author   | Notes                              |
|------------|----------|------------------------------------|
| 2025-04-25 | chatGPT  | Initial draft committed            |

---

_Feel free to edit, extend, and tick off tasks as we ship!_
