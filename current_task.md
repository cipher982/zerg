# Retrofit of Zerg Agent Layer – Project Log & Next-Step Blueprint 🗒️


## 0. Executive summary

Over the last series of debugging sessions we chased a stubborn InvalidUpdateError that crashed the chat run-loop (POST /api/threads/{id}/run) whenever LangGraph received a Future object instead of a
concrete update dict.
Root cause: a mixed programming-model – we wrapped synchronous @task nodes (which always yield a Future) inside a synchronous LangGraph graph, and we sometimes called the async OpenAI API (ChatOpenAI.invoke
→ run_in_executor) that itself also returns a Future when inside an event-loop.
The quick-fixes (Future unwrapping) worked but were brittle. We agreed to pivot to the pure Functional API pattern and introduce a dedicated SQLCheckpointer so persistence remains in our Postgres/SQLite
tables while graph logic stays declarative.

## 1. Timeline of the debugging journey

┌──────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┬────────────────────────────────────────────────────────────────
─────────────┐
│ Step │ Change / Observation                                                                                                                 │ Outcome                                                        
                │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ A    │ Original code used AgentRunner.run_thread() (sync) + ThreadService for DB writes.                                                    │ Worked until LangGraph upgrade tightened state-type checks.
                │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ B    │ 500-error surfaced: InvalidUpdateError: Expected dict, got Future.                                                                   │ Identified Future leak from ChatOpenAI.invoke.
                │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ C    │ Added unwrapping (response.result()).                                                                                                │ Removed crash but blocked threads; tests still passed due to
heavy mocking. │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ D    │ Attempted to “go async” by making call_llm async def, but kept @task → RuntimeError: In a sync context async tasks cannot be called. │ Mix of sync graph + async node invalid.
                │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ E    │ Removed @task, awaited ainvoke – Future resurfaced because entrypoint orchestration missing.                                         │ Realised we were mixing two LangGraph idioms.
                │
├──────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼────────────────────────────────────────────────────────────────
─────────────┤
│ F    │ Discussion converged on “embrace Functional API + Checkpointer”; processed-flag use-case examined; plan drafted.                     │ Current state – ready for refactor.
                │
└──────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┴────────────────────────────────────────────────────────────────
─────────────┘


## 2. Current file/function landscape (relevant parts)

    backend/
    │
    ├─ zerg/agents_def/zerg_react_agent.py        # Graph definition (tasks/nodes)
    ├─ zerg/managers/agent_runner.py              # Orchestration & DB writes (to retire)
    ├─ zerg/services/thread_service.py            # SQL helpers, msg ↔︎ LangChain conversion
    ├─ zerg/routers/threads.py                    # POST /threads/{id}/run endpoint
    └─ tests/
        └─ conftest.py                             # Heavy stubbing of ChatOpenAI + StateGraph

Key pain-points:

    * `call_llm` returns **Future** when mis-decorated or when OpenAI helper detects an event-loop.
    * `ThreadMessage.processed` flag makes sense for **task scheduling**, but current Runner explicitly depends on it.
    * Tests mock away real behaviour, hiding bugs until production.


## 3. Lessons learned

    1. **LangGraph API rules**
        • `@task` always ⇒ returns a *Future* (sync wrapper).
        • A coroutine node (`async def`) **must not** be decorated with `@task`; LangGraph will await it automatically.
        • Graph must eventually emit a **plain dict** per node update.
    2. **ChatOpenAI quirks**
        • `invoke()` is “sync” but inside an event-loop it offloads to a thread-pool and returns a `concurrent.futures.Future`.
        • `ainvoke()` is truly async and returns the final message.
    3. **Testing philosophy**
        • Over-mocking (MagicMock returning concrete values) masked Future behaviour.
        • Better: provide stubs that mimic **async** API (`ainvoke`) so tests exercise the same code path.
    4. **Persistence separation**
        • ThreadService is already an impedance-adapter (LangChain ⇄ SQL).
        • A LangGraph **Checkpointer** offers a cleaner single-responsibility home for the persistence logic, allowing us to drop AgentRunner.

## 4. Goals going forward

### Strategic

    * Migrate to **pure Functional-API** graph with a single `@entrypoint`.
    * Replace AgentRunner with **SQLCheckpointer** that:
        1. Loads full thread history plus **pending** `processed=False` rows.

        2. After each run writes new assistant/tool messages and flips pending rows to processed.
    * Maintain `processed` semantics so scheduled/autonomous tasks and multi-agent scenarios still work.

### Tactical (immediate)

    * Stabilise chat endpoint: pick _one_ consistent node style (sync+unwrap **or** async await).
    * Reduce over-mocking in tests; keep only ChatOpenAI stub implementing `ainvoke`.


## 5. Task list 📋

### Phase 1 – Stabilise current implementation

    * [ ]  **Choose interim node style**
        ☐ Keep `@task` + `.result()` unwrap inside node
        ☐ Or switch to coroutine node + ensure graph invoked via `ainvoke`
    * [ ]  Remove duplicate Future-unwrap guards.
    * [ ]  Slim down `tests/conftest.py`: stub must expose both `.invoke` & `.ainvoke`, but no Magical side-effects.

### Phase 2 – Design SQLCheckpointer

    * [ ]  Draft `SQLCheckpointer` class (subclass `langgraph.checkpoint.base.Checkpointer`).
    * [ ]  Implement `.get(thread_cfg)` that returns `{ "messages": previous, "pending": pending }`.
    * [ ]  Implement `.put(thread_cfg, checkpoint)` to:
        • insert assistant/tool messages
        • mark `pending` IDs as processed
        • update thread timestamp.

### Phase 3 – Pure Functional graph

    * [ ]
        Re-write `zerg_react_agent`:


        * `@task call_model` (sync)

        * `@task call_tool` (sync)

        * `@entrypoint(checkpointer=SQLCheckpointer())` orchestration loop (`.result()` unwrap included).
    * [ ]
        Delete AgentRunner; update FastAPI route to:

            @router.post("/threads/{id}/run")
            async def run_thread(id: int):
                cfg = {"configurable": {"thread_id": str(id)}}
                result = await agent.ainvoke(new_user_msgs, cfg)
                ...
    * [ ]
        Hook websocket streaming via `agent.astream(...)`.

### Phase 4 – Test & clean-up

    * [ ]  Rewrite existing smoke tests to run through new entrypoint; use `MemorySaver` or in-memory SQLite Checkpointer.
    * [ ]  Remove processed-flag logic from legacy paths (keep column for tasks).
    * [ ]  Bench-test long-running scheduled agents.

### Phase 5 – Future features

    * [ ]  Multi-agent threads (processed-by-agent column or payload).
    * [ ]  Per-agent memory strategies (VectorStore, SQL, etc.) via Checkpointer variants.
    * [ ]  Token/latency metrics in Checkpointer.


## 6. References

    * LangGraph Functional API doc: <https://python.langchain.com/docs/langgraph/>
    * `_get_updates` / `_assemble_writes` source – enforces dict update type.
    * ChatOpenAI `invoke()` implementation (see `chat_models/base.py`) – loop detection.


## 7. Next action

    1. **Pick the interim stabilisation style** (sync+unwrap vs async).
    2. Schedule design spike for `SQLCheckpointer`.