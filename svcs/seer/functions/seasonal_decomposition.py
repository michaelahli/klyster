"""Seasonal decomposition + trend extrapolation forecaster."""

from __future__ import annotations

import math
from typing import Any, Dict, List, Sequence, Tuple

import numpy as np
from statsmodels.tsa.seasonal import seasonal_decompose

from seer.functions.linear_regression import (
    DEFAULT_CONFIDENCE,
    _infer_cadence,
    _interpolate_missing,
    _z_score_for,
)
from seer.functions.types import ForecastPoint

DEFAULT_MODEL = "additive"
SUPPORTED_MODELS = ("additive", "multiplicative")
MIN_PERIOD = 2


def seasonal_decomposition_forecast(
    data: Sequence[Tuple[int, float]],
    horizon: int,
    params: Dict[str, Any] | None = None,
) -> List[ForecastPoint]:
    """Forecast by decomposing the series into trend + seasonal + residual.

    Args:
        data: Historical observations as (unix_seconds, value) pairs.
        horizon: Number of future points to predict. Must be > 0.
        params: Optional dict. Recognised keys:
            period: Required positive integer >= 2.
            model: "additive" (default) or "multiplicative".
            confidence_interval: float in (0, 1), defaults to 0.95.

    Returns:
        A list of `ForecastPoint`s of length `horizon`.

    Raises:
        ValueError: if input is too small for the requested period, the
            period or model is invalid, or the confidence interval is out
            of range.
    """
    if horizon <= 0:
        raise ValueError("horizon must be positive")

    params = params or {}
    period = params.get("period")
    if period is None:
        raise ValueError("'period' parameter is required")
    try:
        period = int(period)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"'period' must be an integer: {exc}") from exc
    if period < MIN_PERIOD:
        raise ValueError(f"'period' must be >= {MIN_PERIOD}")

    model = str(params.get("model", DEFAULT_MODEL)).lower()
    if model not in SUPPORTED_MODELS:
        raise ValueError(
            f"'model' must be one of {SUPPORTED_MODELS}, got '{model}'"
        )

    confidence = float(params.get("confidence_interval", DEFAULT_CONFIDENCE))
    if not 0.0 < confidence < 1.0:
        raise ValueError("confidence_interval must be in (0, 1)")
    z_score = _z_score_for(confidence)

    if len(data) < 2 * period:
        raise ValueError(
            f"seasonal decomposition needs at least 2*period points "
            f"(>= {2 * period}), got {len(data)}"
        )

    sorted_data = sorted(data, key=lambda pt: pt[0])
    timestamps = np.array([t for t, _ in sorted_data], dtype=np.float64)
    values = np.array([v for _, v in sorted_data], dtype=np.float64)
    if not np.all(np.isfinite(values)):
        values = _interpolate_missing(values)

    if model == "multiplicative" and np.any(values <= 0):
        raise ValueError("multiplicative model requires strictly positive values")

    decomposition = seasonal_decompose(
        values,
        period=period,
        model=model,
        extrapolate_trend="freq",
    )
    trend = np.asarray(decomposition.trend, dtype=np.float64)
    seasonal = np.asarray(decomposition.seasonal, dtype=np.float64)
    residual = np.asarray(decomposition.resid, dtype=np.float64)

    trend = _interpolate_missing(trend) if not np.all(np.isfinite(trend)) else trend

    # Extrapolate trend by fitting a line to the full trend component.
    indices = np.arange(len(values), dtype=np.float64)
    slope, intercept = np.polyfit(indices, trend, 1)

    # Use the seasonal pattern from the last full period; this is a stable
    # representation for both additive and multiplicative models.
    seasonal_cycle = seasonal[-period:]

    finite_residuals = residual[np.isfinite(residual)]
    residual_std = float(np.std(finite_residuals, ddof=1)) if finite_residuals.size > 1 else 0.0
    margin = z_score * residual_std

    cadence = _infer_cadence(timestamps)
    last_ts = timestamps[-1]
    last_index = len(values) - 1

    forecast: List[ForecastPoint] = []
    for i in range(horizon):
        future_index = last_index + i + 1
        future_ts = last_ts + cadence * (i + 1)
        trend_value = slope * future_index + intercept
        seasonal_value = seasonal_cycle[(future_index) % period]

        if model == "additive":
            predicted = trend_value + seasonal_value
            lower = predicted - margin
            upper = predicted + margin
        else:
            predicted = trend_value * seasonal_value
            # For multiplicative models, scale the margin by the seasonal
            # multiplier so wider variance maps to a wider band.
            scaled_margin = margin * abs(seasonal_value)
            lower = predicted - scaled_margin
            upper = predicted + scaled_margin

        forecast.append(
            ForecastPoint(
                timestamp=int(future_ts),
                predicted_value=float(predicted),
                lower_bound=float(lower),
                upper_bound=float(upper),
            )
        )
    return forecast


__all__ = ["seasonal_decomposition_forecast", "DEFAULT_MODEL", "SUPPORTED_MODELS"]
