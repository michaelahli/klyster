"""Tests for the seasonal decomposition forecaster."""

from __future__ import annotations

import math

import numpy as np
import pytest

from seer.functions.seasonal_decomposition import seasonal_decomposition_forecast


def _seasonal_series(periods: int, period: int, slope: float = 0.0, base: float = 10.0):
    """Build (timestamp, value) pairs with a clear seasonal pattern."""
    pattern = [math.sin(2 * math.pi * i / period) for i in range(period)]
    points = []
    for cycle in range(periods):
        for i in range(period):
            t = cycle * period + i
            value = base + slope * t + pattern[i]
            points.append((t, value))
    return points


def test_forecast_repeats_known_seasonal_pattern() -> None:
    period = 4
    data = _seasonal_series(periods=6, period=period)

    forecast = seasonal_decomposition_forecast(data, horizon=period, params={"period": period})

    assert len(forecast) == period
    last_cycle = data[-period:]
    last_values = [v for _, v in last_cycle]
    forecasted = [pt.predicted_value for pt in forecast]
    # Forecast for the next cycle should track the magnitude of the previous one.
    assert max(forecasted) == pytest.approx(max(last_values), rel=0.2)
    assert min(forecasted) == pytest.approx(min(last_values), rel=0.2)


def test_forecast_extrapolates_trend_through_seasonality() -> None:
    period = 4
    data = _seasonal_series(periods=8, period=period, slope=0.5)

    forecast = seasonal_decomposition_forecast(data, horizon=period, params={"period": period})

    # Average of the next cycle should beat the average of the last observed cycle
    # because the underlying trend is positive.
    last_avg = np.mean([v for _, v in data[-period:]])
    next_avg = np.mean([pt.predicted_value for pt in forecast])
    assert next_avg > last_avg


def test_multiplicative_model_requires_positive_values() -> None:
    data = _seasonal_series(periods=4, period=4, base=0.5)
    data[0] = (data[0][0], -1.0)

    with pytest.raises(ValueError):
        seasonal_decomposition_forecast(
            data, horizon=2, params={"period": 4, "model": "multiplicative"}
        )


def test_rejects_when_data_too_short() -> None:
    period = 6
    data = _seasonal_series(periods=1, period=period)
    with pytest.raises(ValueError):
        seasonal_decomposition_forecast(data, horizon=2, params={"period": period})


def test_rejects_invalid_period() -> None:
    data = _seasonal_series(periods=4, period=4)
    with pytest.raises(ValueError):
        seasonal_decomposition_forecast(data, horizon=2, params={"period": 1})


def test_rejects_unknown_model() -> None:
    data = _seasonal_series(periods=4, period=4)
    with pytest.raises(ValueError):
        seasonal_decomposition_forecast(
            data, horizon=2, params={"period": 4, "model": "exponential"}
        )


def test_rejects_missing_period() -> None:
    data = _seasonal_series(periods=4, period=4)
    with pytest.raises(ValueError):
        seasonal_decomposition_forecast(data, horizon=2)


def test_confidence_bounds_widen_with_higher_confidence() -> None:
    period = 4
    rng = np.random.default_rng(seed=42)
    base = _seasonal_series(periods=10, period=period)
    noisy = [(t, v + float(rng.normal(0, 0.5))) for t, v in base]

    narrow = seasonal_decomposition_forecast(
        noisy, horizon=1, params={"period": period, "confidence_interval": 0.80}
    )
    wide = seasonal_decomposition_forecast(
        noisy, horizon=1, params={"period": period, "confidence_interval": 0.99}
    )

    narrow_width = narrow[0].upper_bound - narrow[0].lower_bound
    wide_width = wide[0].upper_bound - wide[0].lower_bound
    assert wide_width > narrow_width


def test_forecast_uses_inferred_cadence() -> None:
    period = 4
    raw = _seasonal_series(periods=4, period=period)
    data = [(t * 60, v) for t, v in raw]

    forecast = seasonal_decomposition_forecast(data, horizon=3, params={"period": period})

    last_input_ts = data[-1][0]
    expected = [last_input_ts + 60 * (i + 1) for i in range(3)]
    assert [pt.timestamp for pt in forecast] == expected
