"""Graph layout & canvas persistence endpoints.

This is a **placeholder** router that will be expanded in a later phase.
For now it merely accepts the batched layout payload sent by the frontend
every few hundred milliseconds once the user stops dragging/zooming the
canvas and responds with HTTP 204 (no-content).

Keeping the stub in place unblocks the frontend work while we design the
proper persistence model (likely a dedicated `canvas_layouts` table).
"""

from __future__ import annotations

from typing import Dict
from typing import Optional

from fastapi import APIRouter
from fastapi import Response
from fastapi import status
from pydantic import BaseModel
from pydantic import Field
from pydantic import validator

router = APIRouter(prefix="/graph", tags=["canvas"])


class NodePos(BaseModel):
    x: float = Field(..., ge=-10_000.0, le=10_000.0)
    y: float = Field(..., ge=-10_000.0, le=10_000.0)


class Viewport(BaseModel):
    x: float
    y: float
    zoom: float = Field(..., ge=0.1, le=10.0)


class LayoutUpdate(BaseModel):
    nodes: Dict[str, NodePos]
    viewport: Optional[Viewport] = None

    @validator("nodes")
    def check_node_count(cls, v):  # noqa: N805 – Pydantic naming rule
        if len(v) > 5_000:
            raise ValueError("payload too large – max 5000 nodes")
        return v


@router.patch("/layout", status_code=status.HTTP_204_NO_CONTENT)
async def patch_layout(_payload: LayoutUpdate) -> Response:  # noqa: D401 – simple stub
    """Persist batched canvas layout (stub).

    Future work: store in DB, broadcast over WebSocket so other tabs update.
    """

    # For the prototype we simply acknowledge the request so the frontend can
    # consider its save successful.
    return Response(status_code=status.HTTP_204_NO_CONTENT)
