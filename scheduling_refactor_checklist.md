# Agent Scheduling UX Redesign – Development Checklist

This file tracks every concrete step required to migrate from the old
`cron-string + run_on_schedule` implementation to the new, user-friendly
scheduling flow.  Tick the boxes as you land each change via PR / commit.

> **Note:** The project is still in pre-prod.  We can break DB compat and
> remove columns outright – no Alembic migrations are necessary.

---

## 1  Back-end

- [x] **Models** – delete `run_on_schedule` column from
  `backend/zerg/models/models.py::Agent`.
- [x] **Pydantic schemas** – purge every `run_on_schedule` field from
  `backend/zerg/schemas/schemas.py` (`AgentBase`, `AgentCreate`, `AgentUpdate`,
  `Agent`).
- [x] **CRUD helpers** – remove the boolean from
  `backend/zerg/crud/crud.py` (create & update paths).
- [x] **Routers / REST** – strip parameter & response key in
  `backend/zerg/routers/agents.py`.
- [x] **SchedulerService** – refactor
  `backend/zerg/services/scheduler_service.py` to rely solely on
  `schedule != None` for (un)scheduling.
- [x] **Event payloads** – stop emitting `run_on_schedule` in event bus
  messages.
- [x] **Tests** – update fixtures & assertions that reference the flag.

## 2  Front-end (Rust/WASM)

- [ ] **API models** – remove the field from
  `frontend/src/models.rs` (`ApiAgent*` structs); add
  `fn is_scheduled(&self) -> bool` helper.
- [ ] **Global Msg definition** – update `frontend/src/messages.rs` to drop the
  boolean in `SaveAgentDetails` (introduce structured `ScheduleForm`?).
- [ ] **Update logic** – refactor `frontend/src/update.rs` to build cron
  strings from the new UI selector and send `schedule = Some(...) | None`.
- [ ] **Dashboard** – replace status check with `is_scheduled()` in
  `frontend/src/components/dashboard/mod.rs`.
- [ ] **Agent-Config Modal UI** – rebuild schedule controls in
  `frontend/src/components/agent_config_modal.rs` (dropdown based, summary
  label, `cron_from_ui()` helper).
- [ ] **Remove dead references** – `grep -R run_on_schedule frontend/src` and
  clean-up.
- [ ] **Frontend tests** – adjust & add unit tests for the new mapping.

## 3  Data reset

- [ ] Drop and recreate local `agents` table (simple `uv run python` shell or
  `sqlite3`).

## 4  House-keeping / docs

- [ ] Update `DATABASE.md` to describe the simplified model.
- [ ] Update README quick-start (no `run_on_schedule` mention).

---

Feel free to expand, split, or reorder tasks as implementation evolves.  Check
☑️ boxes in follow-up commits to visualise progress.

---

## Lessons learned so far

* The legacy boolean surfaced in more places than expected (CRUD defaults,
  scheduler DB queries, event payload decorator).  Grep-driven removal plus
  unit-test feedback proved essential.
* SchedulerService was already designed to detach jobs before re-adding them;
  this made the refactor simple—just change the predicates.
* Removing a column at ORM level required clearing helper defaults too (e.g.
  `AgentCreate` previously injected `run_on_schedule=False`).
* Tests that looked at raw event payloads no longer needed updates because the
  decorator publishes full model dicts; once the field disappears from the
  model, it disappears from events automatically.
* Having isolated pytest for the scheduler allowed confident rewrites without
  touching the broader suite.

## Next steps quick-links

```
grep -R "run_on_schedule" frontend/src   # verify Rust side is clean after edits
``` 

---
