"""Simple *Fernet* (AES-GCM) encryption helper for Gmail refresh tokens.

The project mandates `cryptography` as a hard dependency, therefore we no
longer carry the legacy XOR fallback.  If the import fails the application
exits early with a helpful message.
"""

from __future__ import annotations

import os
from typing import Final

try:
    from cryptography.fernet import Fernet  # type: ignore
except ModuleNotFoundError as exc:  # pragma: no cover – mandatory dep missing
    raise SystemExit(
        "cryptography package is required.  Install it via\n"
        "  uv pip install cryptography\n"
        "before running the backend."
    ) from exc


# ---------------------------------------------------------------------------
# Load and validate secret
# ---------------------------------------------------------------------------

_SECRET_B64: Final[str | None] = os.getenv("FERNET_SECRET")

if not _SECRET_B64:
    raise SystemExit("FERNET_SECRET environment variable must be set.")


try:
    _fernet = Fernet(_SECRET_B64.encode())
except Exception as exc:  # pragma: no cover – bad key format
    raise SystemExit("FERNET_SECRET is not a valid url-safe base64 32-byte key") from exc


# ---------------------------------------------------------------------------
# Public encryption helpers
# ---------------------------------------------------------------------------


def encrypt(text: str) -> str:  # noqa: D401 – thin wrapper
    """Encrypt *text* using AES-GCM and return url-safe base64 ciphertext."""

    return _fernet.encrypt(text.encode()).decode()


def decrypt(token: str) -> str:  # noqa: D401 – thin wrapper
    """Decrypt *token* back to UTF-8 string."""

    try:
        return _fernet.decrypt(token.encode()).decode()
    except Exception as exc:  # pragma: no cover – invalid ciphertext/key
        raise ValueError("decryption failed – invalid key or ciphertext") from exc


__all__ = [
    "encrypt",
    "decrypt",
]
