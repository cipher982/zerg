"""Background coroutine that polls *email* triggers.

Phase-1 only implements the scaffolding so the new *email* trigger type is
fully wired into the application lifecycle.  Actual IMAP connectivity and
filtering will be implemented in a follow-up milestone.

The service follows the same start/stop interface as ``SchedulerService`` so
`zerg.main` can manage both identically.
"""

from __future__ import annotations

import asyncio

# HTTP helpers
import json
import logging
import urllib.parse
import urllib.request
from contextlib import suppress
from typing import Optional

from sqlalchemy.orm import Session

from zerg.database import default_session_factory
from zerg.models.models import Trigger

logger = logging.getLogger(__name__)


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

    def __init__(self):
        self._task: Optional[asyncio.Task[None]] = None
        self._shutdown_event = asyncio.Event()

    async def start(self):
        if self._task is not None:
            logger.debug("EmailTriggerService already running, start() call ignored")
            return

        self._shutdown_event.clear()
        self._task = asyncio.create_task(self._run_loop())
        logger.info("EmailTriggerService started (stub mode)")

    async def stop(self):
        if self._task is None:
            return

        self._shutdown_event.set()
        # Cancel the task after the event is set in case it's waiting on sleep.
        self._task.cancel()
        with suppress(asyncio.CancelledError):
            await self._task
        self._task = None
        logger.info("EmailTriggerService stopped")

    # ------------------------------------------------------------------
    # Internal helpers
    # ------------------------------------------------------------------

    async def _run_loop(self):
        """Main polling coroutine.

        Runs until ``_shutdown_event`` is set.
        """

        poll_interval_s = 60  # configurable later

        while not self._shutdown_event.is_set():
            try:
                await self._check_email_triggers()
            except Exception as exc:  # pragma: no cover – don't crash service
                logger.exception("EmailTriggerService loop error: %s", exc)

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

        # Fetch *full* trigger rows so we can mutate their config JSON later
        def _db_query() -> list["Trigger"]:
            with default_session_factory() as session:
                from zerg.models.models import Trigger

                return session.query(Trigger).filter(Trigger.type == "email").all()

        triggers = await asyncio.to_thread(_db_query)

        if not triggers:
            return

        for trg in triggers:
            provider = (trg.config or {}).get("provider")
            if provider != "gmail":
                # Other providers will be added later
                continue

            # 1) Renew watch if needed
            try:
                await self._maybe_renew_gmail_watch(trg)
            except Exception as exc:  # pragma: no cover
                logger.exception("Failed to renew Gmail watch for trigger %s: %s", trg.id, exc)

            # 2) Token refresh / placeholder polling
            try:
                await self._handle_gmail_trigger(trg.id)
            except Exception as exc:  # pragma: no cover
                logger.exception("Error in gmail trigger handler %s: %s", trg.id, exc)

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

        # Lazy import to avoid circulars
        from zerg.database import default_session_factory  # noqa: WPS433
        from zerg.models.models import Trigger  # noqa: WPS433
        from zerg.models.models import User  # noqa: WPS433

        with default_session_factory() as session:
            trg: Trigger | None = session.query(Trigger).filter(Trigger.id == trigger_id).first()
            if trg is None:
                logger.warning("Trigger %s disappeared during poll", trigger_id)
                return

            if (trg.config or {}).get("provider") != "gmail":
                return  # Not a gmail trigger – we will support others later

            # MVP: pick first user with refresh token.  Future: link trigger to
            # specific user once agents are owned by users.
            user: User | None = session.query(User).filter(User.gmail_refresh_token.isnot(None)).first()

            if user is None:
                logger.warning("No user with gmail_refresh_token found – skip trigger %s", trigger_id)
                return

            refresh_token = user.gmail_refresh_token  # type: ignore[assignment]
            try:
                access_token = await asyncio.to_thread(self._exchange_refresh_token, refresh_token)
            except Exception as exc:  # pragma: no cover – network error
                logger.error("Failed to exchange gmail refresh_token: %s", exc)
                return

            # For now we just log.  Replace with Gmail API call in Phase-3.
            logger.info(
                "[EmailTriggerService] Obtained Gmail access_token (len=%s) for trigger %s",
                len(access_token),
                trigger_id,
            )

    # ------------------------------------------------------------------
    # Static utility ----------------------------------------------------
    # ------------------------------------------------------------------

    @staticmethod
    def _exchange_refresh_token(refresh_token: str) -> str:
        """Blocking helper that swaps *refresh_token* → *access_token*.

        Raises an exception if Google returns an error.  The caller is
        expected to run this in a threadpool via ``asyncio.to_thread``.
        """

        import os

        client_id = os.getenv("GOOGLE_CLIENT_ID")
        client_secret = os.getenv("GOOGLE_CLIENT_SECRET")

        if not client_id or not client_secret:
            raise RuntimeError("GOOGLE_CLIENT_ID / SECRET not set – cannot refresh token")

        data = {
            "client_id": client_id,
            "client_secret": client_secret,
            "refresh_token": refresh_token,
            "grant_type": "refresh_token",
        }

        encoded = urllib.parse.urlencode(data).encode()

        req = urllib.request.Request(
            "https://oauth2.googleapis.com/token",
            data=encoded,
            headers={"Content-Type": "application/x-www-form-urlencoded"},
        )

        try:
            with urllib.request.urlopen(req, timeout=10) as resp:  # nosec B310
                payload = json.loads(resp.read().decode())
        except Exception as exc:  # network / HTTP error
            raise RuntimeError("token endpoint request failed") from exc

        access_token = payload.get("access_token")
        if not access_token:
            raise RuntimeError(f"invalid token response: {payload}")

        return access_token

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

        from datetime import datetime
        from datetime import timezone

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
            return

        cfg.update(new_watch)
        trigger.config = cfg  # type: ignore[assignment]

        try:
            from sqlalchemy.orm.attributes import flag_modified  # type: ignore

            flag_modified(trigger, "config")
        except ImportError:  # pragma: no cover
            pass

        # Persist
        with default_session_factory() as session:
            session.merge(trigger)
            session.commit()

    @staticmethod
    def _renew_gmail_watch_stub():
        """Return new watch meta – identical to *start* stub."""

        from datetime import datetime
        from datetime import timedelta
        from datetime import timezone

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

        from zerg.models.models import User  # noqa: WPS433 local import to avoid cycle

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

        from datetime import datetime
        from datetime import timedelta
        from datetime import timezone

        now = datetime.now(tz=timezone.utc)
        return {
            "history_id": 1,
            "watch_expiry": int((now + timedelta(days=7)).timestamp() * 1000),
        }


# Public singleton, mirroring ``scheduler_service`` pattern.
email_trigger_service = EmailTriggerService()
