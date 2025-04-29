"""Static path constants used by FastAPI router setup.

This module is intentionally *simple*: it only defines hard-coded string
prefixes that the rest of the codebase imports.  No environment look-ups, no
dynamic logic – just constants.
"""

# Base API prefix (all HTTP routes are served under /api/*)
API_PREFIX = "/api"

# WebSocket endpoint – mounted under API_PREFIX
WS_ENDPOINT = "/ws"

# Router prefixes (relative to API_PREFIX)
AGENTS_PREFIX = "/agents"
THREADS_PREFIX = "/threads"
MODELS_PREFIX = "/models"


def get_full_path(relative_path: str) -> str:  # noqa: D401 – tiny helper
    """Return absolute API path by joining *relative_path* onto API_PREFIX."""

    return f"{API_PREFIX}{relative_path}"


__all__ = [
    "API_PREFIX",
    "WS_ENDPOINT",
    "AGENTS_PREFIX",
    "THREADS_PREFIX",
    "MODELS_PREFIX",
    "get_full_path",
]
