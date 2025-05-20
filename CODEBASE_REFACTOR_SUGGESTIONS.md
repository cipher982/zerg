# 📌 Codebase Refactor & Optimisation Notes

This document is a **living checklist** of hot-spots and quick-wins discovered
during an exploratory review (May 2025).  Nothing here is *broken* – the
project already ships a healthy test-suite and >95 % coverage – but addressing
these points will pay back quickly in maintainability, performance and
operational robustness.

> The sections roughly follow the runtime flow: DB → EventBus/WS → Scheduler →
> Agent execution → Front-end, finishing with cross-cutting concerns & a
> bite-size “pick-me-first” list.

---

## 1&nbsp;· Database / CRUD layer

| Area | Issue | Suggested Improvement |
|------|-------|-----------------------|
| **Hot commit loops** | `crud.create_thread_message`, `mark_message_processed`, `ThreadService.save_new_messages` each call **`commit()` for every row**. The quadratic sync-flush is a major test-suite slow-down. | Collect all new rows → `session.flush()` once → single `commit()`.  Provide `ThreadService.mark_messages_processed(ids)` that runs a **bulk UPDATE**. |
| **Sync SQL inside async routes** | All FastAPI endpoints call the synchronous SA API which blocks the event-loop. | Either migrate to `sqlalchemy.ext.asyncio`, or wrap heavy CRUD in `anyio.to_thread`. |
| **Session lifetime workaround** | `SchedulerService.load_scheduled_agents` keeps the session open to avoid `DetachedInstanceError`. | Close the session and return _tuples_ instead of ORM rows (e.g. `(id, cron_expr)`). |
| **Magic strings** | Status (`"idle"`…), roles (`"assistant"`…), run triggers are free text. | Introduce small `Enum`s (Python + DB CHECK) to catch typos at commit-time. |

---

## 2&nbsp;· Event / WebSocket layer

1. **Thread-safety** – `EventBus._subscribers` and every map in
   `TopicConnectionManager` are mutated by multiple coroutines. Add an
   `asyncio.Lock` or migrate to an actor model (`anyio` TaskGroup + channel).

2. **Sequential fan-out** – `EventBus.publish()` awaits each subscriber in a
   for-loop; one slow handler blocks all others. Change to

   ```python
   await asyncio.gather(*callbacks, return_exceptions=True)
   ```

3. **Zombie sockets** – dead clients are only culled when the *next* broadcast
   to the same topic raises an exception. Consider a periodic
   ping/timeout task.

---

## 3&nbsp;· Scheduler / long-running tasks

* **Bad cron strings crash startup** – ⬤ **Partial** –
  `SchedulerService.schedule_agent()` now wraps `CronTrigger.from_crontab()` in
  `try/except`, preventing service-level crashes, **but** `crud.update_agent`
  still lets invalid cron expressions reach the database.  Add the same
  validation in the write-path and surface a `422` to the client.

* **DB work in scheduler thread** – writes from the APScheduler thread share
  the same synchronous engine. When moving to Postgres use a separate async
  engine or thread-pool.

---

## 4&nbsp;· Agent execution pipeline

| Issue | Detail |
|-------|--------|
| **Tool calls claimed “parallel”** | ✅ **Fixed (May 2025)** – `zerg_react_agent.py` now awaits `asyncio.gather(...)` inside the loop and wraps each blocking tool call in `asyncio.to_thread`. |
| **Graph recompilation** | The LangGraph runnable is rebuilt every time an `AgentRunner` instance is created.  A `@lru_cache` keyed by `(agent_id, model_name, stream_flag)` would shave ~100 ms/run. |
| **Env flag re-parsing** | ⬤ **Partial** – `AgentRunner` now caches the environment lookup once at init-time, but helpers like `_make_llm()` and several test utilities still call `os.getenv()` on every invocation.  Move the flag to `zerg.constants` and import it. |

---

## 5&nbsp;· ThreadService niceties

* Convert `_db_to_langchain` & `_langchain_to_create_kwargs` to
  `functools.singledispatch` for clarity.
* Currently logs “unknown role” then falls back to `AIMessage`; raise in tests
  so new roles are introduced intentionally.

---

## 6&nbsp;· Frontend (Rust / WASM)

1.  **Duplicate logic** – `TopicManager` has near identical `subscribe` /
    `unsubscribe` code in the impl block *and* the trait impl.  Collapse into
    one.

2.  **`RefCell` ergonomics** – many handlers use `try_borrow_mut()` and ignore
    the error; almost all borrows are exclusive and deterministic, so switch
    to `borrow_mut()` and let a panic surface real bugs.

3.  **Exp-backoff leak** – `WsClientV2` clones self to schedule reconnects but
    does not cancel the timer after a successful reconnect.  Store the
    `TimeoutId` and clear it.

---

## 7&nbsp;· Cross-cutting

* **Timezone** – every timestamp is naïve.  Return UTC ISO-8601 (`datetime.now(tz=UTC)`)
  from `get_current_time()` and add `pytz`/`pendulum` to deps.
* **CORS** – wildcard in dev is fine; honour `AUTH_DISABLED=0` by limiting
  origins in production.
* **Webhook hardening** – clamp body size (e.g. 128 KB) before HMAC validation.

---

## 🍬 Low-hanging fruit (pick-me-first)

1. **Batch DB commits** in `ThreadService.save_new_messages` & friends.
2. Make `EventBus.publish` concurrent via `asyncio.gather`.
3. Protect shared maps in `TopicConnectionManager` with a lock.
4. Cache compiled LangGraph runnables with `@lru_cache`.
5. ~~Replace serial tool loop with `asyncio.gather`.~~  **Done in PR #??? – May 2025**

Everything above is covered by unit tests, so each change can land in an
independent PR with high confidence.

---

## 8 · New surface-area since the original review (May 2025)

The code base has grown significantly over the last months.  The following
features are **production-ready** but introduce fresh refactor opportunities
that are *not* captured in the sections above:

| Feature | Potential follow-ups |
|---------|----------------------|
| **Google Sign-In auth layer** | • Pass the JWT through the WebSocket handshake (today the WS is unauthenticated in prod).<br>• Restrict CORS *only* when `AUTH_DISABLED=0`; keep wildcard for local dev. |
| **Run-history tables & API** | • Add DB indexes on `(agent_id, created_at DESC)` – large tenants already show slow list queries.<br>• Expose aggregate cost metrics in a separate endpoint instead of computing them in the dashboard. |
| **HMAC-secured webhook triggers** | • Clamp request body size (≤128 KB) *before* HMAC validation.<br>• Rotate `TRIGGER_SIGNING_SECRET` via admin UI. |
| **Token-level streaming** | • Unify the three chunk types (`assistant_token`, `tool_output`, `assistant_message`) in a small `Enum` on both client & server to avoid typo-bugs.<br>• Document the feature flag exhaustively in the README. |

These items have not been prioritised yet; create tickets once the core
backlog (sections 1-7) shrinks.

---

*Happy refactoring!* 🛠️
