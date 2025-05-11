"""Opinionated *structured log* helper – **structlog required**.

Because we fully own the runtime environments (dev, CI, prod), we commit to
`structlog` as the single logging backend instead of juggling multi-path
fallbacks.  If the import fails the application exits immediately with a
clear error so engineers notice the missing dependency during startup rather
than encountering half-configured log output later.
"""

from __future__ import annotations

import logging
import sys

try:
    import structlog  # type: ignore
except ModuleNotFoundError as exc:  # pragma: no cover – mandatory dep missing
    raise SystemExit(
        "structlog must be installed – add it to your environment via\n"
        "  uv pip install structlog[dev]\n"
        "or ensure the project dependency list is up-to-date."
    ) from exc


# ---------------------------------------------------------------------------
# structlog configuration (only once during import of this module)
# ---------------------------------------------------------------------------


def _configure_structlog() -> None:  # noqa: D401 – private helper
    """Initialise structlog with sensible JSON renderer."""

    if structlog.is_configured():  # type: ignore[attr-defined]
        return

    structlog.configure(
        processors=[
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.add_log_level,
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
            structlog.processors.UnicodeDecoder(),
            structlog.processors.JSONRenderer(),
        ],
        wrapper_class=structlog.make_filtering_bound_logger(logging.INFO),
        logger_factory=structlog.PrintLoggerFactory(file=sys.stdout),
    )


_configure_structlog()


def get_logger(name: str):  # noqa: D401 – public API
    """Return a :pyclass:`structlog.BoundLogger` bound to *name*."""

    return structlog.get_logger(name)


# Convenience global for quick imports (``from zerg.utils.log import log``)
log = get_logger("zerg")


__all__ = [
    "get_logger",
    "log",
]
