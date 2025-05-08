"""Background coroutine that polls *email* triggers.

Phase-1 only implements the scaffolding so the new *email* trigger type is
fully wired into the application lifecycle.  Actual IMAP connectivity and
filtering will be implemented in a follow-up milestone.

The service follows the same start/stop interface as ``SchedulerService`` so
`zerg.main` can manage both identically.
"""

from __future__ import annotations

import asyncio
import logging
from contextlib import suppress
from typing import Optional

# HTTP helpers
import json
import urllib.parse
import urllib.request

# Use the default session factory so tests can monkey-patch it easily via
# ``zerg.database.default_session_factory`` (as they already do for the
# SchedulerService).  We deliberately avoid importing ``SessionLocal`` – a
# simple alias – because that would bind to the *original* sessionmaker *before*
# tests get a chance to swap it out.

from sqlalchemy.orm import Session

from zerg.database import default_session_factory
from zerg.events import EventType
from zerg.events import event_bus

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
            logger.debug("EmailTriggerService already running – start() call ignored")
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
        """Scan DB for *email* triggers and log presence.

        The real implementation will connect to the mailserver defined in each
        trigger's ``config`` JSON and inspect new e-mails.  For now we merely
        look for the existence of such triggers so the stub does something
        useful during development.
        """

        # Use synchronous SQLAlchemy session inside a threadpool via asyncio.to_thread
        def _db_query() -> list[int]:
            with default_session_factory() as session:  # type: Session
                from zerg.models.models import Trigger

                rows = (
                    session.query(Trigger.id)
                    .filter(Trigger.type == "email")
                    .all()
                )
                return [row[0] for row in rows]

        triggers = await asyncio.to_thread(_db_query)

        if not triggers:
            return

        # ------------------------------------------------------------------
        # Phase-2: support *gmail* provider (readonly) ------------------------
        # ------------------------------------------------------------------

        for trg_id in triggers:
            await self._handle_gmail_trigger(trg_id)

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
        from zerg.models.models import Trigger, User  # noqa: WPS433
        from zerg.database import default_session_factory  # noqa: WPS433

        with default_session_factory() as session:
            trg: Trigger | None = session.query(Trigger).filter(Trigger.id == trigger_id).first()
            if trg is None:
                logger.warning("Trigger %s disappeared during poll", trigger_id)
                return

            if (trg.config or {}).get("provider") != "gmail":
                return  # Not a gmail trigger – we will support others later

            # MVP: pick first user with refresh token.  Future: link trigger to
            # specific user once agents are owned by users.
            user: User | None = (
                session.query(User).filter(User.gmail_refresh_token.isnot(None)).first()
            )

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


# Public singleton, mirroring ``scheduler_service`` pattern.
email_trigger_service = EmailTriggerService()
