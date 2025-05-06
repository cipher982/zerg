"""System configuration & feature-flag endpoints (public).

Provides a *single* unauthenticated JSON endpoint so that the WASM frontend
can discover runtime flags (currently only `auth_disabled`) without the
developer having to keep environment variables in sync between the Python
and Rust build steps.
"""

import os
from typing import Any
from typing import Dict

from fastapi import APIRouter
from fastapi import status

router = APIRouter(prefix="/system", tags=["system"])


def _is_truthy(value: str | None) -> bool:
    """Return *True* for "1", "true", "yes" (case-insensitive)."""

    if value is None:
        return False
    return value.lower() in {"1", "true", "yes"}


@router.get("/info", status_code=status.HTTP_200_OK)
def system_info() -> Dict[str, Any]:
    """Return non-sensitive runtime switches used by the SPA at startup."""

    return {
        "auth_disabled": _is_truthy(os.getenv("AUTH_DISABLED")),
        "google_client_id": os.getenv("GOOGLE_CLIENT_ID"),
    }
