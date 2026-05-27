"""Tests for :mod:`seer.serialization`."""

from __future__ import annotations

import math
import time

import pandas as pd
import pytest

from seer import analytics_pb2
from seer.serialization import (
    data_points_to_dataframe,
    data_points_to_pairs,
    dataframe_to_data_points,
)


def make_point(ts: int, value: float, **labels: str) -> analytics_pb2.MetricDataPoint:
    return analytics_pb2.MetricDataPoint(timestamp=ts, value=value, labels=labels)


def test_empty_input_returns_empty_dataframe() -> None:
    df = data_points_to_dataframe([])
    assert df.empty
    assert df.index.tz is not None
    assert df.index.tz.utcoffset(None).total_seconds() == 0
    assert "value" in df.columns


def test_dataframe_indexed_by_utc_and_sorted() -> None:
    points = [
        make_point(300, 3.0),
        make_point(100, 1.0),
        make_point(200, 2.0),
    ]
    df = data_points_to_dataframe(points)
    assert isinstance(df.index, pd.DatetimeIndex)
    assert df.index.tz is not None
    assert list(df["value"]) == [1.0, 2.0, 3.0]
    assert df.index[0] == pd.Timestamp("1970-01-01 00:01:40", tz="UTC")


def test_label_columns_use_prefix_and_fill_missing() -> None:
    points = [
        make_point(100, 1.0, pod="api-1", ns="default"),
        make_point(200, 2.0, pod="api-2"),
    ]
    df = data_points_to_dataframe(points)
    assert df["label_pod"].tolist() == ["api-1", "api-2"]
    assert df["label_ns"].tolist() == ["default", ""]


def test_non_finite_values_become_nan_or_drop() -> None:
    points = [
        make_point(100, 1.0),
        make_point(200, float("nan")),
        make_point(300, float("inf")),
    ]
    kept = data_points_to_dataframe(points)
    assert math.isnan(kept["value"].iloc[1])
    assert math.isnan(kept["value"].iloc[2])

    dropped = data_points_to_dataframe(points, drop_non_finite=True)
    assert len(dropped) == 1
    assert dropped["value"].iloc[0] == 1.0


def test_round_trip_dataframe_back_to_points() -> None:
    original = [
        make_point(100, 1.0, pod="a"),
        make_point(200, 2.0, pod="b"),
        make_point(300, 3.0, pod="a"),
    ]
    df = data_points_to_dataframe(original)
    restored = dataframe_to_data_points(df)
    assert len(restored) == 3
    assert [p.timestamp for p in restored] == [100, 200, 300]
    assert [p.value for p in restored] == [1.0, 2.0, 3.0]
    assert restored[0].labels["pod"] == "a"
    assert restored[2].labels["pod"] == "a"


def test_round_trip_drops_non_finite_values_on_outbound() -> None:
    original = [
        make_point(100, 1.0),
        make_point(200, float("nan")),
        make_point(300, 3.0),
    ]
    df = data_points_to_dataframe(original)
    restored = dataframe_to_data_points(df)
    assert [p.timestamp for p in restored] == [100, 300]


def test_dataframe_to_points_rejects_missing_index() -> None:
    df = pd.DataFrame({"value": [1.0, 2.0]})
    with pytest.raises(TypeError):
        dataframe_to_data_points(df)


def test_dataframe_to_points_localizes_naive_index() -> None:
    df = pd.DataFrame(
        {"value": [1.0, 2.0]},
        index=pd.DatetimeIndex(["2024-01-01", "2024-01-02"], name="timestamp"),
    )
    points = dataframe_to_data_points(df)
    assert len(points) == 2
    # 2024-01-01 00:00:00 UTC == 1704067200
    assert points[0].timestamp == 1704067200


def test_data_points_to_pairs_normalizes_inf() -> None:
    points = [
        make_point(100, 1.0),
        make_point(200, float("inf")),
        make_point(300, 3.0),
    ]
    pairs = data_points_to_pairs(points)
    assert pairs[0] == (100, 1.0)
    assert math.isnan(pairs[1][1])
    assert pairs[2] == (300, 3.0)


def test_data_points_to_pairs_drop_non_finite() -> None:
    points = [
        make_point(100, 1.0),
        make_point(200, float("nan")),
    ]
    assert data_points_to_pairs(points, drop_non_finite=True) == [(100, 1.0)]


def test_invalid_timestamp_raises() -> None:
    points = [make_point(2**62, 1.0)]
    with pytest.raises(ValueError):
        data_points_to_dataframe(points)


def test_serialization_throughput_under_budget() -> None:
    # Acceptance: 10k points serialise in < 50ms.
    points = [make_point(i, float(i)) for i in range(10_000)]

    started = time.perf_counter()
    df = data_points_to_dataframe(points, sort=False)
    assert len(df) == 10_000
    forward_ms = (time.perf_counter() - started) * 1000

    started = time.perf_counter()
    restored = dataframe_to_data_points(df)
    assert len(restored) == 10_000
    backward_ms = (time.perf_counter() - started) * 1000

    assert forward_ms < 50, f"forward conversion took {forward_ms:.1f} ms"
    assert backward_ms < 50, f"backward conversion took {backward_ms:.1f} ms"
