"""Minimal wrapper that prefers *structlog* but gracefully falls back to the
standard ``logging`` module when the dependency is not installed.

Why this wrapper?
-----------------
* We want structured JSON logs in production (``structlog``).
* Unit-tests and local dev environments might not have the extra package.
* The rest of the codebase can therefore *always* ``from zerg.utils.log import log``
  and use ``log.info("msg", key=value)`` – calls behave the same regardless
  of whether structlog is available.
"""

from __future__ import annotations

import logging
from typing import Any


def _make_fallback_logger() -> "logging.Logger":  # noqa: D401 – small helper
    """Return a std-lib logger configured for dev/tests."""

    logger = logging.getLogger("zerg")

    if not logger.handlers:
        # BasicConfig is a no-op if already configured – run only once.
        logging.basicConfig(level=logging.INFO, format="%(levelname)s - %(message)s")

    return logger


# Try structlog first
try:
    import structlog  # type: ignore

    # Keep the logger global so every import shares the same base instance.
    log = structlog.get_logger("zerg")  # type: ignore[invalid-name]

    # Attach default processor chain only if structlog has not been
    # configured by the application already.
    if not structlog.is_configured():
        structlog.configure(processors=[structlog.processors.JSONRenderer()])

    # Defensive: some minimal structlog builds may fall back to stdlib logger
    # which lacks ``bind``.  Wrap in adapter if needed so callers can rely on
    # the method.
    if not hasattr(log, "bind"):

        class _StructAdapter:  # noqa: D401 – simple shim
            def __init__(self, base):
                self._base = base

            def bind(self, **_kw):  # noqa: D401
                return self

            def __getattr__(self, name):  # proxy other attrs
                return getattr(self._base, name)

        log = _StructAdapter(log)  # type: ignore[assignment, invalid-name]

except ModuleNotFoundError:  # pragma: no cover – dev env without extra dep

    class _StdLoggerAdapter:
        """Thin adapter so code can call ``.bind()`` like with structlog."""

        def __init__(self, base: logging.Logger):
            self._base = base

        # structlog-compatible API ------------------------------------------------

        def bind(self, **_kw):  # noqa: D401 – API shim
            return self  # Ignore bindings for std logging

        # Delegate common log methods -------------------------------------------

        def _fmt(self, msg: str, kw) -> str:  # noqa: D401 – helper
            return f"{msg} {kw}" if kw else msg

        def debug(self, msg: str, *args, **kw):  # noqa: D401 – keep parity
            if args:
                msg = msg % args  # mimic printf style for legacy calls
            self._base.debug(self._fmt(msg, kw))

        def info(self, msg: str, *args, **kw):
            if args:
                msg = msg % args
            self._base.info(self._fmt(msg, kw))

        def warning(self, msg: str, *args, **kw):
            if args:
                msg = msg % args
            self._base.warning(self._fmt(msg, kw))

        def error(self, msg: str, *args, **kw):
            if args:
                msg = msg % args
            self._base.error(self._fmt(msg, kw))

        def exception(self, msg: str, *args, **kw):
            if args:
                msg = msg % args
            self._base.exception(self._fmt(msg, kw))

    log = _StdLoggerAdapter(_make_fallback_logger())  # type: ignore[invalid-name]


def get_logger(**bindings: Any):  # noqa: D401 – factory helper
    """Return a child/bound logger with optional key/value bindings."""

    try:
        # For structlog the global *log* is a BoundLogger; just call .bind()
        import structlog  # type: ignore

        return log.bind(**bindings)  # type: ignore[attr-defined]
    except ModuleNotFoundError:  # pragma: no cover – fallback path
        return log
