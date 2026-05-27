"""Tests for the threshold-based rule forecaster."""

from __future__ import annotations

import math

import pytest

from seer.functions.threshold_rules import threshold_rules_forecast


def test_default_lookback_passes_through_recent_average() -> None:
    data = [(i, 50.0) for i in range(10)]

    forecast = threshold_rules_forecast(
        data,
        horizon=3,
        params={"upper_threshold": 80.0, "lower_threshold": 20.0},
    )

    assert len(forecast) == 3
    for point in forecast:
        assert math.isclose(point.predicted_value, 50.0)
        assert point.lower_bound == 20.0
        assert point.upper_bound == 80.0


def test_custom_lookback_window() -> None:
    data = [(i, float(i)) for i in range(20)]

    forecast = threshold_rules_forecast(
        data,
        horizon=2,
        params={
            "upper_threshold": 100.0,
            "lower_threshold": 0.0,
            "lookback_window": 5,
        },
    )

    # Average of the last 5 points: (15+16+17+18+19) / 5 = 17.0
    assert all(math.isclose(pt.predicted_value, 17.0) for pt in forecast)


def test_uses_inferred_cadence() -> None:
    data = [(i * 60, 1.0) for i in range(8)]

    forecast = threshold_rules_forecast(
        data,
        horizon=3,
        params={"upper_threshold": 5.0, "lower_threshold": 0.0},
    )

    last_ts = data[-1][0]
    expected = [last_ts + 60 * (i + 1) for i in range(3)]
    assert [pt.timestamp for pt in forecast] == expected


def test_rejects_non_positive_horizon() -> None:
    with pytest.raises(ValueError):
        threshold_rules_forecast(
            [(0, 1.0), (1, 2.0)],
            horizon=0,
            params={"upper_threshold": 10.0, "lower_threshold": 0.0},
        )


def test_rejects_thresholds_in_wrong_order() -> None:
    with pytest.raises(ValueError):
        threshold_rules_forecast(
            [(0, 1.0), (1, 2.0)],
            horizon=1,
            params={"upper_threshold": 5.0, "lower_threshold": 10.0},
        )


def test_rejects_missing_thresholds() -> None:
    with pytest.raises(ValueError):
        threshold_rules_forecast([(0, 1.0)], horizon=1, params={})


def test_rejects_invalid_lookback() -> None:
    data = [(i, float(i)) for i in range(5)]
    with pytest.raises(ValueError):
        threshold_rules_forecast(
            data,
            horizon=1,
            params={
                "upper_threshold": 10.0,
                "lower_threshold": 0.0,
                "lookback_window": 0,
            },
        )


def test_rejects_empty_data() -> None:
    with pytest.raises(ValueError):
        threshold_rules_forecast(
            [],
            horizon=1,
            params={"upper_threshold": 10.0, "lower_threshold": 0.0},
        )
