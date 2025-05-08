"""Light-weight Gmail REST helpers used by *EmailTriggerService*.

All functions are *safe* for the CI environment which has **no external
network access**:

* If the outgoing HTTPS request fails (e.g. due to DNS block) we log and
  return a *neutral* value so the caller can gracefully skip processing.

* Unit-tests that need deterministic responses can monkey-patch the public
  helpers – this pattern is already used widely across the test-suite.

The module purposefully keeps **zero third-party dependencies** – we re-use
`urllib.request` for the tiny HTTP calls to avoid bloating the runtime.
"""

from __future__ import annotations

import json
import logging
import os
import urllib.parse
import urllib.request
from typing import Any
from typing import Dict
from typing import List

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# OAuth helpers
# ---------------------------------------------------------------------------


def exchange_refresh_token(refresh_token: str) -> str:  # noqa: D401 – helper
    """Swap a *refresh_token* for a short-lived *access_token*.

    Follows Google OAuth 2.0 spec.  Raises ``RuntimeError`` on failure so the
    caller can handle back-off or retry.  The function **does not** attempt
    automatic retries because the service layer already loops regularly.
    """

    client_id = os.getenv("GOOGLE_CLIENT_ID")
    client_secret = os.getenv("GOOGLE_CLIENT_SECRET")

    if not client_id or not client_secret:
        raise RuntimeError(
            "GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET must be set to refresh tokens",
        )

    data = {
        "client_id": client_id,
        "client_secret": client_secret,
        "refresh_token": refresh_token,
        "grant_type": "refresh_token",
    }

    encoded = urllib.parse.urlencode(data).encode()

    req = urllib.request.Request(
        "https://oauth2.googleapis.com/token",
        data=encoded,
        headers={"Content-Type": "application/x-www-form-urlencoded"},
    )

    try:
        with urllib.request.urlopen(req, timeout=10) as resp:  # nosec B310
            payload: Dict[str, Any] = json.loads(resp.read().decode())
    except Exception as exc:  # pragma: no cover – network error
        logger.warning("Token refresh network failure: %s", exc)
        raise RuntimeError("token endpoint request failed") from exc

    access_token = payload.get("access_token")
    if not access_token:
        raise RuntimeError(f"invalid token response: {payload}")

    return access_token  # noqa: WPS331 – explicit return value


# ---------------------------------------------------------------------------
# Gmail History helpers
# ---------------------------------------------------------------------------


def _make_request(url: str, access_token: str):  # noqa: D401 – tiny helper
    req = urllib.request.Request(url, headers={"Authorization": f"Bearer {access_token}"})
    return urllib.request.urlopen(req, timeout=10)  # nosec B310


def list_history(access_token: str, start_history_id: int) -> List[Dict[str, Any]]:  # noqa: D401 – helper
    """Return *raw* history records newer than ``start_history_id``.

    The function automatically handles paging via ``nextPageToken`` and stops
    as soon as no further pages are indicated.  If any network error occurs
    an *empty list* is returned so the caller can degrade gracefully.
    """

    url_base = (
        "https://gmail.googleapis.com/gmail/v1/users/me/history"
        f"?startHistoryId={start_history_id}"
        "&historyTypes=messageAdded"
        "&maxResults=100"
    )

    history: List[Dict[str, Any]] = []
    page_token: str | None = None

    while True:  # pagination loop
        url = url_base + (f"&pageToken={page_token}" if page_token else "")

        try:
            with _make_request(url, access_token) as resp:  # type: ignore[attr-defined]
                payload: Dict[str, Any] = json.loads(resp.read().decode())
        except Exception as exc:  # pragma: no cover – network offline
            logger.warning("list_history network failure: %s", exc)
            return []  # graceful degradation

        history.extend(payload.get("history", []))

        page_token = payload.get("nextPageToken")
        if not page_token:
            break

    return history


def get_message_metadata(access_token: str, msg_id: str) -> Dict[str, Any]:  # noqa: D401 – helper
    """Fetch *minimal* metadata (From, Subject, labelIds) for ``msg_id``.

    On any error we return an **empty dict** so higher-level code can treat it
    as non-matching and continue.
    """

    url = (
        "https://gmail.googleapis.com/gmail/v1/users/me/messages/"
        f"{msg_id}?format=metadata&metadataHeaders=From&metadataHeaders=Subject"
    )

    try:
        with _make_request(url, access_token) as resp:  # type: ignore[attr-defined]
            payload: Dict[str, Any] = json.loads(resp.read().decode())
    except Exception as exc:  # pragma: no cover – offline
        logger.warning("get_message_metadata network failure: %s", exc)
        return {}

    # Normalise headers list into a dict {Header: value}
    headers_list = payload.get("payload", {}).get("headers", [])
    headers_dict: Dict[str, str] = {h.get("name"): h.get("value") for h in headers_list if h.get("name")}

    return {
        "id": payload.get("id"),
        "labelIds": payload.get("labelIds", []),
        "headers": headers_dict,
    }
