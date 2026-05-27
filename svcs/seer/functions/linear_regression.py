"""Linear regression forecasting using scikit-learn."""

from __future__ import annotations

import math
from typing import Any, Dict, List, Sequence, Tuple

import numpy as np
from sklearn.linear_model import LinearRegression

from seer.functions.types import ForecastPoint

# Approximate two-sided z-scores for common confidence levels. Linear
# regression sample sizes here are typically large enough that the normal
# approximation is acceptable for capacity planning.
_Z_SCORES = {
    0.80: 1.2816,
    0.90: 1.6449,
    0.95: 1.9600,
    0.975: 2.2414,
    0.99: 2.5758,
}

DEFAULT_CONFIDENCE = 0.95
MIN_POINTS = 2


def linear_regression_forecast(
    data: Sequence[Tuple[int, float]],
    horizon: int,
    params: Dict[str, Any] | None = None,
) -> List[ForecastPoint]:
    """Forecast future values using ordinary least squares.

    Args:
        data: Historical observations as (unix_seconds, value) pairs. The
            sequence does not need to be sorted; missing timestamps are
            tolerated and the model uses raw timestamps as the predictor.
        horizon: Number of points to predict beyond the last observation.
            Must be > 0.
        params: Optional dict. Recognised keys:
            confidence_interval: float in (0, 1), defaults to 0.95.

    Returns:
        A list of `ForecastPoint`s with predicted values plus residual-std
        based confidence bounds. The returned list has length `horizon`.

    Raises:
        ValueError: if input is too small, horizon is non-positive, or
            confidence_interval is outside the supported range.
    """
    if horizon <= 0:
        raise ValueError("horizon must be positive")
    if len(data) < MIN_POINTS:
        raise ValueError(
            f"linear regression requires at least {MIN_POINTS} points, got {len(data)}"
        )

    params = params or {}
    confidence = float(params.get("confidence_interval", DEFAULT_CONFIDENCE))
    if not 0.0 < confidence < 1.0:
        raise ValueError("confidence_interval must be in (0, 1)")
    z_score = _z_score_for(confidence)

    # Sort by timestamp; this avoids surprising behaviour when callers feed
    # interleaved series and lets us derive a reasonable forecast cadence.
    sorted_data = sorted(data, key=lambda pt: pt[0])
    timestamps = np.array([t for t, _ in sorted_data], dtype=np.float64)
    values = np.array([v for _, v in sorted_data], dtype=np.float64)

    if not np.all(np.isfinite(values)):
        # Replace NaN/Inf with a linear interpolation across the series so a
        # handful of missing samples does not poison the fit.
        values = _interpolate_missing(values)

    model = LinearRegression()
    model.fit(timestamps.reshape(-1, 1), values)

    in_sample = model.predict(timestamps.reshape(-1, 1))
    residuals = values - in_sample
    # Use sample std with ddof=1 when we have enough points; fall back to 0
    # for the degenerate case (n == MIN_POINTS) to avoid divide-by-zero.
    if len(residuals) > MIN_POINTS:
        residual_std = float(np.std(residuals, ddof=1))
    else:
        residual_std = 0.0
    margin = z_score * residual_std

    cadence = _infer_cadence(timestamps)
    last_ts = timestamps[-1]
    future_ts = np.array(
        [last_ts + cadence * (i + 1) for i in range(horizon)],
        dtype=np.float64,
    )
    predictions = model.predict(future_ts.reshape(-1, 1))

    return [
        ForecastPoint(
            timestamp=int(ts),
            predicted_value=float(pred),
            lower_bound=float(pred - margin),
            upper_bound=float(pred + margin),
        )
        for ts, pred in zip(future_ts, predictions)
    ]


def _z_score_for(confidence: float) -> float:
    """Return the closest tabulated two-sided z-score for `confidence`."""
    closest = min(_Z_SCORES.keys(), key=lambda c: abs(c - confidence))
    return _Z_SCORES[closest]


def _interpolate_missing(values: np.ndarray) -> np.ndarray:
    """Fill NaN/Inf entries via linear interpolation between finite samples."""
    finite = np.isfinite(values)
    if finite.all():
        return values
    if not finite.any():
        raise ValueError("all data points are missing")

    indices = np.arange(len(values))
    interpolated = np.interp(indices, indices[finite], values[finite])
    return interpolated


def _infer_cadence(timestamps: np.ndarray) -> float:
    """Estimate the median sampling interval to project future timestamps."""
    if len(timestamps) < 2:
        return 1.0
    diffs = np.diff(timestamps)
    diffs = diffs[diffs > 0]
    if diffs.size == 0:
        return 1.0
    cadence = float(np.median(diffs))
    if not math.isfinite(cadence) or cadence <= 0:
        return 1.0
    return cadence


__all__ = ["linear_regression_forecast", "DEFAULT_CONFIDENCE", "MIN_POINTS"]
