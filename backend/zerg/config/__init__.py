"""Centralised configuration helper (no external dependencies).

This module eliminates scattered ``os.getenv`` calls by exposing a **single**
process-wide :class:`Settings` instance (retrieved via :func:`get_settings`).

We intentionally **avoid** a runtime dependency on *pydantic-settings* to keep
the core backend lightweight – a bespoke implementation is less than 100 LOC
while covering all current needs.
"""

from __future__ import annotations

import functools
import os
from dataclasses import dataclass
from pathlib import Path
from typing import Any

_BASE_DIR = Path(__file__).resolve().parent.parent.parent


def _truthy(value: str | None) -> bool:  # noqa: D401 – small helper
    """Return *True* when *value* looks like an affirmative string."""

    if value is None:
        return False
    return value.strip().lower() in {"1", "true", "yes", "on"}


@dataclass
class Settings:  # noqa: D401 – simple data container
    """Lightweight settings container populated from environment variables."""

    # Runtime flags -----------------------------------------------------
    testing: bool
    auth_disabled: bool

    # Secrets & IDs -----------------------------------------------------
    jwt_secret: str
    google_client_id: Any
    google_client_secret: Any
    trigger_signing_secret: Any

    # Database ---------------------------------------------------------
    database_url: str

    # Cryptography -----------------------------------------------------
    fernet_secret: Any

    # Feature flags ----------------------------------------------------
    _llm_token_stream_default: bool  # internal default

    # Misc
    dev_admin: bool
    log_level: str
    e2e_log_suppress: bool
    environment: Any
    allowed_cors_origins: str
    openai_api_key: Any

    # ------------------------------------------------------------------
    # Dynamic feature helper – evaluates *each time* so tests that tweak the
    # env var at runtime still propagate.
    # ------------------------------------------------------------------

    @property
    def llm_token_stream(self) -> bool:  # noqa: D401 – convenience
        return _truthy(os.getenv("LLM_TOKEN_STREAM")) or self._llm_token_stream_default

    # Helper for tests to override values at runtime -------------------
    def override(self, **kwargs: Any) -> None:  # pragma: no cover – test util
        for key, value in kwargs.items():
            if not hasattr(self, key):  # pragma: no cover – safety
                raise AttributeError(f"Settings has no attribute '{key}'")
            setattr(self, key, value)


# ---------------------------------------------------------------------------
# Singleton accessor – values loaded only once per interpreter
# ---------------------------------------------------------------------------


def _load_settings() -> Settings:  # noqa: D401 – helper
    """Populate :class:`Settings` from environment variables."""

    testing = _truthy(os.getenv("TESTING"))

    # Respect .env file in repo-root for local development
    env_path = _BASE_DIR / ".env"
    if env_path.exists():
        for line in env_path.read_text().splitlines():
            if not line or line.strip().startswith("#") or "=" not in line:
                continue
            key, val = line.split("=", 1)
            os.environ.setdefault(key.strip(), val.strip())

    return Settings(
        testing=testing,
        auth_disabled=_truthy(os.getenv("AUTH_DISABLED")) or testing,
        jwt_secret=os.getenv("JWT_SECRET", "dev-secret"),
        google_client_id=os.getenv("GOOGLE_CLIENT_ID"),
        google_client_secret=os.getenv("GOOGLE_CLIENT_SECRET"),
        trigger_signing_secret=os.getenv("TRIGGER_SIGNING_SECRET"),
        database_url=os.getenv("DATABASE_URL", ""),
        fernet_secret=os.getenv("FERNET_SECRET"),
        _llm_token_stream_default=_truthy(os.getenv("LLM_TOKEN_STREAM")),
        dev_admin=_truthy(os.getenv("DEV_ADMIN")),
        log_level=os.getenv("LOG_LEVEL", "INFO"),
        e2e_log_suppress=_truthy(os.getenv("E2E_LOG_SUPPRESS")),
        environment=os.getenv("ENVIRONMENT"),
        allowed_cors_origins=os.getenv("ALLOWED_CORS_ORIGINS", ""),
        openai_api_key=os.getenv("OPENAI_API_KEY"),
    )


@functools.lru_cache(maxsize=1)
def get_settings() -> Settings:  # noqa: D401 – public accessor
    """Return **singleton** :class:`Settings` instance (cached)."""

    return _load_settings()


__all__ = [
    "Settings",
    "get_settings",
]
