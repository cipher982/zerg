"""Authentication strategy abstraction for the Agent Platform backend.

This module formalises the authentication flow behind a small *strategy*
interface so that the actual logic can be swapped depending on the runtime
configuration (development bypass vs. production JWT validation).

The previous implementation in ``zerg.dependencies.auth`` mixed two modes in
conditionals which complicated unit-testing and violated *single-responsibility*.
By extracting **DevAuthStrategy** and **JWTAuthStrategy** into discrete
classes we can now:

• Decide once at *startup* which branch to use – no per-request branching.
• Monkey-patch :pydata:`zerg.dependencies.auth._strategy` in tests to inject
  custom behaviour.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
from abc import ABC
from abc import abstractmethod
from datetime import datetime
from typing import Any
from typing import Optional

from fastapi import HTTPException
from fastapi import Request
from fastapi import status
from sqlalchemy.orm import Session

from zerg.config import get_settings
from zerg.crud import crud

# ---------------------------------------------------------------------------
# Minimal HS256 JWT decoding fallback (keeps CI lightweight)
# ---------------------------------------------------------------------------


def _b64url_decode(data: str) -> bytes:  # pragma: no cover – helper
    """Decode *URL-safe* base64, adding padding if required."""

    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


def _decode_jwt_fallback(token: str, secret: str) -> dict[str, Any]:  # pragma: no cover
    """Very small HS256 validator used when *python-jose* is unavailable."""

    try:
        header_b64, payload_b64, signature_b64 = token.split(".")
    except ValueError as exc:  # noqa: BLE001 – malformed token
        raise ValueError("Invalid JWT structure") from exc

    signing_input = f"{header_b64}.{payload_b64}".encode()
    signature = _b64url_decode(signature_b64)
    expected_sig = hmac.new(secret.encode(), signing_input, hashlib.sha256).digest()

    if not hmac.compare_digest(signature, expected_sig):
        raise ValueError("Invalid signature")

    try:
        payload: dict[str, Any] = json.loads(_b64url_decode(payload_b64))
    except json.JSONDecodeError as exc:
        raise ValueError("Invalid payload JSON") from exc

    exp_ts_raw: Optional[float] = None
    if isinstance(payload.get("exp"), (int, float)):
        exp_ts_raw = float(payload["exp"])

    if exp_ts_raw is not None and datetime.utcnow().timestamp() > exp_ts_raw:
        raise ValueError("Token expired")

    return payload


# ---------------------------------------------------------------------------
# Strategy base-class
# ---------------------------------------------------------------------------


class AuthStrategy(ABC):
    """Pluggable authentication backend (strategy pattern)."""

    @abstractmethod
    def get_current_user(self, request: Request, db: Session):  # noqa: D401 – abstract
        """Return the authenticated user or raise **401**."""

    @abstractmethod
    def validate_ws_token(self, token: str | None, db: Session):  # noqa: D401 – abstract
        """Return user for valid token, *None* otherwise (WS handshake)."""


# ---------------------------------------------------------------------------
# Development-mode bypass
# ---------------------------------------------------------------------------


class DevAuthStrategy(AuthStrategy):
    """Bypass all checks – used when *AUTH_DISABLED* is true or in tests."""

    DEV_EMAIL = "dev@local"

    def __init__(self):
        self._settings = get_settings()

    # Internal helpers --------------------------------------------------

    def _get_or_create_dev_user(self, db: Session):
        import os

        # Skip database operations in test mode with NODE_ENV=test
        if os.getenv("NODE_ENV") == "test":
            # Return a mock user for tests to avoid database issues
            from datetime import datetime

            from zerg.models.models import User

            mock_user = User()
            mock_user.id = 1
            mock_user.email = self.DEV_EMAIL
            mock_user.role = "ADMIN" if self._settings.dev_admin else "USER"
            mock_user.is_active = True
            mock_user.provider = "dev"
            mock_user.provider_user_id = "test-user-1"
            mock_user.display_name = "Test User"
            mock_user.avatar_url = None
            mock_user.prefs = {}
            mock_user.last_login = None
            mock_user.gmail_refresh_token = None
            mock_user.created_at = datetime.utcnow()
            mock_user.updated_at = datetime.utcnow()
            return mock_user

        desired_role = "ADMIN" if self._settings.dev_admin else "USER"

        user = crud.get_user_by_email(db, self.DEV_EMAIL)
        if user is not None:
            if getattr(user, "role", "USER") != desired_role:
                user.role = desired_role  # type: ignore[attr-defined]
                db.commit()
                db.refresh(user)
            return user

        return crud.create_user(db, email=self.DEV_EMAIL, provider=None, role=desired_role)

    # Public API --------------------------------------------------------

    def get_current_user(self, request: Request, db: Session):  # noqa: D401 – impl
        auth_header = request.headers.get("Authorization")

        # If *no* header provided we still allow access for almost all paths.
        if not auth_header:
            if "/mcp-servers" in request.url.path:
                raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")
            return self._get_or_create_dev_user(db)

        # Header present → always return dev user regardless of its content.
        return self._get_or_create_dev_user(db)

    def validate_ws_token(self, token: str | None, db: Session):  # noqa: D401 – impl
        return self._get_or_create_dev_user(db)


# ---------------------------------------------------------------------------
# HS256 JWT validation (production)
# ---------------------------------------------------------------------------


class JWTAuthStrategy(AuthStrategy):
    """Production strategy that validates HS256 tokens."""

    def __init__(self):
        self._secret = get_settings().jwt_secret

    # Internal ----------------------------------------------------------

    def _decode(self, token: str) -> dict[str, Any]:  # noqa: D401 – helper
        try:
            from jose import jwt  # type: ignore

            return jwt.decode(token, self._secret, algorithms=["HS256"])
        except ModuleNotFoundError:
            return _decode_jwt_fallback(token, self._secret)

    # Public API --------------------------------------------------------

    def get_current_user(self, request: Request, db: Session):  # noqa: D401 – impl
        auth_header: str | None = request.headers.get("Authorization")
        if not auth_header or not auth_header.lower().startswith("bearer "):
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")

        token = auth_header[7:].strip()
        if not token:
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")

        try:
            payload = self._decode(token)
        except Exception:  # noqa: BLE001
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or expired token")

        try:
            user_id_int = int(payload.get("sub"))
        except Exception:  # noqa: BLE001
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token subject")

        user = crud.get_user(db, user_id_int)
        if user is None or not getattr(user, "is_active", True):
            raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="User not found or inactive")

        if getattr(user, "last_login", None) is None:
            user.last_login = datetime.utcnow()  # type: ignore[attr-defined]
            db.commit()

        return user

    def validate_ws_token(self, token: str | None, db: Session):  # noqa: D401 – impl
        if not token:
            return None

        try:
            payload = self._decode(token)
        except Exception:  # noqa: BLE001
            return None

        try:
            user_id_int = int(payload.get("sub"))
        except Exception:  # noqa: BLE001
            return None

        user = crud.get_user(db, user_id_int)
        if user is None or not getattr(user, "is_active", True):
            return None

        return user


# Public re-exports ---------------------------------------------------------


__all__ = [
    "AuthStrategy",
    "DevAuthStrategy",
    "JWTAuthStrategy",
]
