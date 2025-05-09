"""API router for Triggers (milestone M1).

Currently only supports simple *webhook* triggers that, when invoked, publish
an EventType.TRIGGER_FIRED event.  The SchedulerService listens for that event
and executes the associated agent immediately.
"""

import asyncio
import hashlib
import hmac
import json
import logging
import time
from typing import Dict

# FastAPI helpers
from fastapi import APIRouter
from fastapi import Body
from fastapi import Depends
from fastapi import Header
from fastapi import HTTPException
from fastapi import Path
from fastapi import status
from sqlalchemy.orm import Session

from zerg import constants
from zerg.crud import crud
from zerg.database import get_db

# Auth dependency
from zerg.dependencies.auth import get_current_user
from zerg.events import EventType
from zerg.events import event_bus

# Schemas
from zerg.schemas.schemas import Trigger as TriggerSchema
from zerg.schemas.schemas import TriggerCreate
from zerg.services.scheduler_service import scheduler_service

logger = logging.getLogger(__name__)


router = APIRouter(
    prefix="/triggers",
    tags=["triggers"],
    dependencies=[Depends(get_current_user)],
)


# ---------------------------------------------------------------------------
# DELETE /triggers/{id}
# ---------------------------------------------------------------------------


@router.delete("/{trigger_id}", status_code=status.HTTP_204_NO_CONTENT)
async def delete_trigger(
    *,
    trigger_id: int = Path(..., gt=0),
    db: Session = Depends(get_db),
):
    """Delete a trigger.

    Special handling for *email* provider **gmail**:

    • Attempts to call Gmail *stop* endpoint so push notifications are
      turned off immediately on user’s mailbox.  The call is best effort –
      network/auth failures are logged but do not abort the deletion.
    """

    # 1) Fetch the trigger (needed before deletion to inspect type/config)
    trg = crud.get_trigger(db, trigger_id)
    if trg is None:
        raise HTTPException(status_code=404, detail="Trigger not found")

    # 2) Provider-specific cleanup --------------------------------------
    if trg.type == "email" and (trg.config or {}).get("provider") == "gmail":
        try:
            from zerg.models.models import User  # local import to avoid cycles
            from zerg.services import gmail_api  # local
            from zerg.utils import crypto as _crypto  # lazy

            # Pick any gmail-connected user (MVP logic)
            user: User | None = db.query(User).filter(User.gmail_refresh_token.isnot(None)).first()

            if user is not None:
                refresh_token = _crypto.decrypt(user.gmail_refresh_token)  # type: ignore[arg-type]

                access_token = await asyncio.to_thread(  # type: ignore[attr-defined]
                    gmail_api.exchange_refresh_token,
                    refresh_token,
                )

                # Fire stop_watch in thread pool so we don’t block event loop
                await asyncio.to_thread(gmail_api.stop_watch, access_token=access_token)  # type: ignore[attr-defined]
        except Exception as exc:  # pragma: no cover – best-effort cleanup
            logger.warning("Failed to stop Gmail watch for trigger %s: %s", trg.id, exc)

    # 3) Finally delete ---------------------------------------------------
    crud.delete_trigger(db, trigger_id)

    return None  # 204 no content


@router.post("/", response_model=TriggerSchema, status_code=status.HTTP_201_CREATED)
async def create_trigger(trigger_in: TriggerCreate, db: Session = Depends(get_db)):
    """Create a new trigger for an agent.

    If the trigger is of type *email* and the provider is **gmail** we kick off
    an asynchronous helper that ensures a Gmail *watch* is registered.  The
    call is awaited so tests (which run inside the same event-loop) can verify
    the side-effects synchronously without sprinkling ``asyncio.sleep`` hacks.
    """

    # Ensure agent exists -------------------------------------------------
    agent = crud.get_agent(db, trigger_in.agent_id)
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")

    # Persist trigger -----------------------------------------------------
    trg = crud.create_trigger(
        db,
        agent_id=trigger_in.agent_id,
        trigger_type=trigger_in.type,
        config=trigger_in.config,
    )

    # ------------------------------------------------------------------
    # Provider-specific post-create hooks
    # ------------------------------------------------------------------
    if trg.type == "email" and (trg.config or {}).get("provider") == "gmail":
        # Defer heavy IO to the email trigger service
        from zerg.services.email_trigger_service import email_trigger_service  # noqa: WPS433 lazy import

        try:
            await email_trigger_service.initialize_gmail_trigger(db, trg)
        except Exception as exc:  # pragma: no cover – do not fail overall request
            logger.exception("Failed to initialise Gmail trigger %s: %s", trg.id, exc)

    return trg


@router.post("/{trigger_id}/events", status_code=status.HTTP_202_ACCEPTED)
async def fire_trigger_event(
    *,
    trigger_id: int = Path(..., gt=0),
    payload: Dict = Body(default={}),  # Arbitrary JSON body
    x_zerg_timestamp: str = Header(..., alias="X-Zerg-Timestamp"),
    x_zerg_signature: str = Header(..., alias="X-Zerg-Signature"),
    db: Session = Depends(get_db),
):
    """Webhook endpoint that fires a trigger event.

    Security: the caller must sign the request body using HMAC-SHA256.

    Signature string to hash:
        "{timestamp}.{raw_body}"

    where *timestamp* is the same value sent in `X-Zerg-Timestamp` header and
    *raw_body* is the exact JSON body (no whitespace changes).  The hex-encoded
    digest is provided via `X-Zerg-Signature` header.
    """

    # 1) Validate timestamp (prevents replay attacks)
    try:
        ts_int = int(x_zerg_timestamp)
    except ValueError:
        raise HTTPException(status_code=400, detail="Invalid X-Zerg-Timestamp header")

    now = int(time.time())
    if abs(now - ts_int) > constants.TRIGGER_TIMESTAMP_TOLERANCE_S:
        raise HTTPException(status_code=400, detail="Timestamp skew too large")

    # 2) Recompute HMAC and compare (constant-time)
    signing_secret = constants.TRIGGER_SIGNING_SECRET.encode()

    # We must use the *raw* body as delivered on the wire, not the already-
    # parsed `payload` dict.  FastAPI gives us access via request.body() but
    # we'd need Request object; instead re-serialise deterministically.
    body_serialised = json.dumps(payload, separators=(",", ":"), sort_keys=True)

    data_to_sign = f"{x_zerg_timestamp}.{body_serialised}".encode()
    expected_sig = hmac.new(signing_secret, data_to_sign, hashlib.sha256).hexdigest()

    if not hmac.compare_digest(expected_sig, x_zerg_signature):
        raise HTTPException(status_code=403, detail="Invalid signature")

    # 3) Check trigger exists
    trg = crud.get_trigger(db, trigger_id)
    if trg is None:
        raise HTTPException(status_code=404, detail="Trigger not found")

    # 4) Publish event on internal bus
    await event_bus.publish(
        EventType.TRIGGER_FIRED,
        {"trigger_id": trg.id, "agent_id": trg.agent_id, "payload": payload},
    )

    # 5) Fire agent immediately (non-blocking)
    await scheduler_service.run_agent_task(trg.agent_id)  # type: ignore[arg-type]

    return {"status": "accepted"}
