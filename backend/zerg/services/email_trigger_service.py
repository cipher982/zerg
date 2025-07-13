"""Background coroutine that polls *email* triggers.

Phase-1 only implements the scaffolding so the new *email* trigger type is
fully wired into the application lifecycle.  Actual IMAP connectivity and
filtering will be implemented in a follow-up milestone.

The service follows the same start/stop interface as ``SchedulerService`` so
`zerg.main` can manage both identically.
"""

from __future__ import annotations

import asyncio
import time as _time_mod
import warnings
from contextlib import suppress
from datetime import datetime
from datetime import timedelta
from datetime import timezone
from typing import Optional

from sqlalchemy.orm import Session
from sqlalchemy.orm import sessionmaker

from zerg.database import db_session
from zerg.database import get_session_factory
from zerg.email.providers import get_provider
from zerg.events.event_bus import EventType
from zerg.events.event_bus import event_bus
from zerg.metrics import gmail_api_error_total
from zerg.metrics import gmail_watch_renew_total
from zerg.models.models import Trigger
from zerg.models.models import User
from zerg.services import email_filtering
from zerg.services import gmail_api
from zerg.services.scheduler_service import scheduler_service

# HTTP helpers
# Structured logger wrapper
from zerg.utils.log import log

# Structured wrapper `log` is used directly but several legacy helper
# blocks still reference a module-level ``logger`` variable.  Provide an
# alias so we retain backward-compatibility without rewriting all calls.

# Legacy *std logging* style adapter for compatibility with existing format
# Legacy alias – point to *log* so existing formatted strings keep working
logger = log  # type: ignore


class EmailTriggerService:
    """Very lightweight polling loop for *email* triggers (stub version).

    The loop periodically checks the database for any triggers of type
    ``email``.  For **now** it only outputs a warning if such a trigger exists
    because the actual IMAP handling has not yet landed.
    """

    _instance: "EmailTriggerService | None" = None

    def __new__(cls):
        # Force singleton so the global ``email_trigger_service`` behaves
        # similar to ``scheduler_service`` in the codebase.
        if cls._instance is None:
            cls._instance = super().__new__(cls)
        return cls._instance

    def __init__(self, session_factory: Optional["sessionmaker"] = None):
        # Lazily initialise the DB session factory so callers (main app or
        # tests) can inject a custom one.  Falling back to
        # ``get_session_factory()`` removes the last implicit dependency on
        # the *default_session_factory* global which we plan to deprecate.

        self._session_factory = session_factory or get_session_factory()

        self._task: Optional[asyncio.Task[None]] = None
        self._shutdown_event = asyncio.Event()

    async def start(self):
        if self._task is not None:
            log.debug("email-trigger", event="already-running")
            return

        self._shutdown_event.clear()
        self._task = asyncio.create_task(self._run_loop())
        log.info("email-trigger", event="started", mode="stub")

    async def stop(self):
        if self._task is None:
            return

        self._shutdown_event.set()
        # Cancel the task after the event is set in case it's waiting on sleep.
        self._task.cancel()
        with suppress(asyncio.CancelledError):
            await self._task
        self._task = None
        log.info("email-trigger", event="stopped")

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    async def _run_loop(self):
        """Main polling coroutine.

        Runs until ``_shutdown_event`` is set.
        """

        poll_interval_s = 600  # 10-minute safety-net poll; configurable via env later

        while not self._shutdown_event.is_set():
            try:
                await self._check_email_triggers()
            except Exception as exc:  # pragma: no cover – don't crash service
                log.exception("email-trigger", event="loop-error", error=str(exc))

            # Wait for either shutdown event or next poll interval
            try:
                await asyncio.wait_for(self._shutdown_event.wait(), timeout=poll_interval_s)
            except asyncio.TimeoutError:
                # Normal – loop again
                pass

    async def _check_email_triggers(self):
        """Process *email* triggers.

        Current responsibilities per trigger:
        1. Renew Gmail watches that are about to expire.
        2. Perform lightweight polling (token refresh), stub.
        3. (Future) IMAP / provider-specific checks.
        """

        # Fetch trigger data with extracted config to avoid DetachedInstanceError
        def _db_query() -> list[dict]:
            with db_session(self._session_factory) as session:
                db_triggers = session.query(Trigger).filter(Trigger.type == "email").all()
                # Extract all needed data while still in session context
                return [
                    {
                        "id": trg.id,
                        "config": trg.config or {},
                        "agent_id": trg.agent_id,
                        "type": trg.type,
                        # Add other fields as needed
                    }
                    for trg in db_triggers
                ]

        trigger_data = await asyncio.to_thread(_db_query)

        if not trigger_data:
            return

        for trg_data in trigger_data:
            provider_name = trg_data["config"].get("provider", "gmail")

            # ------------------------------------------------------------------
            # Delegate provider-specific handling
            # ------------------------------------------------------------------

            provider_impl = get_provider(str(provider_name))

            if provider_impl is None:
                log.warning(
                    "email-trigger", event="unsupported-provider", provider=provider_name, trigger_id=trg_data["id"]
                )
                continue

            # Gmail logic now lives **entirely** in ``GmailProvider``.  The
            # deprecated helper ``_handle_gmail_trigger`` has been turned into
            # a no-op stub so we can safely skip it here.  Watch renewal logic
            # is still implemented below because it has not yet been ported to
            # the provider abstraction.

            # Renew watch logic is Gmail-specific and currently lives as a
            # private method.  Call it directly here to avoid altering the
            # established tests for now.  For non-Gmail providers we expect
            # them to handle renewal inside ``process_trigger``.

            # GmailProvider now handles watch renewal internally – no need
            # to call the legacy helper here.

            # Finally hand over to provider implementation ----------------------------------------------------

            try:
                await provider_impl.process_trigger(trg_data["id"])
            except NotImplementedError:
                log.info(
                    "email-trigger", event="provider-not-implemented", provider=provider_name, trigger_id=trg_data["id"]
                )
            except Exception as exc:  # pragma: no cover – safety net
                log.exception(
                    "email-trigger",
                    event="provider-error",
                    provider=provider_name,
                    trigger_id=trg_data["id"],
                    error=str(exc),
                )

    # ------------------------------------------------------------------
    # Gmail helpers
    # ------------------------------------------------------------------

    async def _handle_gmail_trigger(self, trigger_id: int):
        """Fetch unread messages for the given Gmail trigger (MVP).

        The implementation is intentionally *very* lightweight: it merely
        obtains a short-lived *access token* from the stored refresh token and
        logs the fact.  A real implementation would call the Gmail *history*
        or *messages.list* endpoint and publish TRIGGER_FIRED events when
        criteria match.
        """

        # ------------------------------------------------------------------
        # DEPRECATED – logic moved into GmailProvider -------------------
        # ------------------------------------------------------------------

        warnings.warn(
            "EmailTriggerService._handle_gmail_trigger() is deprecated and no longer used. "
            "The handling logic has been migrated to zerg.email.providers.GmailProvider.",
            DeprecationWarning,
            stacklevel=2,
        )

        log.debug("email-trigger", event="deprecated-gmail-helper", trigger_id=trigger_id)
        return None  # short-circuit – legacy method retained for back-compat

        # Deleted unreachable legacy imports for circular/type-check purposes.

        # Re-load trigger inside a fresh session so it is attached => we can
        # mutate ``config`` and commit at the end.
        with db_session(self._session_factory) as session:
            trg: Trigger | None = session.query(Trigger).filter(Trigger.id == trigger_id).first()
            if trg is None:
                log.warning("email-trigger", event="trigger-disappeared", trigger_id=trigger_id)
                return

            if (trg.config or {}).get("provider") != "gmail":
                return  # Not a gmail trigger – will support others later

            # ------------------------------------------------------------------
            # Resolve *user* that owns refresh-token (MVP: first available)
            # ------------------------------------------------------------------
            user: User | None = session.query(User).filter(User.gmail_refresh_token.isnot(None)).first()

            if user is None:
                logger.warning("No Gmail-connected user found – cannot poll trigger %s", trg.id)
                return

            # ------------------------------------------------------------------
            # OAuth – refresh → access token (cached for 55 min)
            # ------------------------------------------------------------------
            refresh_token = user.gmail_refresh_token  # type: ignore[assignment]

            # minimal in-memory cache keyed by refresh-token string
            token_cache: dict[str, tuple[str, float]] = getattr(self, "_token_cache", {})
            self._token_cache = token_cache  # store back on instance

            now = _time_mod.time()
            cached = token_cache.get(refresh_token)
            access_token: str
            if cached and cached[1] > now:
                access_token = cached[0]
            else:
                try:
                    access_token = await asyncio.to_thread(
                        gmail_api.exchange_refresh_token,
                        refresh_token,
                    )
                except Exception as exc:  # pragma: no cover – network error
                    logger.error("Failed to exchange refresh_token: %s", exc)
                    gmail_api_error_total.inc()
                    return

                # cache for 55 minutes
                token_cache[refresh_token] = (access_token, now + 55 * 60)

            # ------------------------------------------------------------------
            # History diff call – get additions since last stored history_id
            # ------------------------------------------------------------------
            start_hid = int((trg.config or {}).get("history_id", 0))

            history_records = await asyncio.to_thread(
                gmail_api.list_history,
                access_token,
                start_hid,
            )

            if not history_records:
                logger.debug("Trigger %s – no new history records", trg.id)
                return  # nothing to do

            # Flatten message IDs -----------------------------------------------
            message_ids: list[str] = []
            max_hid = start_hid
            for h in history_records:
                try:
                    hid_int = int(h["id"])
                    max_hid = max(max_hid, hid_int)
                except Exception:  # noqa: BLE001 – tolerant parsing
                    pass

                for added in h.get("messagesAdded", []):
                    msg = added.get("message", {})
                    mid = msg.get("id")
                    if mid:
                        message_ids.append(str(mid))

            if not message_ids:
                logger.debug("Trigger %s – history records contained no messages", trg.id)
                # Still advance history id to avoid re-processing same empty diff
                if max_hid > start_hid:
                    (trg.config or {}).update({"history_id": max_hid})
                    try:
                        from sqlalchemy.orm.attributes import flag_modified  # type: ignore

                        flag_modified(trg, "config")
                    except ImportError:
                        pass
                    session.add(trg)
                    session.commit()
                return

            # ------------------------------------------------------------------
            # Process each message – filter & possibly trigger agent
            # ------------------------------------------------------------------
            filters = (trg.config or {}).get("filters")

            fired_any = False
            for mid in message_ids:
                meta = await asyncio.to_thread(
                    gmail_api.get_message_metadata,
                    access_token,
                    mid,
                )

                if not meta:
                    continue  # skip on error

                if not email_filtering.matches(meta, filters):
                    continue

                # Fire!
                await event_bus.publish(
                    EventType.TRIGGER_FIRED,
                    {
                        "trigger_id": trg.id,
                        "agent_id": trg.agent_id,
                        "provider": "gmail",
                        "message_id": mid,
                    },
                )

                await scheduler_service.run_agent_task(trg.agent_id)  # type: ignore[arg-type]
                fired_any = True

            # ------------------------------------------------------------------
            # Persist *new* history id (largest seen)
            # ------------------------------------------------------------------
            if max_hid > start_hid:
                cfg = dict(trg.config or {})
                cfg["history_id"] = max_hid
                trg.config = cfg  # type: ignore[assignment]

                try:
                    from sqlalchemy.orm.attributes import flag_modified  # type: ignore

                    flag_modified(trg, "config")
                except ImportError:  # pragma: no cover
                    pass

                session.add(trg)
                session.commit()

            logger.info(
                "Gmail trigger %s processed %s messages (fired=%s)",
                trg.id,
                len(message_ids),
                fired_any,
            )

    # ------------------------------------------------------------------
    # Static utility ----------------------------------------------------
    # ------------------------------------------------------------------

    # `_exchange_refresh_token` was moved to `zerg.services.gmail_api` so the
    # logic can be re-used by other services.  The old helper is deleted to
    # avoid duplicate code paths.

    # ------------------------------------------------------------------
    # Watch renewal helpers --------------------------------------------
    # ------------------------------------------------------------------

    async def _maybe_renew_gmail_watch(self, trigger):  # noqa: D401 – short name
        """Renew Gmail watch if expiry is within 24 h.

        We operate fully with **stub** data when running under the test
        suite or dev mode so no external network traffic is generated.  The
        method therefore delegates to ``_renew_gmail_watch_stub`` which
        returns new history / expiry values.
        """

        cfg = trigger.config or {}
        expiry_ts = cfg.get("watch_expiry")  # milliseconds since epoch

        if expiry_ts is None:
            # Cannot decide – trigger likely never initialised yet
            return

        now_ms = int(datetime.now(tz=timezone.utc).timestamp() * 1000)

        # Renew if expiry within next 24 hours (86400*1000 ms)
        if expiry_ts - now_ms > 24 * 60 * 60 * 1000:
            return  # still valid

        # Renew ----------------------------------------------------------
        logger.info("Renewing Gmail watch for trigger %s", trigger.id)

        try:
            new_watch = await asyncio.to_thread(self._renew_gmail_watch_stub)
        except Exception as exc:  # pragma: no cover
            logger.error("Failed to renew Gmail watch: %s", exc)
            # Metrics ---------------------------------------------------
            gmail_api_error_total.inc()
            return

        cfg.update(new_watch)
        trigger.config = cfg  # type: ignore[assignment]

        # Metrics -------------------------------------------------------
        gmail_watch_renew_total.inc()

        try:
            from sqlalchemy.orm.attributes import flag_modified  # type: ignore

            flag_modified(trigger, "config")
        except ImportError:  # pragma: no cover
            pass

        # Persist
        with db_session(self._session_factory) as session:
            session.merge(trigger)

    @staticmethod
    def _renew_gmail_watch_stub():
        """Return new watch meta – identical to *start* stub."""

        now = datetime.now(tz=timezone.utc)
        return {
            "history_id": int(now.timestamp()),
            "watch_expiry": int((now + timedelta(days=7)).timestamp() * 1000),
        }

    # ------------------------------------------------------------------
    # Public helpers called by router hooks -----------------------------
    # ------------------------------------------------------------------

    async def initialize_gmail_trigger(self, db_session: Session, trigger):  # noqa: D401 – short name
        """Ensure Gmail *watch* registration & baseline history id.

        This helper is invoked right after an *email* trigger of provider
        **gmail** is created.  The logic is deliberately very light-weight for
        now – we *do not* perform the network request to Google by default so
        CI remains free of external dependencies.  Instead we:

        1. Look up any user who has a ``gmail_refresh_token`` (dev/CI only has
           one user row).  In the future triggers will be linked to a specific
           user.
        2. If the trigger config already contains ``history_id`` we assume the
           watch is active and do nothing.
        3. Otherwise we generate a **dummy** history_id plus an expiry that is
           7 days in the future.  Tests can monkey-patch the private helper
           ``_start_gmail_watch`` to return deterministic values.
        4. Persist the updated config JSON on the trigger row.

        A follow-up milestone will replace step 3 with a real call to the
        *watch* endpoint and will store the returned ``historyId`` &
        ``expiration``.
        """

        if (trigger.config or {}).get("history_id") is not None:
            logger.debug("Trigger %s already has Gmail watch metadata – skip init", trigger.id)
            return

        # Pick *any* user with gmail refresh token for MVP ----------------
        user: User | None = db_session.query(User).filter(User.gmail_refresh_token.isnot(None)).first()

        if user is None:
            logger.warning("No gmail-connected user found – cannot register watch for trigger %s", trigger.id)
            return  # Cannot proceed – will try later when service runs

        # ------------------------------------------------------------------
        # In production we would obtain an access_token and call Gmail here.
        # The call is encapsulated in a private helper so tests can patch it.
        # ------------------------------------------------------------------

        try:
            watch_info = await asyncio.to_thread(self._start_gmail_watch_stub)
        except Exception as exc:  # pragma: no cover – network failure etc.
            logger.error("Failed to start Gmail watch for trigger %s: %s", trigger.id, exc)
            return

        # Merge into trigger.config ----------------------------------------
        cfg = dict(trigger.config or {})
        cfg.update(
            {
                "history_id": watch_info["history_id"],
                "watch_expiry": watch_info["watch_expiry"],
            }
        )
        trigger.config = cfg  # type: ignore[assignment]

        # Flag JSON column as modified so SQLAlchemy persists it
        try:
            from sqlalchemy.orm.attributes import flag_modified  # type: ignore

            flag_modified(trigger, "config")
        except ImportError:  # pragma: no cover
            pass

        db_session.add(trigger)
        db_session.commit()

    # ------------------------------------------------------------------
    # Stub for Gmail watch registration --------------------------------
    # ------------------------------------------------------------------

    @staticmethod
    def _start_gmail_watch_stub():
        """Return dummy watch metadata (for dev / CI).

        The real implementation will contact the Gmail API – this stub keeps
        unit-tests self-contained and allows monkey-patching.
        """

        now = datetime.now(tz=timezone.utc)
        return {
            "history_id": 1,
            "watch_expiry": int((now + timedelta(days=7)).timestamp() * 1000),
        }


# Public singleton, mirroring ``scheduler_service`` pattern.
email_trigger_service = EmailTriggerService()
