"""User profile routes.

Currently only *self-service* endpoints are exposed ("/users/me").  All
routes require authentication so we rely on the existing `get_current_user`
dependency to supply the active user.
"""

from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import status
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db

# Auth guard ---------------------------------------------------------------
from zerg.dependencies.auth import get_current_user
from zerg.events import EventType
from zerg.events.decorators import publish_event
from zerg.schemas.schemas import UserOut
from zerg.schemas.schemas import UserUpdate

router = APIRouter(tags=["users"], dependencies=[Depends(get_current_user)])


# ---------------------------------------------------------------------------
# /users/me – retrieve current profile
# ---------------------------------------------------------------------------


@router.get("/users/me", response_model=UserOut)
def read_current_user(current_user=Depends(get_current_user)):
    """Return the authenticated user's profile."""

    return current_user  # SQLAlchemy row – FastAPI will use attrs to dict


# ---------------------------------------------------------------------------
# /users/me – partial update
# ---------------------------------------------------------------------------


@router.put("/users/me", response_model=UserOut)
@publish_event(EventType.USER_UPDATED)
async def update_current_user(
    patch: UserUpdate,
    db: Session = Depends(get_db),
    current_user=Depends(get_current_user),
):
    """Patch the authenticated user's profile (display name, avatar, prefs)."""

    updated = crud.update_user(
        db,
        current_user.id,
        display_name=patch.display_name,
        avatar_url=patch.avatar_url,
        prefs=patch.prefs,
    )

    if updated is None:
        # Should not happen if auth dependency returned a valid row.
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")

    return updated
