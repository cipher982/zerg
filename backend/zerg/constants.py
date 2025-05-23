import os

# Base API prefix (all HTTP routes are served under /api/*)
API_PREFIX = "/api"

# WebSocket endpoint – mounted under API_PREFIX
WS_ENDPOINT = "/ws"

# Router prefixes (relative to API_PREFIX)
AGENTS_PREFIX = "/agents"
THREADS_PREFIX = "/threads"
MODELS_PREFIX = "/models"

# ---------------------------------------------------------------------------
# Trigger HMAC signing secret – used by /api/triggers/{id}/events HMAC
# verification (Stage 5 of auth hardening roadmap).
# ---------------------------------------------------------------------------

# ---------------------------------------------------------------------------
# In production deployments users *must* provide an explicit
# ``TRIGGER_SIGNING_SECRET``.  Tests, local dev setups or documentation
# generation, however, should not break just because the environment variable
# is missing.  We therefore fall back to a deterministic placeholder when the
# variable is absent **and** the ``TESTING`` flag is set.
# ---------------------------------------------------------------------------

if "TRIGGER_SIGNING_SECRET" in os.environ:
    TRIGGER_SIGNING_SECRET: str = os.environ["TRIGGER_SIGNING_SECRET"]
else:
    if os.getenv("TESTING") == "1":
        # Deterministic value – identical to fixture used elsewhere so HMAC
        # computations remain reproducible across the suite.
        TRIGGER_SIGNING_SECRET = "super-secret-hex"
    else:
        raise KeyError(
            "TRIGGER_SIGNING_SECRET is not set. This secret is required for "
            "secure webhook triggering in production deployments."
        )

# Accept ±5 minutes clock skew for HMAC timestamp header
TRIGGER_TIMESTAMP_TOLERANCE_S: int = 300


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
