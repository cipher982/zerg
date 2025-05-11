"""Provider abstraction for *email* triggers.

The concrete Gmail implementation is just a **thin wrapper** around the
existing helper methods inside :pymod:`zerg.services.email_trigger_service`
and :pymod:`zerg.services.gmail_api`.  No runtime behaviour changes are
introduced – we only expose a common interface so that new providers (e.g.
Outlook, generic IMAP) can be added incrementally without further bloating
``EmailTriggerService``.

Why a separate module instead of baking the classes into
``email_trigger_service``?
    * Avoid circular imports once ``EmailTriggerService`` depends on the
      registry provided here.
    * Keep the provider implementations easily discoverable.

The **registry pattern** keeps initialisation trivial: a global dictionary
maps the provider identifier (string) to a *singleton* provider instance.  We
expect the provider code to be *stateless* so a single instance per process
is sufficient.
"""

from __future__ import annotations

import logging
from typing import Protocol
from typing import runtime_checkable

# ---------------------------------------------------------------------------
# Public interface
# ---------------------------------------------------------------------------


@runtime_checkable
class EmailProvider(Protocol):
    """Minimal methods an email provider implementation must expose."""

    name: str  # human readable identifier, also used for metrics label

    async def process_trigger(self, trigger_id: int) -> None:  # noqa: D401 – async handler
        """Handle *one* trigger.

        The provider is responsible for:
        1. Any watch renewal logic (if applicable)
        2. Detecting new messages / events
        3. Publishing ``TRIGGER_FIRED`` and scheduling the agent run

        The signature intentionally matches the existing private helper
        ``EmailTriggerService._handle_gmail_trigger`` so we can migrate the
        internal call-sites with minimal diff.
        """


# ---------------------------------------------------------------------------
# Gmail implementation – thin adapter around existing helpers
# ---------------------------------------------------------------------------


class GmailProvider:  # noqa: D101 – obvious from context
    """Concrete EmailProvider implementation for **Gmail**.

    The implementation was migrated from the legacy private helper
    ``EmailTriggerService._handle_gmail_trigger`` so that the full handling
    logic now lives inside the provider itself.  This removes the
    cross-service dependency and makes the call-sites uniform across all
    current and future providers.
    """

    name = "gmail"

    # A tiny in-memory cache that maps *refresh_token* → (access_token, expiry
    # epoch).  We purposely keep it **per-process** – the EmailTriggerService
    # as well as webhook callbacks live inside the same interpreter so a
    # shared dictionary is sufficient.
    _token_cache: dict[str, tuple[str, float]]

    def __init__(self) -> None:  # noqa: D401 – small helper
        # Instance is a singleton registered in `_REGISTRY` so we can safely
        # store mutable state here.
        self._token_cache = {}

    # ------------------------------------------------------------------
    # Public API (EmailProvider) ---------------------------------------
    # ------------------------------------------------------------------

    async def process_trigger(self, trigger_id: int) -> None:  # noqa: D401 – see protocol docs
        """Fetch unread Gmail messages for *one* trigger and fire events.

        The logic mirrors the behaviour that previously lived in
        ``EmailTriggerService._handle_gmail_trigger`` with only cosmetic
        changes (use provider-level state instead of ``self`` from the
        service).  No functional behaviour has been altered.
        """

        # ------------------------------------------------------------------
        # Local imports to avoid import cycles (tests patch some internals)
        # ------------------------------------------------------------------
        import asyncio
        import logging
        import time as _time_mod

        from zerg.database import default_session_factory  # local import
        from zerg.events import EventType  # noqa: WPS433 – runtime import
        from zerg.events import event_bus  # noqa: WPS433
        from zerg.metrics import gmail_api_error_total  # noqa: WPS433
        from zerg.models.models import Trigger  # noqa: WPS433
        from zerg.models.models import User  # noqa: WPS433
        from zerg.services import email_filtering  # noqa: WPS433
        from zerg.services import gmail_api  # noqa: WPS433
        from zerg.services.scheduler_service import scheduler_service  # noqa: WPS433

        logger = logging.getLogger(__name__)

        # Re-load the trigger inside a fresh session so we can mutate JSON and
        # commit safely.
        with default_session_factory() as session:
            trg: Trigger | None = session.query(Trigger).filter(Trigger.id == trigger_id).first()

            if trg is None:
                logger.warning("Trigger %s disappeared during poll", trigger_id)
                return

            if (trg.config or {}).get("provider") != "gmail":
                # Mis-configuration – guard against accidental calls.
                logger.debug("Trigger %s provider mismatch – skip", trigger_id)
                return

            # ------------------------------------------------------------------
            # OAuth – exchange *refresh* → *access* token (55-min cache)
            # ------------------------------------------------------------------

            user: User | None = session.query(User).filter(User.gmail_refresh_token.isnot(None)).first()

            if user is None:
                logger.warning("No Gmail-connected user found – cannot poll trigger %s", trg.id)
                return

            refresh_token: str = user.gmail_refresh_token  # type: ignore[assignment]

            now = _time_mod.time()
            cached = self._token_cache.get(refresh_token)
            if cached and cached[1] > now:
                access_token = cached[0]
            else:
                try:
                    access_token = await asyncio.to_thread(gmail_api.exchange_refresh_token, refresh_token)
                except Exception as exc:  # pragma: no cover – network error
                    logger.error("Failed to exchange refresh_token: %s", exc)
                    gmail_api_error_total.inc()
                    return

                # cache for 55 minutes
                self._token_cache[refresh_token] = (access_token, now + 55 * 60)

            # ------------------------------------------------------------------
            # History diff – ask Gmail API for changes since last stored HID
            # ------------------------------------------------------------------

            start_hid = int((trg.config or {}).get("history_id", 0))

            history_records = await asyncio.to_thread(gmail_api.list_history, access_token, start_hid)

            if not history_records:
                logger.debug("Trigger %s – no new history records", trg.id)
                return  # nothing new

            # Flatten message IDs + track max historyId encountered
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
            # Inspect each message; apply filters; fire events & run agent
            # ------------------------------------------------------------------

            filters = (trg.config or {}).get("filters")
            fired_any = False

            for mid in message_ids:
                meta = await asyncio.to_thread(gmail_api.get_message_metadata, access_token, mid)

                if not meta:
                    continue  # skip on error

                if not email_filtering.matches(meta, filters):
                    continue

                # Fire the platform-level trigger-fired event and schedule run
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
            # Persist the updated history_id so we do not re-process
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


# ---------------------------------------------------------------------------
# Outlook placeholder – keeps tests & type-checkers happy
# ---------------------------------------------------------------------------


class OutlookProvider:  # noqa: D101 – placeholder, raises for now
    name = "outlook"

    async def process_trigger(self, trigger_id: int) -> None:  # noqa: D401 – interface compliance
        raise NotImplementedError("Outlook provider not implemented yet")


# ---------------------------------------------------------------------------
# Registry helpers
# ---------------------------------------------------------------------------


_REGISTRY: dict[str, EmailProvider] = {
    "gmail": GmailProvider(),
    "outlook": OutlookProvider(),
}


def get_provider(name: str) -> EmailProvider | None:  # noqa: D401 – tiny helper
    """Return provider instance or *None* if unsupported."""

    return _REGISTRY.get(name)


# ---------------------------------------------------------------------------
# Convenience for logging / debug prints
# ---------------------------------------------------------------------------


def list_supported() -> list[str]:  # noqa: D401 – tiny helper
    """Return list of provider identifiers registered."""

    return list(_REGISTRY)


logger = logging.getLogger(__name__)
