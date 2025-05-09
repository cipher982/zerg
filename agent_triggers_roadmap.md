# Results of Codebase Scan: Trigger Infrastructure & Recommendations

## Backend: Triggers

### What Exists

* **Trigger Model:**
    File: `backend/zerg/models/models.py`

        class Trigger(Base):
            __tablename__ = "triggers"
            id = Column(Integer, primary_key=True, index=True)
            agent_id = Column(Integer, ForeignKey("agents.id"), nullable=False)
            type = Column(String, default="webhook", nullable=False)
            secret = Column(String, nullable=False, unique=True, index=True)
            created_at = Column(DateTime, server_default=func.now())
            # ...model docstring: Only webhook implemented, room for email/etc
* **API Endpoints:**


* **Create Trigger**: supports `type = "webhook"` **or** `"email"` (Gmail provider today)
        See `backend/zerg/routers/triggers.py`

    * **POST `/api/triggers/{id}/events`**: Webhook endpoint, HMAC protected
* **Event bus integration**:
    When `/api/triggers/{id}/events` is called, code fires an `EventType.TRIGGER_FIRED` event, which downstream (e.g., `SchedulerService`) listens to and executes the associated agent.
* **Tests:**
    There are backend tests for "webhook trigger flow" (`tests/test_triggers.py`) confirming it will invoke agent execution.

#### Gaps / Not Yet Implemented

The core Gmail-based *email* trigger path (watch registration → push webhook  → history diff → `TRIGGER_FIRED`) is now **fully working and tested**.  The remaining gaps are:

* **Provider abstraction:** only Gmail is supported – Outlook / generic IMAP are still on the roadmap.
* **Robust error handling:** exponential back-off & quota-retry wrapper around Gmail API calls.
* **Production-grade crypto:** refresh-tokens are XOR-obfuscated; switch to AES-GCM/Fernet.
* **Observability:** add structured logs + Prometheus metrics around trigger latency and failures.
* **Front-end UI:** no CRUD surfaces yet (see dedicated section below).

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Frontend: Triggers

### What Exists

* **No visible trigger management UI**


    * No references to listing/adding triggers in `agent_config_modal.rs` or related files.

    * Dashboard: displays runs, possibly logs trigger source (shows manual/schedule/api).
* **Models & API:**


    * Models and API client code understand `"trigger"` as a *string describing run source* (`manual`, `schedule`, `api`); no trigger objects or trigger management.

    * No code for `email` or `webhook` config, no trigger CRUD calls surfaced.

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Summary Table

┌──────────┬─────────────────┬───────────────┬─────────────────┬──────────────────────┐
│ Area     │ Webhook Support │ Email Support │ UI for Triggers │ Comments             │
├──────────┼─────────────────┼───────────────┼─────────────────┼──────────────────────┤
│ Backend  │ Yes (MVP)       │ ✅ Gmail (push+diff complete) │ n/a             │ Webhook & Gmail email triggers fully live │
├──────────┼─────────────────┼─────────────────┼─────────────────┼──────────────────────┤
│ Frontend │ Not surfaced    │ Connect-button WIP │ No              │ Gmail consent UI pending │
└──────────┴─────────────────┴───────────────┴─────────────────┴──────────────────────┘

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Actionable Backend TODOs for Email Triggers

### Completed backend tasks (✅)

* JSON `config` column on Trigger model
* `/api/auth/google/gmail` – auth-code → refresh-token storage
* `gmail_refresh_token` column on User
* `/api/email/webhook/google` endpoint (MVP, publishes `TRIGGER_FIRED`)
* EmailTriggerService singleton (polls, refresh-token → access-token)
* Tests: webhook + gmail flow
* Gmail Watch registration & renewal (real API with stub fallback)
* Gmail History diff + filtering; publishes `TRIGGER_FIRED` and schedules agent runs
* DELETE /triggers/{id} now stops Gmail watch (clean-up)
* Security hardening: encrypted refresh-tokens, optional Google JWT validation
* Regression tests: watch initialisation, renewal, history progression, stop-watch clean-up

### Outstanding backend work (🔄) – updated 2025-05-09

1. **API reliability & hardening**  
   • Implement retry / exponential-backoff wrapper around Gmail HTTP calls.  
   • Surface quota / auth errors via metrics & structured logs.

2. **Provider abstraction**  
   • Introduce `BaseEmailProvider` interface.  
   • Add Outlook (Microsoft Graph) implementation.  
   • Ship fallback IMAP polling for generic hosts.

3. **Security**  
   • Replace XOR scheme with AES-GCM / Fernet for `gmail_refresh_token`.  
   • Make Google-JWT validation **mandatory** in production (remove opt-in flag).  
   • Improve UX when `GOOGLE_CLIENT_ID/SECRET` are missing.

4. **Observability**  
   • Emit Prometheus counters (`trigger_fired_total`, `gmail_watch_renew_total`, `gmail_api_error_total`).  
   • Add JSON logs for every watch renewal and trigger fire.

5. **Test coverage**  
   • Unit-tests for `email_filtering.matches` edge cases.  
   • Tests that exercise the access-token **cache hit** path.

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Frontend TODOs

1. **Trigger Management UI:**


    * List current triggers for an agent (in modal/dashboard).

    * “Add trigger” flow that allows selection between `"webhook"` and `"email"` triggers.

* For email: UI to collect server/imap config, mailbox, filters, etc.

2. **Gmail Connect UX (In-progress)**

    * “Connect Gmail” button that launches GIS *code client* with
      `gmail.readonly` + `access_type=offline` + `prompt=consent`.
    * POSTs `auth_code` to `/api/auth/google/gmail`; store success flag in
      AppState to show a ✓ badge.
    * “Disconnect” (DELETE token) – **TODO**.

3. **API Integration:**


    * Fetch, create, delete triggers.

* Show trigger status/history per agent.

4. **Real-Time:**


* Show when triggers fire (toast overlay, run history badge).

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Kinds of Triggers Possible

* *Webhook* (already shipped)
* *Email* (**next—see above**)
* (Later: Slack, Google Calendar, File upload, Web form, etc.)

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## Priorities for Email Trigger MVP

* **Backend**: Model/config, background service, event firing.
* **Frontend**: Add/configure via agent modal. Show status/history/log.

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
## 2025-05-08 – Implementation Log / Progress

### ✅ Phase 1: Backend scaffolding (config column + stub polling service)

1. **Trigger model extended**  
   • Added `config : JSON` column so arbitrary settings can be stored per-trigger (commit `models/models.py`).

2. **Pydantic schemas updated**  
   • `TriggerBase`, `TriggerCreate`, `Trigger` now expose an optional `config` field (commit `schemas/schemas.py`).

3. **CRUD helper broadened**  
   • `create_trigger()` accepts `config` kwarg; existing webhook tests unaffected (commit `crud/crud.py`).

4. **Trigger router tweaked**  
   • Pass `trigger_in.config` through to CRUD (commit `routers/triggers.py`).

5. **EmailTriggerService stub introduced**  
   • New file `services/email_trigger_service.py` with singleton `email_trigger_service`.  
   • Background poller merely detects presence of `email` triggers and logs a warning.  
   • Future work: connect to IMAP & publish `EventType.TRIGGER_FIRED`.

6. **Application lifecycle wired**  
   • `zerg.main` now starts/stops `email_trigger_service` alongside `scheduler_service`.

7. **Database session alias**  
   • Re-introduced `SessionLocal` alias for backwards compatibility (commit `database.py`).

8. **Tests**  
   • Added `backend/tests/test_email_trigger_service.py` smoke-test.  
   • Suite now at **110 passed / 15 skipped** (`./backend/run_backend_tests.sh` on local).  
   • Coverage for email triggers remains minimal (stub detection only).

### 🔄 Smoke-Test Addition (same date)

* The new test verifies REST creation of an *email* trigger and executes the stub checker without raising.

### Next Steps

### ✅ Phase 2: Gmail OAuth & Push Skeleton (2025-06-XX)

1. **User refresh-token storage**  (`gmail_refresh_token` column).  
2. **/api/auth/google/gmail** – exchanges *authorization_code* → refresh-token.  
3. **EmailTriggerService** – converts refresh→access token every 60 s.  
4. **/api/email/webhook/google** – receives Gmail push, publishes `TRIGGER_FIRED`, schedules agent run.  
5. **Tests:** `test_gmail_webhook_trigger.py` exercises full flow (agent + trigger + webhook).

---

test `test_gmail_watch_initialisation.py` added.
### ✅ Phase 3 – Gmail Watch & History Diff  *(completed – 2025-05 → 2025-06)*

Milestone goal: end-to-end *push* flow that only fires runs for **new**
messages which pass user-defined filters.

**Implemented so far**

✅ *Watch auto-creation* (real API with stub fallback) – `initialize_gmail_trigger` persists
`history_id` & `watch_expiry` into `trigger.config`.  See
`test_gmail_watch_initialisation.py`.

✅ *Watch renewal* – `EmailTriggerService._maybe_renew_gmail_watch` now first
tries **real Gmail watch** API and falls back to the deterministic stub when
offline.  Unit-tested (`test_gmail_watch_renewal.py`).

✅ *Webhook de-duplication* – Handler tracks `last_msg_no` to avoid double
processing.  Covered by `test_gmail_webhook_trigger.py`.

✅ *History diff + filtering* – `_handle_gmail_trigger` now:
   1. Exchanges **refresh_token → access_token** via
      `gmail_api.exchange_refresh_token` (requires
      `GOOGLE_CLIENT_ID/SECRET` env vars – tests stub this).
   2. Calls `gmail_api.list_history(start_history_id)` and flattens the
      result to message-IDs.
   3. Fetches metadata for each message and evaluates
      `email_filtering.matches()` which currently supports:
      • query (substring)
      • from_contains / subject_contains
      • label_include / label_exclude
   4. Fires `TRIGGER_FIRED` events *and* schedules the agent run via
      `scheduler_service.run_agent_task`.

✅ *Cross-session commit/refresh fix* – Webhook handler now commits
`last_msg_no` *before* calling the helper and then refreshes the instance so
concurrent updates to `history_id` are merged correctly (2025-05-10).

✅ *High-level regression tests* – Added
`test_gmail_webhook_history_progress.py` ensuring
    • history_id advances to the highest value from Gmail History diff
    • a new X-Goog-Message-Number triggers another agent run.
  Suite now **114 passed** / 15 skipped.

✅ *Security hardening* –
   • `gmail_refresh_token` now stored encrypted (`zerg.utils.crypto`).  
   • Optional Google JWT validation added to `/api/email/webhook/google` (enable with `EMAIL_GMAIL_JWT_VALIDATION=1`).  

**Remaining technical debt (moved to *Outstanding backend work* list)**

• Robust back-off & quota handling around Gmail API requests.  
• AES-GCM encryption (current XOR scheme is placeholder).  
• Enable JWT validation by default in prod envs (remove opt-in flag).  

**Test impact**

Because token exchange is now part of the flow, **unit tests must monkey-patch
`gmail_api.exchange_refresh_token`** or set dummy `GOOGLE_CLIENT_ID` /
`GOOGLE_CLIENT_SECRET` in the environment.  A failing test is a hint that the
patch is missing.


### 🚧 Phase 4 – Front-end CRUD / UX

– “Connect Gmail” button (in progress – `google_code_flow.rs`).  
– Trigger list & wizard in Agent modal.  
– Real-time toast & run history filter.

**Backend prerequisite finished** – `/api/auth/google/gmail` endpoint is now
stable; once the front-end stores the returned *success* flag the user’s
refresh-token is ready for the trigger service.

### Phase 5 – Docs & Examples

– Update README, write `docs/triggers_email.md` example walk-through.
