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


    * **Create Trigger**: (type: currently only `"webhook"`)
        See `backend/zerg/routers/triggers.py`

    * **POST `/api/triggers/{id}/events`**: Webhook endpoint, HMAC protected
* **Event bus integration**:
    When `/api/triggers/{id}/events` is called, code fires an `EventType.TRIGGER_FIRED` event, which downstream (e.g., `SchedulerService`) listens to and executes the associated agent.
* **Tests:**
    There are backend tests for "webhook trigger flow" (`tests/test_triggers.py`) confirming it will invoke agent execution.

#### What Doesn’t Exist

* **Email triggers implementation:**
    There is *no* `email` or similar code in backend/ (no IMAP, no polling, no email config).
* **No "trigger config" for email credentials, inbox, filters, etc.**
* **No email background service:**
    No polling/task for IMAP, no firing code to raise an event based on incoming email.

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
│ Backend  │ Yes (MVP)       │ Partial (Gmail) │ n/a             │ Webhook + Gmail endpoints live │
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

### Outstanding backend work (🔄)

1. **Gmail Watch & Message Diff**  
   • Automatically create `watch` after user connects Gmail.  
   • Use Gmail History API to identify *new* messages only.  
   • Evaluate trigger filters before firing.

2. **Provider abstraction**  
   • Add Outlook (Microsoft Graph) implementation mirroring Gmail flow.  
   • Fallback IMAP polling for generic hosts.

3. **Security hardening**  
   • Validate Google-signed JWT on webhook calls.  
   • Encrypt refresh-tokens at rest.

4. **Observability**  
   • Detailed logging + metrics for e-mail polling / push latency.

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

### 🚧 Phase 3 – Gmail Watch & History Diff  (next sprint)
– Auto-create / renew `watch` after Gmail connect.  
– Call Gmail History API with `startHistoryId`.  
– Fire trigger only for *new* matching messages.

**Progress 2025-05-08 → 2025-05-09**

✅ **Watch auto-creation (stub)** – When a *gmail* email-trigger is created the
backend now calls `EmailTriggerService.initialize_gmail_trigger`. The helper
stores a `history_id` & `watch_expiry` field inside the trigger `config`
JSON (stub values during development, real API call TBD).  Comprehensive unit
test `test_gmail_watch_initialisation.py` added.

🔄 **Renewal & History diff** – still pending.

2025-05-09 (commit <hash>)

✅ **Watch renewal (stub)** – EmailTriggerService now renews Gmail watches when
`watch_expiry` is within 24 h.  Renewal logic uses a stubbed helper and is
fully unit-tested (`test_gmail_watch_renewal.py`).

✅ **Webhook de-duplication** – Gmail webhook handler stores
`last_msg_no` (header *X-Goog-Message-Number*) per trigger and fires only once
per message.  Added test `test_gmail_webhook_trigger.py` to assert dedup.

🔄 **History diff API call & advanced filters** – next.

🔄 **Filtering new messages** – pending.


### 🚧 Phase 4 – Front-end CRUD / UX

– “Connect Gmail” button (in progress – `google_code_flow.rs`).  
– Trigger list & wizard in Agent modal.  
– Real-time toast & run history filter.

### Phase 5 – Docs & Examples

– Update README, write `docs/triggers_email.md` example walk-through.
