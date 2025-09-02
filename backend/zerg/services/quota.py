"""Quota helpers for per-user daily run caps.

Centralised, reusable checks to keep routers/services lean.
"""

from __future__ import annotations

from datetime import datetime
from datetime import timezone

from fastapi import HTTPException
from fastapi import status
from sqlalchemy import func
from sqlalchemy.orm import Session

from zerg.config import get_settings
from zerg.models.models import Agent as AgentModel
from zerg.models.models import AgentRun as AgentRunModel
from zerg.models.models import User as UserModel


def _is_admin(user: UserModel | None) -> bool:
    return getattr(user, "role", "USER") == "ADMIN"


def assert_can_start_run(db: Session, *, user: UserModel) -> None:
    """Raise 429 when non-admin user exceeds DAILY_RUNS_PER_USER.

    Treat 0 or missing env as "disabled" (no limit).
    """

    settings = get_settings()
    try:
        limit = int(getattr(settings, "daily_runs_per_user", 0))
    except Exception:  # noqa: BLE001
        limit = 0

    if limit <= 0:
        return

    if _is_admin(user):
        return

    today_utc = datetime.now(timezone.utc).date()

    # Count runs started today for this user by joining AgentRun → Agent(owner)
    count_q = (
        db.query(func.count(AgentRunModel.id))
        .join(AgentModel, AgentModel.id == AgentRunModel.agent_id)
        .filter(
            AgentModel.owner_id == user.id,
            AgentRunModel.started_at.isnot(None),
            func.date(AgentRunModel.started_at) == today_utc,
        )
    )
    used = int(count_q.scalar() or 0)

    if used >= limit:
        raise HTTPException(
            status_code=status.HTTP_429_TOO_MANY_REQUESTS,
            detail=f"Daily run limit reached ({used}/{limit}). Try again tomorrow or contact admin.",
        )
