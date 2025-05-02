"""Authentication routes (Stage 2 – Google Sign-In)."""

import os
from datetime import datetime
from datetime import timedelta
from typing import Any

from dotenv import load_dotenv
from fastapi import APIRouter
from fastapi import Depends
from fastapi import HTTPException
from fastapi import status
from sqlalchemy.orm import Session

from zerg.crud import crud
from zerg.database import get_db
from zerg.schemas.schemas import TokenOut

load_dotenv()

router = APIRouter(prefix="/auth", tags=["auth"])


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


GOOGLE_CLIENT_ID = os.getenv("GOOGLE_CLIENT_ID")
JWT_SECRET = os.getenv("JWT_SECRET", "dev-secret")


def _verify_google_id_token(id_token_str: str) -> dict[str, Any]:
    """Validate the JWT issued by Google and return the decoded claims.

    Raises HTTPException(401) if the token is invalid or the *aud* claim does
    not match our configured client ID.
    """

    if not GOOGLE_CLIENT_ID:
        raise HTTPException(status_code=status.HTTP_500_INTERNAL_SERVER_ERROR, detail="GOOGLE_CLIENT_ID not set")

    try:
        # Import google-auth lazily to keep optional.
        from google.auth.transport import requests as google_requests  # type: ignore
        from google.oauth2 import id_token  # type: ignore

        # verify_oauth2_token does signature, expiration, issuer & audience.
        request = google_requests.Request()  # Re-usable HTTP transport
        idinfo = id_token.verify_oauth2_token(id_token_str, request, GOOGLE_CLIENT_ID)
        return idinfo
    except Exception as exc:  # broad but we map to 401 below
        raise HTTPException(status_code=status.HTTP_401_UNAUTHORIZED, detail="Invalid Google token") from exc


def _issue_access_token(user_id: int, email: str, expires_delta: timedelta = timedelta(minutes=30)) -> str:
    """Return signed HS256 access token."""

    expiry = datetime.utcnow() + expires_delta
    # Import python-jose lazily to avoid hard dependency during unit tests.
    try:
        from jose import jwt  # type: ignore
    except ModuleNotFoundError:  # pragma: no cover – fallback minimal signer
        import base64
        import hashlib
        import hmac
        import json

        class _MiniJWT:
            @staticmethod
            def _b64(data: bytes) -> bytes:
                return base64.urlsafe_b64encode(data).rstrip(b"=")

            @classmethod
            def encode(cls, payload_: dict[str, Any], secret_: str, algorithm: str = "HS256") -> str:  # noqa: D401 – purpose
                if algorithm != "HS256":
                    raise ValueError("Only HS256 supported in fallback")

                header = {"alg": algorithm, "typ": "JWT"}
                header_b64 = cls._b64(json.dumps(header, separators=(",", ":")).encode())
                payload_b64 = cls._b64(json.dumps(payload_, separators=(",", ":")).encode())
                signing_input = header_b64 + b"." + payload_b64
                signature = hmac.new(secret_.encode(), signing_input, hashlib.sha256).digest()
                sig_b64 = cls._b64(signature)
                return (signing_input + b"." + sig_b64).decode()

        jwt = _MiniJWT  # type: ignore

    payload = {
        "sub": str(user_id),
        "email": email,
        "exp": expiry,
    }
    return jwt.encode(payload, JWT_SECRET, algorithm="HS256")


# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------


@router.post("/google", response_model=TokenOut)
def google_sign_in(body: dict[str, str], db: Session = Depends(get_db)) -> TokenOut:  # noqa: D401 – simple name
    """Exchange a Google ID token for a platform access token.

    Expected JSON body: `{ "id_token": "<JWT from Google>" }`.
    """

    raw_token = body.get("id_token")
    if not raw_token or not isinstance(raw_token, str):
        raise HTTPException(status_code=status.HTTP_422_UNPROCESSABLE_ENTITY, detail="id_token must be provided")

    # 1. Validate & decode Google token
    claims = _verify_google_id_token(raw_token)

    # 2. Upsert (or fetch) user record
    email: str = claims.get("email")  # type: ignore[assignment]
    sub: str = claims.get("sub")  # stable Google user id

    if not email:
        # Should never happen for verified google accounts
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="Google token missing email claim")

    user = crud.get_user_by_email(db, email)
    if not user:
        user = crud.create_user(db, email=email, provider="google", provider_user_id=sub)

    # 3. Issue platform JWT
    access_token = _issue_access_token(user.id, user.email)

    # 4. Return response
    return TokenOut(access_token=access_token, expires_in=30 * 60)
