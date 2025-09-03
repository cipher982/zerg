"""Pricing catalog for LLM token costs (per 1K tokens).

Only models explicitly listed here have cost computed. Unknown models result
in a `None` cost and a structured log entry – no estimation or fallback.
"""

from __future__ import annotations

from typing import Optional
from typing import Tuple

# USD per 1K tokens (in, out). Populate conservatively and review via PRs.
# Empty by default to avoid implicit assumptions.
MODEL_PRICES_USD_PER_1K: dict[str, Tuple[float, float]] = {
    # "gpt-4o-mini": (0.0, 0.0),  # example placeholder – fill with audited values only
    # "gpt-4o": (0.0, 0.0),
    # "gpt-3.5-turbo": (0.0, 0.0),
    # Mock model has no cost
    "gpt-mock": (0.0, 0.0),
}


def get_usd_prices_per_1k(model_id: str) -> Optional[Tuple[float, float]]:
    return MODEL_PRICES_USD_PER_1K.get(model_id)
