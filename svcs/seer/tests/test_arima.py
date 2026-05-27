"""Tests for the ARIMA forecaster."""

from __future__ import annotations

import math

import numpy as np
import pytest

from seer.functions.arima import arima_forecast


def _trending_series(n: int, slope: float = 0.5, base: float = 10.0, noise: float = 0.0):
    rng = np.random.default_rng(seed=7)
    return [
        (i, base + slope * i + (rng.normal(0, noise) if noise > 0 else 0.0))
        for i in range(n)
    ]


def test_forecast_follows_trend_with_explicit_order() -> None:
    data = _trending_series(40)

    forecast = arima_forecast(data, horizon=5, params={"order": [1, 1, 0]})

    assert len(forecast) == 5
    last_value = data[-1][1]
    # Predicted values should keep moving up under a positive trend.
    assert forecast[0].predicted_value > last_value - 1.0
    assert forecast[-1].predicted_value > forecast[0].predicted_value - 1.0


def test_auto_order_runs_without_explicit_params() -> None:
    data = _trending_series(50, noise=0.2)

    forecast = arima_forecast(data, horizon=3)

    assert len(forecast) == 3
    for point in forecast:
        assert math.isfinite(point.predicted_value)
        assert point.lower_bound <= point.predicted_value <= point.upper_bound


def test_returns_confidence_interval_widening_with_horizon() -> None:
    data = _trending_series(60, noise=0.5)

    forecast = arima_forecast(data, horizon=10, params={"order": [1, 1, 1]})

    widths = [pt.upper_bound - pt.lower_bound for pt in forecast]
    # Generally the interval should widen further out, allow non-strict on
    # short horizons where it can plateau.
    assert widths[-1] >= widths[0]


def test_uses_inferred_cadence_for_future_timestamps() -> None:
    raw = _trending_series(40)
    data = [(t * 60, v) for t, v in raw]

    forecast = arima_forecast(data, horizon=3, params={"order": [0, 1, 0]})

    last_ts = data[-1][0]
    expected = [last_ts + 60 * (i + 1) for i in range(3)]
    assert [pt.timestamp for pt in forecast] == expected


def test_rejects_too_few_points() -> None:
    with pytest.raises(ValueError):
        arima_forecast([(i, float(i)) for i in range(3)], horizon=2)


def test_rejects_invalid_horizon() -> None:
    data = _trending_series(20)
    with pytest.raises(ValueError):
        arima_forecast(data, horizon=0)


def test_rejects_malformed_order() -> None:
    data = _trending_series(20)
    with pytest.raises(ValueError):
        arima_forecast(data, horizon=2, params={"order": [1, 1]})


def test_rejects_non_integer_order() -> None:
    data = _trending_series(20)
    with pytest.raises(ValueError):
        arima_forecast(data, horizon=2, params={"order": [1.5, 1, 0]})


def test_rejects_invalid_confidence() -> None:
    data = _trending_series(20)
    with pytest.raises(ValueError):
        arima_forecast(data, horizon=2, params={"order": [1, 1, 0], "confidence_interval": 1.2})
