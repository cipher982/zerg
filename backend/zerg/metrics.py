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

    # ------------------------------------------------------------------
    # Histograms (latency) ---------------------------------------------
    # ------------------------------------------------------------------

    from prometheus_client import Histogram  # type: ignore  # noqa: WPS433

    gmail_http_latency_seconds = Histogram(
        "gmail_http_latency_seconds",
        "Latency of Gmail HTTP requests (seconds)",
        buckets=(0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0),
    )

    trigger_processing_seconds = Histogram(
        "trigger_processing_seconds",
        "End-to-end processing time of a single trigger (seconds)",
        buckets=(0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1, 2, 5),
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

    # Provide *noop* Histogram so code can call ``observe`` without importing
    # the optional dependency in minimal CI images.

    class _NoopHistogram:  # noqa: D401 – tiny helper
        def observe(self, _value: float):  # noqa: D401 – mimic prometheus
            return None

    gmail_http_latency_seconds = _NoopHistogram()  # type: ignore[assignment]
    trigger_processing_seconds = _NoopHistogram()  # type: ignore[assignment]
