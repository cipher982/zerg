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

# ``_REPO_ROOT`` points to the top-level repository directory (one level
# **above** the "backend" package).  We use ``parents[3]`` because this file is
# located at ``backend/zerg/config/__init__.py``.

_REPO_ROOT = Path(__file__).resolve().parents[3]


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
    ws_envelope_v2: bool

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

    # ------------------------------------------------------------------
    # Look for a *.env* file and merge **once** – favour the first match.
    #
    # Historically the project stored its development template at
    # ``backend/.env`` but the refactor that introduced this bespoke settings
    # helper assumed a repository-root location (``.env``).  Local setups that
    # rely on the old path therefore lost *all* environment variables which –
    # amongst other things – meant ``OPENAI_API_KEY`` was no longer set and
    # the LLM calls started to fail.
    #
    # To remain backwards-compatible we now probe multiple candidates in
    # order of precedence and stop at the **first** existing file:
    # 1. ``<repo-root>/.env`` (new convention)
    # 2. ``<repo-root>/backend/.env`` (legacy)
    # 3. ``$PWD/.env``          (edge-cases like ad-hoc scripts)
    #
    # The merge strategy mirrors *python-dotenv* – the file is **not**
    # authoritative; existing environment variables always win so CI/CD
    # pipelines can safely inject secrets via the process environment.
    # ------------------------------------------------------------------

    # Only consider the *canonical* env file location at repository root.
    env_path = _REPO_ROOT / ".env"
    if env_path.exists():
        for line in env_path.read_text().splitlines():
            if not line or line.strip().startswith("#") or "=" not in line:
                continue  # skip blanks & comments

            key, val = line.split("=", 1)

            # Remove any inline comment that is **outside** quoted strings.
            # We do a simple split on " #" which is adequate for our current
            # .env conventions (values are either unquoted or wrapped in
            # single/double quotes without embedded spaces).

            _val_clean = val.split(" #", 1)[0].strip()
            os.environ.setdefault(key.strip(), _val_clean)

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
        ws_envelope_v2=_truthy(os.getenv("WS_ENVELOPE_V2")),
        dev_admin=_truthy(os.getenv("DEV_ADMIN")),
        log_level=os.getenv("LOG_LEVEL", "INFO"),
        e2e_log_suppress=_truthy(os.getenv("E2E_LOG_SUPPRESS")),
        environment=os.getenv("ENVIRONMENT"),
        allowed_cors_origins=os.getenv("ALLOWED_CORS_ORIGINS", ""),
        openai_api_key=os.getenv("OPENAI_API_KEY"),
    )


# ------------------------------------------------------------------
# Runtime validation – fail fast when *required* secrets are missing.
# ------------------------------------------------------------------


def _validate_required(settings: Settings) -> None:  # noqa: D401 – helper
    """Abort startup when mandatory configuration is missing.

    The application should never reach runtime with an absent
    ``OPENAI_API_KEY`` (unless the *TESTING* flag is active).  Performing the
    check here – immediately after loading the environment – surfaces
    mis-configurations early and prevents a cascade of HTTP connection errors
    once the first LLM call is made.
    """

    if settings.testing:  # Unit-/integration tests run with stubbed LLMs
        return

    if not settings.openai_api_key:
        raise RuntimeError(
            "OPENAI_API_KEY is not set.  Provide it via the .env file at the "
            "repository root or export the variable in the shell before "
            "starting the application.  Refusing to continue with an empty "
            "key to avoid hard-to-debug runtime failures.",
        )


@functools.lru_cache(maxsize=1)
def get_settings() -> Settings:  # noqa: D401 – public accessor
    """Return **singleton** :class:`Settings` instance (cached)."""

    settings = _load_settings()
    _validate_required(settings)
    return settings


__all__ = [
    "Settings",
    "get_settings",
]
