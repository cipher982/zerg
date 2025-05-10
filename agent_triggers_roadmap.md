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
* **Observability groundwork:** Prometheus counters + `/metrics` route implemented (2025-05-09 commit `metrics.py`).

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
   ✅ *Prometheus counters shipped* – `trigger_fired_total`, `gmail_watch_renew_total`, `gmail_api_error_total` are now registered in `zerg.metrics` and available via a new `/metrics` endpoint (text exposition format).  
   • Next: add histograms (latency, token counts) and structured JSON logs for each trigger event.

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

### ✅ Phase 3.5 – Metrics & Observability (2025-05-09)

🎯 *Goal*: first-class visibility into trigger volume and Gmail interactions.

**Delivered**

• **Prometheus integration** – Added optional dependency `prometheus-client` and
  new module `zerg.metrics` registering the following counters:
  `trigger_fired_total`, `gmail_watch_renew_total`, `gmail_api_error_total`.

• **Instrumentation** – Counters are incremented in
  `routers/triggers.py` (webhook events) and
  `services/email_trigger_service.py` (watch renewal & API error paths).

• **/metrics endpoint** – New router exposes the text exposition format; the
  route returns *501* if the library is not available so minimal CI
  environments continue to work without the extra dependency.

• **Logging** – Added `logger.info` lines around watch renewals including the
  trigger ID and processed message counts (stepping stone for structured logs).

**Next Observability steps**

1. Histogram buckets for: trigger end-to-end latency, Gmail API latency, and
   agent run duration.
2. Structured JSON logs (or OpenTelemetry spans) for every trigger event.
3. Alerting rules in staging: >1% gmail_api_error_total over 5 min = page.


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

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

## Lessons learned from Phase B (UI spike)

1. **Never use native browser dialogs** (`alert / confirm / prompt`) inside the modal flow – they freeze the event loop and look off-brand.  All confirmations/input should be handled by in-app elements.  
2. **Runtime DOM patching** proved valuable: users might still have an old modal HTML snippet in localStorage; injecting the new Triggers tab at runtime avoided cache-clear support tickets.  
3. **Utility classes > new CSS** – by re-using existing `btn-primary`, `card`, `input-select` we shipped a visually consistent pane with zero CSS additions.  
4. **Inline wizard > full modal** – the small inline “Add Trigger” card reduced context switching and performed better on narrow tablet viewports.


– Update README, write `docs/triggers_email.md` example walk-through.

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
---

## 2025-05-09 – **Detailed Front-end Implementation Plan**  *(tracking section)*

This section captures the concrete, incremental steps required to surface **Trigger** functionality in the Rust/WASM front-end.  Each phase should compile and be testable in isolation so that partial PRs can land without breaking `main`.

### Phase A – API & Data-model groundwork

1. `frontend/src/models.rs`
   • Add a `Trigger` struct (Serde-derived) matching the backend schema.

2. `frontend/src/network/api_client.rs`
   • New helpers: `get_triggers(agent_id)`, `create_trigger`, `delete_trigger`.

3. State layer
   • Extend `AppState` with `triggers: HashMap<u32, Vec<Trigger>>` keyed by `agent_id`.
   • Messages: `LoadTriggers(agent_id)`, `TriggersLoaded(agent_id, Vec<Trigger>)`, `TriggerCreated`, `TriggerDeleted`.

### Phase B – Agent-Modal UI

4. `components/agent_config_modal.rs`
   • Add a **“Triggers”** tab.
   • List existing triggers with copy-to-clipboard for webhook secrets.
   • “Add Trigger” wizard: choose type (`webhook`, `email:gmail`).

### Phase C – Gmail Connect flow

5. Implement / finish `google_code_flow.rs`
   • Launch Google Identity Services code-client.
   • POST auth-code → `/api/auth/google/gmail`.
   • Persist `gmail_connected` flag in `AppState`.

6. Wizard checks the flag before allowing **email** trigger creation.

### Phase D – Real-time UX polish

7. Listen for `run.created` WS messages where `trigger != manual/schedule` and toast “Trigger fired”.

8. (Optional) later subscribe to future `trigger:{id}` topics.

### Phase E – Dashboard surfacing

9. Add a “Triggers” badge / column in the Dashboard agent list (shows count).

### Phase F – Tests & QA

10. WASM tests: Trigger (de)serialisation, modal Msg dispatch.
11. Manual UX pass: dark-mode contrast, clipboard success feedback, error banners.

*Progress tracker (updated 2025-05-10):*  
`[x]` **Phase A** – data-model & API helpers  
`[x]` **Phase B** – modal UI (Triggers tab, list, Add Trigger wizard, basic CRUD)  
`[ ]` **Phase C** – Gmail connect + email trigger enable  
`[ ]` Phase D  `[ ]` Phase E  `[ ]` Phase F

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
## 2025-05-12 – Front-end Phase C groundwork landed (stub)

Commit `<hash>` introduces the UI and state plumbing for Gmail connect:

• `gmail_connected` flag added to `AppState`; toggled by new
  `Message::GmailConnected`.
• Triggers tab now shows a “Connect Gmail” button and ✓ badge once connected.
• “Email (Gmail)” option in the Add-Trigger wizard is dynamically enabled only
  when `gmail_connected == true`.
• New module `frontend/src/auth/google_code_flow.rs` holds
  `initiate_gmail_connect()` – currently a stub that immediately dispatches
  `GmailConnected` so the UI path can be demoed without real OAuth.

**What’s still missing to complete Phase C**

1. Real Google Identity Services integration
   – Add JS bindings (`initCodeClient`) via `wasm_bindgen`.  
   – Pass `client_id` from `APP_STATE.google_client_id`.  
   – Exchange `auth_code` → refresh-token (`POST /api/auth/google/gmail`).  
   – On HTTP 200 dispatch `GmailConnected`; on error show toast.

2. Persist gmail_connected flag on reload
   – Once backend exposes a “/users/me” field or `system/info` reflects
     connection status, load it at startup to avoid requiring reconnection.

After 1 + 2 are done we can mark **Phase C** complete and move to Phase D
real-time toasts & dashboard surfacing.

---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
## 2025-05-12 – Additional Context / Clarifications  *(added after end-to-end code review)*

This section was appended after verifying the current `main` branch on
2025-05-12.  Keep it in sync with future changes.

1. **Phases A & B confirmed shipped**  
   If you can open the *Agent Configuration* modal and see the “Triggers” tab
   with the “Add Trigger” inline wizard, you are on the correct build.  It is
   fully wired: selecting **Webhook** → POST `/api/triggers` → list refreshes
   with secret shown.

   • Trigger model 👉 `frontend/src/models.rs`  
   • REST helpers 👉 `frontend/src/network/api_client.rs`  
   • Msg / Command plumbing 👉 `frontend/src/messages.rs`, `frontend/src/update.rs`, `frontend/src/command_executors.rs`  
   • Modal UI 👉 `frontend/src/components/agent_config_modal.rs`

2. **Webhook HMAC quick-start**  
   A valid call to `/api/triggers/{id}/events` must include:

   ```text
   X-Zerg-Timestamp: <unix-epoch-seconds>
   X-Zerg-Signature: <hex(hmac_sha256(TRIGGER_SIGNING_SECRET, "{ts}.{raw_body}"))>
   ```

   Implementation details live in `backend/zerg/routers/triggers.py`.

3. **Critical env vars**  
   • `TRIGGER_SIGNING_SECRET` – required by webhook consumers.  
   • `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` – mandatory for Phase C.  
   • `EMAIL_GMAIL_JWT_VALIDATION=1` enables stricter Gmail webhook checks in
     prod.

4. **Styling rule-of-thumb**  
   The Triggers tab re-uses utility classes (`btn-primary`, `card`, etc.).
   Please consult design-system owners before adding new CSS.

5. **Next engineering focus** – Phase C  
   Implement Google Identity Services code-client in
   `frontend/src/auth/google_code_flow.rs` (file not yet created).  On success
   POST the `auth_code` to `/api/auth/google/gmail` and set
   `state.gmail_connected = true`.

6. **Testing tip**  
   When writing WASM unit tests for triggers (Phase F) enable the
   `wasm_test` feature flag which swaps real `fetch` calls for
   `gloo::net::http::FakeTransport`.


