"""API router for Triggers (milestone M1).

Currently only supports simple *webhook* triggers that, when invoked, publish
an EventType.TRIGGER_FIRED event.  The SchedulerService listens for that event
and executes the associated agent immediately.
"""

import logging
from typing import Dict

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import Path
from fastapi import status
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db
from zerg.events import EventType
from zerg.events import event_bus
from zerg.schemas.schemas import Trigger as TriggerSchema
from zerg.schemas.schemas import TriggerCreate
from zerg.services.scheduler_service import scheduler_service

logger = logging.getLogger(__name__)


router = APIRouter(prefix="/triggers", tags=["triggers"])


@router.post("/", response_model=TriggerSchema, status_code=status.HTTP_201_CREATED)
def create_trigger(trigger_in: TriggerCreate, db: Session = Depends(get_db)):
    """Create a new trigger for an agent."""

    # Ensure agent exists
    agent = crud.get_agent(db, trigger_in.agent_id)
    if not agent:
        raise HTTPException(status_code=404, detail="Agent not found")

    trg = crud.create_trigger(db, agent_id=trigger_in.agent_id, trigger_type=trigger_in.type)
    return trg


@router.post("/{trigger_id}/events", status_code=status.HTTP_202_ACCEPTED)
async def fire_trigger_event(
    *,
    trigger_id: int = Path(..., gt=0),
    payload: Dict = {},  # Arbitrary JSON from caller
    db: Session = Depends(get_db),
):
    """Endpoint that external services hit to fire a trigger.

    For now we do **not** validate a signature – the client must supply the
    `secret` query parameter that matches the stored secret.  A more robust
    signing approach can be added later.
    """

    trg = crud.get_trigger(db, trigger_id)
    if trg is None:
        raise HTTPException(status_code=404, detail="Trigger not found")

    # Simple secret‑check via query param ( ?secret=... )
    # Very naive secret validation – expects caller to supply `{"secret": "..."}` in
    # the JSON body.  A real implementation would use an HTTP header or HMAC.
    supplied_secret = payload.pop("secret", None)
    if supplied_secret != trg.secret:
        raise HTTPException(status_code=403, detail="Invalid secret")

    # Publish event.  The scheduler will pick it up.
    await event_bus.publish(
        EventType.TRIGGER_FIRED,
        {"trigger_id": trg.id, "agent_id": trg.agent_id, "payload": payload},
    )

    # Immediately execute the agent asynchronously; this guarantees that the
    # trigger has a visible effect even if the SchedulerService listener has
    # not been started (e.g. during certain unit‑test contexts).
    await scheduler_service.run_agent_task(trg.agent_id)  # type: ignore[arg-type]

    return {"status": "accepted"}
