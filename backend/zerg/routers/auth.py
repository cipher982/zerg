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
from zerg.dependencies.auth import get_current_user

load_dotenv()

router = APIRouter(prefix="/auth", tags=["auth"])


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


GOOGLE_CLIENT_ID = os.getenv("GOOGLE_CLIENT_ID")
JWT_SECRET = os.getenv("JWT_SECRET", "dev-secret")

# Client secret is required for *server-side* OAuth code exchange when users
# connect their Gmail account to enable *email triggers*.  The variable is
# intentionally **optional** so that the regular Google Sign-In flow (which
# only needs the *client ID* for ID-token verification) keeps working in
# existing development setups.

GOOGLE_CLIENT_SECRET = os.getenv("GOOGLE_CLIENT_SECRET")


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

    from datetime import timezone

    expiry = datetime.now(timezone.utc) + expires_delta
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

    # ``exp`` must be an **integer** UNIX timestamp so that json.dumps (used
    # by the lightweight fallback encoder below) can serialise the payload
    # without hitting "Object of type datetime is not JSON serialisable".

    payload = {
        "sub": str(user_id),
        "email": email,
        "exp": int(expiry.timestamp()),
    }
    return jwt.encode(payload, JWT_SECRET, algorithm="HS256")


# ---------------------------------------------------------------------------
# Gmail *offline_access* helper – Phase-2 Email Triggers
# ---------------------------------------------------------------------------


def _exchange_google_auth_code(auth_code: str, *, redirect_uri: str | None = None) -> dict[str, str]:
    """Exchange an *authorization code* for tokens via Google's OAuth endpoint.

    The function returns the full token response as a dictionary.  We are
    primarily interested in the ``refresh_token`` which we persist so that the
    forthcoming *Email Trigger Service* can fetch short-lived access tokens on
    demand.

    The call requires a **client secret** which must be provided via the
    ``GOOGLE_CLIENT_SECRET`` environment variable.  In unit-tests the function
    is usually monkey-patched so no external HTTP call is performed.
    """

    if GOOGLE_CLIENT_ID is None or GOOGLE_CLIENT_SECRET is None:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail="Google OAuth client not configured",
        )

    import json
    import urllib.parse
    import urllib.request

    token_endpoint = "https://oauth2.googleapis.com/token"

    data = {
        "code": auth_code,
        "client_id": GOOGLE_CLIENT_ID,
        "client_secret": GOOGLE_CLIENT_SECRET,
        "grant_type": "authorization_code",
        # The recommended value for SPA → backend hand-off is ``postmessage``.
        # Callers can override by passing an explicit redirect_uri.
        "redirect_uri": redirect_uri or "postmessage",
    }

    # Encode as application/x-www-form-urlencoded
    encoded = urllib.parse.urlencode(data).encode()

    req = urllib.request.Request(
        token_endpoint,
        data=encoded,
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:  # nosec B310 – trusted URL
            payload = json.loads(resp.read().decode())
    except Exception as exc:  # pragma: no cover – network failure mapped to 502
        raise HTTPException(status_code=status.HTTP_502_BAD_GATEWAY, detail="Google token exchange failed") from exc

    if "refresh_token" not in payload:
        # The most common reason is that *offline* access was not requested or
        # the user previously granted access.  We surface a descriptive error
        # so the frontend can re-prompt with `prompt=consent`.
        raise HTTPException(status_code=status.HTTP_400_BAD_REQUEST, detail="No refresh_token in Google response")

    return payload  # Contains refresh_token, access_token, expires_in, scope, …


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


# ---------------------------------------------------------------------------
# Gmail connection endpoint (Phase-2 Email Triggers)
# ---------------------------------------------------------------------------


@router.post("/google/gmail", status_code=status.HTTP_200_OK)
def connect_gmail(
    body: dict[str, str],
    db: Session = Depends(get_db),
    current_user: Any = Depends(get_current_user),
) -> dict[str, str]:
    """Store *offline* Gmail permissions for the **current** user.

    Expected body: ``{ "auth_code": "<code from OAuth consent window>" }``.

    The frontend must request the following when launching the consent screen::

        scope=https://www.googleapis.com/auth/gmail.readonly
        access_type=offline
        prompt=consent

    The *refresh token* returned by Google is stored on the user row.  The
    endpoint returns a simple JSON confirmation so the client knows the
    account is connected.
    """

    auth_code = body.get("auth_code")
    if not auth_code:
        raise HTTPException(status_code=status.HTTP_422_UNPROCESSABLE_ENTITY, detail="auth_code missing")

    # 1. Exchange code for tokens (patched to stub out in unit-tests)
    token_payload = _exchange_google_auth_code(auth_code)

    refresh_token: str = token_payload["refresh_token"]

    # 2. Persist on current user row
    updated = crud.update_user(
        db,
        current_user.id,
        gmail_refresh_token=refresh_token,
    )
    if updated is None:
        raise HTTPException(status_code=status.HTTP_404_NOT_FOUND, detail="User not found")

    return {"status": "connected"}
