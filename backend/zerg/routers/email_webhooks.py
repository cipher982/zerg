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

from zerg.crud import crud
from zerg.database import get_db
from zerg.events import EventType
from zerg.events import event_bus
from zerg.services.scheduler_service import scheduler_service

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

    for trg in triggers:
        await event_bus.publish(
            EventType.TRIGGER_FIRED,
            {
                "trigger_id": trg.id,
                "agent_id": trg.agent_id,
                "provider": "gmail",
                "resource_id": x_goog_resource_id,
            },
        )

        # Schedule agent execution (non-blocking)
        await scheduler_service.run_agent_task(trg.agent_id)  # type: ignore[arg-type]

    return {"status": "accepted", "trigger_count": len(triggers)}
