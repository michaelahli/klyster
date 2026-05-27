"""Threshold-based rule forecaster (no ML, fast path)."""

from __future__ import annotations

from typing import Any, Dict, List, Sequence, Tuple

import numpy as np

from seer.functions.linear_regression import _infer_cadence, _interpolate_missing
from seer.functions.types import ForecastPoint

DEFAULT_LOOKBACK = 10


def threshold_rules_forecast(
    data: Sequence[Tuple[int, float]],
    horizon: int,
    params: Dict[str, Any] | None = None,
) -> List[ForecastPoint]:
    """Predict the recent average and bracket it with the configured thresholds.

    Useful as a deterministic baseline or for capacity rules that should not
    be governed by an ML model.

    Args:
        data: Historical observations as (unix_seconds, value) pairs.
        horizon: Number of future points. Must be > 0.
        params: Required dict. Recognised keys:
            upper_threshold: float, scaling-up cue.
            lower_threshold: float, scaling-down cue. Must be < upper_threshold.
            lookback_window: int, defaults to ``DEFAULT_LOOKBACK``.

    Returns:
        A list of `ForecastPoint`s of length ``horizon`` whose predicted
        value is the rolling average over the lookback window. The lower /
        upper bounds carry the supplied thresholds so downstream consumers
        can flag scaling decisions directly.

    Raises:
        ValueError: if input is empty, thresholds are missing or
            inconsistent, or the lookback is invalid.
    """
    if horizon <= 0:
        raise ValueError("horizon must be positive")
    if not data:
        raise ValueError("data must not be empty")

    params = params or {}
    if "upper_threshold" not in params or "lower_threshold" not in params:
        raise ValueError("'upper_threshold' and 'lower_threshold' are required")

    upper = float(params["upper_threshold"])
    lower = float(params["lower_threshold"])
    if not lower < upper:
        raise ValueError("'lower_threshold' must be strictly less than 'upper_threshold'")

    lookback = int(params.get("lookback_window", DEFAULT_LOOKBACK))
    if lookback < 1:
        raise ValueError("'lookback_window' must be >= 1")

    sorted_data = sorted(data, key=lambda pt: pt[0])
    timestamps = np.array([t for t, _ in sorted_data], dtype=np.float64)
    values = np.array([v for _, v in sorted_data], dtype=np.float64)
    if not np.all(np.isfinite(values)):
        values = _interpolate_missing(values)

    window = min(lookback, len(values))
    average = float(np.mean(values[-window:]))

    cadence = _infer_cadence(timestamps)
    last_ts = timestamps[-1]
    return [
        ForecastPoint(
            timestamp=int(last_ts + cadence * (i + 1)),
            predicted_value=average,
            lower_bound=lower,
            upper_bound=upper,
        )
        for i in range(horizon)
    ]


__all__ = ["threshold_rules_forecast", "DEFAULT_LOOKBACK"]
