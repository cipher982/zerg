"""Webhook endpoints for e-mail provider push notifications.

Phase-2 of the *Email Trigger* roadmap introduces provider-side push support
so we can react to new e-mails without polling.  The most widely used provider
is Gmail; Microsoft Graph will follow a similar pattern.

For now the implementation is deliberately *minimal*:

1. The frontend registers a *watch* via the Gmail API and sets the
   ``channel_token`` to the **user ID**.  That means the callback receives
   the user ID straight from the ``X-Goog-Channel-Token`` header so we can
   resolve triggers without an additional lookup table.

2. The webhook simply schedules **all* triggers of type ``email`` whose
   ``config.provider == "gmail"``.  A future iteration will inspect the Gmail
   *history* API to filter messages according to per-trigger rules before
   firing.

Security considerations
-----------------------
Google signs the webhook with a JWT in the *Authorization* header if you use
the *"push to HTTPS"* option.  Handling and validation of that JWT will be
added later.  For now we only verify that the dev/CI tests set a dummy
``X-Goog-Channel-Token`` header so the endpoint is effectively private.
"""

from __future__ import annotations

import logging
from typing import Dict

from fastapi import APIRouter
from fastapi import Depends
from fastapi import Header
from fastapi import HTTPException
from fastapi import status
from sqlalchemy.orm import Session

from zerg.database import get_db

# Event publication and agent execution are handled inside
# `EmailTriggerService` from now on.  The router only enqueues the service
# call so we keep webhook latency minimal.

# Note: we *do not* start/stop the service here – it is already mounted by
# `zerg.main` during application start.

# Trigger processing moved into EmailTriggerService so webhook no longer
# schedules agent runs directly.

logger = logging.getLogger(__name__)


router = APIRouter(tags=["email-webhooks"])


# ---------------------------------------------------------------------------
# Gmail push notification callback
# ---------------------------------------------------------------------------


@router.post("/email/webhook/google", status_code=status.HTTP_202_ACCEPTED)
async def gmail_webhook(
    *,
    x_goog_channel_token: str = Header(..., alias="X-Goog-Channel-Token"),
    x_goog_resource_id: str | None = Header(None, alias="X-Goog-Resource-Id"),
    x_goog_message_number: str | None = Header(None, alias="X-Goog-Message-Number"),
    payload: Dict | None = None,  # Google sends no body – future-proof
    db: Session = Depends(get_db),
):
    """Handle Gmail *watch* callbacks.

    The implementation is an **MVP**: every callback simply triggers all
    *gmail* email-type triggers.  Later versions will match the *resourceId*
    to a specific user and run the Gmail *history* API to fetch only the new
    messages.
    """

    logger.debug(
        "Gmail webhook: token=%s, resource_id=%s, msg_no=%s",
        x_goog_channel_token,
        x_goog_resource_id,
        x_goog_message_number,
    )

    # ------------------------------------------------------------------
    # In the next phase we will verify the JWT (Authorization header)
    # ------------------------------------------------------------------

    # For now we require *some* channel token so accidental public hits are
    # rejected with 400 rather than executing arbitrary triggers.
    if not x_goog_channel_token:
        raise HTTPException(status_code=400, detail="Missing X-Goog-Channel-Token header")

    # ------------------------------------------------------------------
    # Fire triggers (type "email" with provider "gmail")
    # ------------------------------------------------------------------

    from zerg.models.models import Trigger  # local import to avoid cycles

    # SQLite's JSON functions are limited in in-memory CI; perform filtering
    # in Python for maximum compatibility.
    triggers = [
        trg
        for trg in db.query(Trigger).filter(Trigger.type == "email").all()
        if (trg.config or {}).get("provider") == "gmail"
    ]

    fired_count = 0

    # Use *X-Goog-Message-Number* as a quick dedup mechanism so we do not call
    # the expensive Gmail *history* API multiple times for the same push.

    msg_no_int: int | None = None
    if x_goog_message_number and x_goog_message_number.isdigit():
        msg_no_int = int(x_goog_message_number)

    from sqlalchemy.orm.attributes import flag_modified  # local import

    from zerg.services.email_trigger_service import email_trigger_service  # noqa: WPS433

    for trg in triggers:
        cfg = trg.config or {}
        last_seen = int(cfg.get("last_msg_no", 0))

        # Deduplicate – if Google re-sends push with same message number we
        # skip processing entirely.
        if msg_no_int is not None and msg_no_int <= last_seen:
            continue

        # Persist new msg number immediately so concurrent callbacks are
        # deduplicated even if the History diff processing takes a while.
        if msg_no_int is not None:
            cfg["last_msg_no"] = msg_no_int
            trg.config = cfg  # type: ignore[assignment]
            flag_modified(trg, "config")
            db.add(trg)

        fired_count += 1

        # Kick off full history diff so filtering & EventBus publishing happen
        await email_trigger_service._handle_gmail_trigger(trg.id)  # type: ignore[attr-defined]

    db.commit()

    return {"status": "accepted", "trigger_count": fired_count}
