"""System configuration & feature-flag endpoints (public).

Provides a *single* unauthenticated JSON endpoint so that the WASM frontend
can discover runtime flags (currently only `auth_disabled`) without the
developer having to keep environment variables in sync between the Python
and Rust build steps.
"""

from typing import Any
from typing import Dict

from fastapi import APIRouter
from fastapi import status

from zerg.config import get_settings

router = APIRouter(prefix="/system", tags=["system"])

_settings = get_settings()


@router.get("/info", status_code=status.HTTP_200_OK)
def system_info() -> Dict[str, Any]:
    """Return non-sensitive runtime switches used by the SPA at startup."""

    return {
        "auth_disabled": _settings.auth_disabled,
        "google_client_id": _settings.google_client_id,
    }
