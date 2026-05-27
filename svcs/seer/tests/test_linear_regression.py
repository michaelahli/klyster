"""Tests for the linear regression forecaster."""

from __future__ import annotations

import math

import pytest

from seer.functions.linear_regression import linear_regression_forecast


def test_forecast_extrapolates_constant_slope() -> None:
    data = [(i, 2.0 * i + 1.0) for i in range(20)]

    forecast = linear_regression_forecast(data, horizon=5)

    assert len(forecast) == 5
    expected = [2.0 * (20 + i) + 1.0 for i in range(5)]
    for point, target in zip(forecast, expected):
        assert math.isclose(point.predicted_value, target, rel_tol=1e-6)
        assert point.lower_bound <= point.predicted_value <= point.upper_bound


def test_confidence_bounds_widen_with_higher_confidence() -> None:
    rng = [(i, i + (-1) ** i * 0.5) for i in range(40)]

    narrow = linear_regression_forecast(rng, horizon=1, params={"confidence_interval": 0.80})
    wide = linear_regression_forecast(rng, horizon=1, params={"confidence_interval": 0.99})

    narrow_width = narrow[0].upper_bound - narrow[0].lower_bound
    wide_width = wide[0].upper_bound - wide[0].lower_bound
    assert wide_width > narrow_width


def test_forecast_uses_inferred_cadence() -> None:
    data = [(i * 60, float(i)) for i in range(10)]

    forecast = linear_regression_forecast(data, horizon=3)

    last_input_ts = data[-1][0]
    expected_timestamps = [last_input_ts + 60 * (i + 1) for i in range(3)]
    assert [pt.timestamp for pt in forecast] == expected_timestamps


def test_handles_missing_values_via_interpolation() -> None:
    data = [
        (0, 0.0),
        (1, float("nan")),
        (2, 4.0),
        (3, 6.0),
        (4, float("nan")),
        (5, 10.0),
    ]

    forecast = linear_regression_forecast(data, horizon=2)

    assert len(forecast) == 2
    for point in forecast:
        assert math.isfinite(point.predicted_value)
        assert math.isfinite(point.lower_bound)
        assert math.isfinite(point.upper_bound)


def test_unsorted_input_is_handled() -> None:
    data = [(i, float(i)) for i in range(10)]
    shuffled = list(reversed(data))

    forecast_sorted = linear_regression_forecast(data, horizon=3)
    forecast_shuffled = linear_regression_forecast(shuffled, horizon=3)

    for a, b in zip(forecast_sorted, forecast_shuffled):
        assert a.timestamp == b.timestamp
        assert math.isclose(a.predicted_value, b.predicted_value, rel_tol=1e-9)


def test_rejects_negative_horizon() -> None:
    with pytest.raises(ValueError):
        linear_regression_forecast([(0, 1.0), (1, 2.0)], horizon=0)


def test_rejects_too_few_points() -> None:
    with pytest.raises(ValueError):
        linear_regression_forecast([(0, 1.0)], horizon=1)


def test_rejects_invalid_confidence() -> None:
    data = [(i, float(i)) for i in range(5)]
    with pytest.raises(ValueError):
        linear_regression_forecast(data, horizon=1, params={"confidence_interval": 1.5})
