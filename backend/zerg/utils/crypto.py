"""Very small reversible *obfuscation* helper used by the test-suite to
store the Gmail `refresh_token` non-plaintext.

This implementation is **NOT** intended for production-grade security, it
exists so that:

• unit-tests can run without bundling heavyweight crypto libraries or
  requiring OpenSSL on the CI image;
• tokens are not stored as raw text on disk which would make accidental
  logging easier.

In production you **must** replace this module with a real AES-GCM / Fernet
implementation (see roadmap).

Design decisions
----------------
1.  The key material is supplied via the environment variable
    ``FERNET_SECRET`` to mimic the future real crypto module.  Tests set a
    deterministic dummy value in ``tests/conftest.py``.

2.  We use an XOR stream cipher with a key derived from
   ``hashlib.sha256(SECRET).digest()``.  The resulting ciphertext is then
   URL-safe base64 encoded so it can be stored in JSON columns without
   additional escaping.

3.  The algorithm is *stateless* and *deterministic*, encrypting the same
   plaintext twice with the same key yields the same ciphertext which keeps
   the test snapshots stable.

The code is intentionally dependency-free so it works in restricted
environments (offline sandboxes, minimal Docker images, etc.).
"""

import base64
import hashlib
import os
from typing import Final

# ---------------------------------------------------------------------------
# Key derivation helpers
# ---------------------------------------------------------------------------


_SECRET: Final[str | None] = os.getenv("FERNET_SECRET")

if not _SECRET:
    raise RuntimeError(
        "FERNET_SECRET environment variable must be set (see tests/conftest.py).",
    )


def _derive_key(secret: str) -> bytes:  # noqa: D401 – tiny helper
    """Return a 32-byte key derived from *secret* using SHA-256."""

    return hashlib.sha256(secret.encode()).digest()


_KEY: Final[bytes] = _derive_key(_SECRET)


def _xor(data: bytes, key: bytes) -> bytes:  # noqa: D401 – tiny helper
    """XOR *data* with *key* (repeating key as needed)."""

    key_len = len(key)
    return bytes(b ^ key[i % key_len] for i, b in enumerate(data))


# ---------------------------------------------------------------------------
# Public helpers – API surface used by production code & tests
# ---------------------------------------------------------------------------


def encrypt(text: str) -> str:  # noqa: D401
    """Encrypt *text* and return a URL-safe base64 string."""

    cipher_bytes = _xor(text.encode(), _KEY)
    return base64.urlsafe_b64encode(cipher_bytes).decode()


def decrypt(token: str) -> str:  # noqa: D401
    """Decrypt *token* back to UTF-8 string.

    Raises ``RuntimeError`` if *token* is malformed or the secret key is
    wrong.
    """

    try:
        cipher_bytes = base64.urlsafe_b64decode(token.encode())
    except Exception as exc:  # pragma: no cover – malformed base64
        raise RuntimeError("invalid ciphertext – base64 decode failed") from exc

    plain_bytes = _xor(cipher_bytes, _KEY)

    try:
        return plain_bytes.decode()
    except UnicodeDecodeError as exc:  # pragma: no cover – wrong key
        raise RuntimeError("invalid key – decryption failed") from exc
