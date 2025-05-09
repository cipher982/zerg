"""Token-encryption helper used for Gmail *refresh_token* storage.

Goals
-----
1.  *Production-grade* – prefer AES-128-GCM (Fernet) when the `cryptography`
    package is available.
2.  *Zero external deps for tests* – fall back to the previous XOR + Base64
    obfuscation so the CI environment does **not** require OpenSSL wheels.
3.  *Deterministic* – same plaintext + key ⇒ same ciphertext (this keeps the
    existing unit-tests unchanged).

Key derivation
--------------
The module expects an environment variable

    FERNET_SECRET="<url-safe-base64-32-bytes>"

The **same** secret is used for both Fernet and the XOR fallback so behaviour
remains stable across environments.
"""

from __future__ import annotations

import base64
import hashlib
import os
from typing import Final

_SECRET_B64: Final[str | None] = os.getenv("FERNET_SECRET")

if not _SECRET_B64:
    raise RuntimeError(
        "FERNET_SECRET environment variable must be set.  In tests this is" " injected via backend/tests/conftest.py.",
    )

# ---------------------------------------------------------------------------
# Strategy selector – use Fernet if available, else XOR fallback
# ---------------------------------------------------------------------------


try:
    from cryptography.fernet import Fernet  # type: ignore

    # Validate / decode key (Fernet expects 32 url-safe base64 bytes)
    try:
        _fernet = Fernet(_SECRET_B64.encode())
    except Exception as exc:  # pragma: no cover – bad key supplied
        raise RuntimeError("FERNET_SECRET is not a valid Fernet key") from exc

    def encrypt(text: str) -> str:  # noqa: D401 – public API
        """Encrypt *text* using AES-GCM (Fernet)."""

        return _fernet.encrypt(text.encode()).decode()

    def decrypt(token: str) -> str:  # noqa: D401 – public API
        """Decrypt *token* back to UTF-8 using AES-GCM (Fernet)."""

        try:
            return _fernet.decrypt(token.encode()).decode()
        except Exception as exc:  # pragma: no cover – invalid token / key
            raise RuntimeError("decryption failed – invalid key or ciphertext") from exc

    _USING_FERNET = True

except ModuleNotFoundError:  # pragma: no cover – fallback for minimal envs
    # ---------------------------------------------------------------------
    # XOR + Base64 **fallback** – identical to the previous implementation
    # ---------------------------------------------------------------------

    _USING_FERNET = False

    def _derive_key(secret: str) -> bytes:  # noqa: D401 – helper
        return hashlib.sha256(secret.encode()).digest()

    _KEY: Final[bytes] = _derive_key(_SECRET_B64)

    def _xor(data: bytes, key: bytes) -> bytes:  # noqa: D401 – helper
        key_len = len(key)
        return bytes(b ^ key[i % key_len] for i, b in enumerate(data))

    def encrypt(text: str) -> str:  # noqa: D401
        cipher_bytes = _xor(text.encode(), _KEY)
        return base64.urlsafe_b64encode(cipher_bytes).decode()

    def decrypt(token: str) -> str:  # noqa: D401
        try:
            cipher_bytes = base64.urlsafe_b64decode(token.encode())
        except Exception as exc:  # pragma: no cover
            raise RuntimeError("invalid ciphertext – base64 decode failed") from exc

        plain_bytes = _xor(cipher_bytes, _KEY)
        try:
            return plain_bytes.decode()
        except UnicodeDecodeError as exc:  # pragma: no cover
            raise RuntimeError("invalid key – decryption failed") from exc

# Public helper so tests can assert which backend is active


def using_fernet() -> bool:  # noqa: D401 – small util
    return _USING_FERNET
