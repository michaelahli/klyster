"""ARIMA forecasting (auto-order via pmdarima with statsmodels fallback)."""

from __future__ import annotations

from typing import Any, Dict, List, Sequence, Tuple

import numpy as np

from seer.functions.linear_regression import (
    DEFAULT_CONFIDENCE,
    _infer_cadence,
    _interpolate_missing,
)
from seer.functions.types import ForecastPoint

MIN_POINTS = 8


def arima_forecast(
    data: Sequence[Tuple[int, float]],
    horizon: int,
    params: Dict[str, Any] | None = None,
) -> List[ForecastPoint]:
    """Forecast using ARIMA (or auto-ARIMA if `order` is omitted).

    Args:
        data: Historical observations as (unix_seconds, value) pairs.
        horizon: Number of future points to predict. Must be > 0.
        params: Optional dict. Recognised keys:
            order: tuple/list `(p, d, q)`; auto-selected if absent.
            seasonal_order: tuple/list `(P, D, Q, m)`; optional.
            confidence_interval: float in (0, 1), defaults to 0.95.

    Returns:
        A list of `ForecastPoint`s of length `horizon`.

    Raises:
        ValueError: if input is too small or parameters are malformed.
    """
    if horizon <= 0:
        raise ValueError("horizon must be positive")
    if len(data) < MIN_POINTS:
        raise ValueError(
            f"ARIMA requires at least {MIN_POINTS} points, got {len(data)}"
        )

    params = params or {}
    confidence = float(params.get("confidence_interval", DEFAULT_CONFIDENCE))
    if not 0.0 < confidence < 1.0:
        raise ValueError("confidence_interval must be in (0, 1)")

    order = _coerce_order(params.get("order"), expected=3, name="order")
    seasonal_order = _coerce_order(
        params.get("seasonal_order"), expected=4, name="seasonal_order"
    )

    sorted_data = sorted(data, key=lambda pt: pt[0])
    timestamps = np.array([t for t, _ in sorted_data], dtype=np.float64)
    values = np.array([v for _, v in sorted_data], dtype=np.float64)
    if not np.all(np.isfinite(values)):
        values = _interpolate_missing(values)

    predicted, lower, upper = _run_arima(values, horizon, confidence, order, seasonal_order)

    cadence = _infer_cadence(timestamps)
    last_ts = timestamps[-1]
    return [
        ForecastPoint(
            timestamp=int(last_ts + cadence * (i + 1)),
            predicted_value=float(predicted[i]),
            lower_bound=float(lower[i]),
            upper_bound=float(upper[i]),
        )
        for i in range(horizon)
    ]


def _coerce_order(value: Any, expected: int, name: str) -> tuple[int, ...] | None:
    if value is None:
        return None
    if not isinstance(value, (list, tuple)):
        raise ValueError(f"'{name}' must be a list or tuple")
    if len(value) != expected:
        raise ValueError(f"'{name}' must have exactly {expected} elements")
    coerced: list[int] = []
    for v in value:
        if isinstance(v, bool) or not isinstance(v, int):
            raise ValueError(f"'{name}' values must be integers, got {type(v).__name__}")
        coerced.append(v)
    return tuple(coerced)


def _run_arima(
    values: np.ndarray,
    horizon: int,
    confidence: float,
    order: tuple[int, ...] | None,
    seasonal_order: tuple[int, ...] | None,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Fit ARIMA via pmdarima when available, otherwise fall back to statsmodels."""
    if order is None:
        try:
            return _auto_arima(values, horizon, confidence, seasonal_order)
        except Exception:  # pragma: no cover - environment-specific
            order = _heuristic_order(values)

    return _statsmodels_arima(values, horizon, confidence, order, seasonal_order)


def _auto_arima(
    values: np.ndarray,
    horizon: int,
    confidence: float,
    seasonal_order: tuple[int, ...] | None,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    import pmdarima as pm  # local import to keep import cost out of cold paths

    seasonal = seasonal_order is not None
    m = seasonal_order[3] if seasonal else 1
    model = pm.auto_arima(
        values,
        seasonal=seasonal,
        m=m,
        suppress_warnings=True,
        error_action="ignore",
        stepwise=True,
    )
    alpha = 1.0 - confidence
    forecast, conf_int = model.predict(n_periods=horizon, return_conf_int=True, alpha=alpha)
    forecast = np.asarray(forecast, dtype=np.float64)
    conf_int = np.asarray(conf_int, dtype=np.float64)
    return forecast, conf_int[:, 0], conf_int[:, 1]


def _statsmodels_arima(
    values: np.ndarray,
    horizon: int,
    confidence: float,
    order: tuple[int, ...],
    seasonal_order: tuple[int, ...] | None,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    from statsmodels.tsa.arima.model import ARIMA

    sm_seasonal = seasonal_order if seasonal_order else (0, 0, 0, 0)
    model = ARIMA(values, order=order, seasonal_order=sm_seasonal)
    fitted = model.fit()
    forecast_obj = fitted.get_forecast(steps=horizon)
    predicted = np.asarray(forecast_obj.predicted_mean, dtype=np.float64)
    conf_int = np.asarray(forecast_obj.conf_int(alpha=1.0 - confidence), dtype=np.float64)
    return predicted, conf_int[:, 0], conf_int[:, 1]


def _heuristic_order(values: np.ndarray) -> tuple[int, int, int]:
    """Cheap fallback: differentiate once if non-stationary, no AR/MA terms."""
    if values.size < 2:
        return (0, 0, 0)
    diffs = np.diff(values)
    if np.allclose(diffs, diffs[0], atol=1e-9):
        return (0, 0, 0)
    return (1, 1, 1)


__all__ = ["arima_forecast", "MIN_POINTS"]
