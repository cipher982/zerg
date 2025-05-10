"""FastAPI dependency that resolves the *current user*.

Stage 3 of the authentication roadmap introduces a lightweight auth guard for
all API routes (except the public */models* and */auth* endpoints).  The logic
is intentionally simple:

1. If the environment variable ``AUTH_DISABLED`` is truthy ("1", "true",
   "yes") we *bypass* authentication altogether.  A deterministic *dev user*
   row (email ``dev@local``) is created on first access and returned for every
   request.  This keeps local development friction-free.

2. Otherwise we expect an ``Authorization: Bearer <jwt>`` header that carries
   the **platform access token** previously issued by ``/api/auth/google``.
   The token is verified with *python-jose* (HS256, secret from
   ``JWT_SECRET``).  On success we load the corresponding user row from the
   database and return it.  Any failure raises **HTTP 401**.

The module avoids a hard dependency on *python-jose* – a tiny fallback JWT
decoder is shipped so unit-tests can run without the extra wheel.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
from datetime import datetime
from typing import Any

from fastapi import Depends
from fastapi import HTTPException
from fastapi import Request
from fastapi import status
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db

# ---------------------------------------------------------------------------
# Configuration – read once at import time
# ---------------------------------------------------------------------------


JWT_SECRET = os.getenv("JWT_SECRET", "dev-secret")

# Normalise AUTH_DISABLED to boolean – accepts "1", "true", "yes" (case-ins).
_AUTH_DISABLED_RAW = os.getenv("AUTH_DISABLED", "0").lower()
AUTH_DISABLED = _AUTH_DISABLED_RAW in {"1", "true", "yes"}

# E-mail used for the implicit development user when auth is disabled.
DEV_EMAIL = "dev@local"


# ---------------------------------------------------------------------------
# Minimal HS256 JWT helper (used as fallback if python-jose is unavailable)
# ---------------------------------------------------------------------------


def _b64url_decode(data: str) -> bytes:  # pragma: no cover – helper
    """Decode *URL-safe* base64, adding padding if necessary."""

    padding = "=" * (-len(data) % 4)
    return base64.urlsafe_b64decode(data + padding)


def _decode_jwt_fallback(token: str, secret: str) -> dict[str, Any]:  # pragma: no cover
    """Very small HS256 decoder/validator (no external deps).

    • *Only* supports unsigned integers for the ``exp`` claim (as UNIX ts).
    • Raises ValueError on any validation error.
    """

    try:
        header_b64, payload_b64, signature_b64 = token.split(".")
    except ValueError as exc:
        raise ValueError("Invalid JWT structure") from exc

    signing_input = f"{header_b64}.{payload_b64}".encode()
    signature = _b64url_decode(signature_b64)

    expected_sig = hmac.new(secret.encode(), signing_input, hashlib.sha256).digest()
    if not hmac.compare_digest(signature, expected_sig):
        raise ValueError("Invalid signature")

    payload_raw = _b64url_decode(payload_b64)
    try:
        payload: dict[str, Any] = json.loads(payload_raw)
    except json.JSONDecodeError as exc:
        raise ValueError("Invalid payload JSON") from exc

    # Very small ``exp`` handling – reject if expired.
    exp_ts = payload.get("exp")
    if exp_ts is not None and isinstance(exp_ts, (int, float)):
        if datetime.utcnow().timestamp() > float(exp_ts):
            raise ValueError("Token expired")

    return payload


# ---------------------------------------------------------------------------
# Public dependency
# ---------------------------------------------------------------------------


def get_current_user(request: Request, db: Session = Depends(get_db)):
    """Resolve and return the *current User* row.

    • In *dev mode* (``AUTH_DISABLED``) we create/return a fixed dummy user.
    • Otherwise we expect an *Authorization* header with `Bearer <jwt>`.
    """

    # --------------------------------------------------
    # 1. Development bypass
    # --------------------------------------------------
    if AUTH_DISABLED:
        return _get_or_create_dev_user(db)

    # --------------------------------------------------
    # 2. Extract token from *Authorization* header
    # --------------------------------------------------
    auth_header: str | None = request.headers.get("Authorization")
    if not auth_header or not auth_header.lower().startswith("bearer "):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")

    token = auth_header[7:].strip()
    if not token:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")

    # --------------------------------------------------
    # 3. Decode + validate JWT
    # --------------------------------------------------
    try:
        # Prefer python-jose if installed.
        try:
            from jose import jwt  # type: ignore

            payload: dict[str, Any] = jwt.decode(token, JWT_SECRET, algorithms=["HS256"])
        except ModuleNotFoundError:
            payload = _decode_jwt_fallback(token, JWT_SECRET)

    except Exception:  # broad – we translate any error to 401
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or expired token")

    # --------------------------------------------------
    # 4. Lookup user record
    # --------------------------------------------------
    user_id_claim = payload.get("sub")
    if user_id_claim is None:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token claims")

    try:
        user_id = int(user_id_claim)
    except (TypeError, ValueError):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token subject")

    user = crud.get_user(db, user_id)
    if user is None or not user.is_active:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="User not found or inactive")

    # -------------------------------------------------------------------
    # Update *last_login* timestamp in the background – we persist lazily to
    # avoid extra DB writes on every authenticated request while still
    # keeping reasonable accuracy.
    # -------------------------------------------------------------------

    from datetime import datetime as _dt  # local import to avoid cycles

    if getattr(user, "last_login", None) is None:
        user.last_login = _dt.utcnow()
        db.commit()

    return user


# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _get_or_create_dev_user(db: Session):
    """Return a deterministic development user (create if missing)."""

    user = crud.get_user_by_email(db, DEV_EMAIL)
    if user is not None:
        return user

    return crud.create_user(db, email=DEV_EMAIL, provider=None)


# ---------------------------------------------------------------------------
# Helper for WebSocket authentication (Stage 8)
# ---------------------------------------------------------------------------


def validate_ws_jwt(token: str | None, db: Session):
    """Validate JWT passed as ``?token=…`` in WebSocket URL.

    Mirrors :pyfunc:`get_current_user` but works in the *WebSocket* context
    where we cannot rely on an HTTP *Authorization* header.  Returns the
    resolved ``User`` row or raises ``HTTPException`` *(401)*.
    """

    # 1. Development bypass -------------------------------------------------
    if AUTH_DISABLED:
        return _get_or_create_dev_user(db)

    # 2. Require a token ----------------------------------------------------
    if not token:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Missing bearer token")

    # 3. Decode / verify  ---------------------------------------------------
    try:
        # Prefer python-jose if available.
        try:
            from jose import jwt  # type: ignore

            payload: dict[str, Any] = jwt.decode(token, JWT_SECRET, algorithms=["HS256"])
        except ModuleNotFoundError:
            payload = _decode_jwt_fallback(token, JWT_SECRET)

    except Exception:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid or expired token")

    # 4. Lookup user --------------------------------------------------------
    user_id_claim = payload.get("sub")
    if user_id_claim is None:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token claims")

    try:
        user_id = int(user_id_claim)
    except (TypeError, ValueError):
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid token subject")

    user = crud.get_user(db, user_id)
    if user is None or not user.is_active:
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="User not found or inactive")

    return user
