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
    name = "gmail"

    async def process_trigger(self, trigger_id: int) -> None:  # noqa: D401 – see protocol docs
        """Delegate to the *current* Gmail handler inside the service.

        Importing inside the method avoids an *import cycle* (the service
        will import the registry → importing this module globally would loop
        back to the service).
        """

        from zerg.services.email_trigger_service import email_trigger_service  # local import

        await email_trigger_service._handle_gmail_trigger(trigger_id)  # noqa: WPS437 – intentional private access


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
