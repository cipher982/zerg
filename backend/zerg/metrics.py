"""Prometheus metrics for trigger subsystem and Gmail integration.

The module bundles all counters in one place so importing side-effects
(metric registration) happen exactly once per process.  Routers and
services can simply ``from zerg.metrics import …`` and increment.
"""

from __future__ import annotations

# ``prometheus_client`` is an optional dependency pulled in via
# ``backend/pyproject.toml``.  Import lazily so unit-tests that filter
# deps via *–no-deps* still succeed.


try:
    from prometheus_client import Counter  # type: ignore

    trigger_fired_total = Counter(
        "trigger_fired_total",
        "Total number of triggers that fired (all types)",
    )

    gmail_watch_renew_total = Counter(
        "gmail_watch_renew_total",
        "Total number of Gmail watch renewals performed",
    )

    gmail_api_error_total = Counter(
        "gmail_api_error_total",
        "Number of errors when interacting with the Gmail API",
    )

    external_api_retry_total = Counter(
        "external_api_retry_total",
        "Total retries executed against external providers",
        labelnames=("provider", "function"),
    )

except ModuleNotFoundError:  # pragma: no cover – metrics disabled when lib absent

    class _NoopCounter:  # noqa: D401 – tiny helper
        def inc(self, _value: int | float = 1):  # noqa: D401 – mimic prometheus
            return None

        def labels(self, *args, **kwargs):  # type: ignore
            return self

    trigger_fired_total = _NoopCounter()  # type: ignore[assignment]
    gmail_watch_renew_total = _NoopCounter()  # type: ignore[assignment]
    gmail_api_error_total = _NoopCounter()  # type: ignore[assignment]
    external_api_retry_total = _NoopCounter()  # type: ignore[assignment]
