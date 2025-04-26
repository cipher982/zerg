"""API route configuration."""

# Base prefix for all API routes
API_PREFIX = "/api"

# WebSocket endpoint (relative to API_PREFIX)
WS_ENDPOINT = "/ws"

# Router prefixes (relative to API_PREFIX)
AGENTS_PREFIX = "/agents"
THREADS_PREFIX = "/threads"
MODELS_PREFIX = "/models"


def get_full_path(relative_path: str) -> str:
    """Get the full API path for a relative path."""
    return f"{API_PREFIX}{relative_path}"
