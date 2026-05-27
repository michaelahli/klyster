"""Conversion between protobuf ``MetricDataPoint`` messages and pandas DataFrames.

The Seer service receives metric history as ``ForecastRequest.data`` (a repeated
``MetricDataPoint`` field). Forecasting code prefers a clean indexed pandas
DataFrame, so this module owns the conversion in both directions and centralises
handling of timezones, missing values, and label flattening.

Wire conventions (see ``proto/analytics.proto``):
    * ``timestamp``: int64, Unix epoch *seconds* (UTC).
    * ``value``: double; NaN/+Inf/-Inf are valid on the wire but get coerced to
      NaN in the DataFrame so callers can interpolate or drop uniformly.
    * ``labels``: ``map<string, string>`` of dimensional metadata.
"""

from __future__ import annotations

from typing import Iterable, List, Mapping, Sequence

import math

import numpy as np
import pandas as pd

from seer import analytics_pb2

__all__ = [
    "data_points_to_dataframe",
    "dataframe_to_data_points",
    "data_points_to_pairs",
]

VALUE_COLUMN = "value"
TIMESTAMP_COLUMN = "timestamp"
LABEL_PREFIX = "label_"


def data_points_to_dataframe(
    points: Sequence[analytics_pb2.MetricDataPoint],
    *,
    sort: bool = True,
    drop_non_finite: bool = False,
) -> pd.DataFrame:
    """Convert protobuf data points to a pandas DataFrame.

    The DataFrame uses a UTC ``DatetimeIndex`` derived from ``timestamp`` and
    contains a single ``value`` column plus one ``label_<key>`` column per
    distinct label key encountered (missing entries become empty strings).

    Args:
        points: Repeated ``MetricDataPoint`` field from the wire.
        sort: When ``True`` (default), the result is sorted ascending by
            timestamp. The wire format does not guarantee ordering.
        drop_non_finite: When ``True``, rows whose value is NaN/Inf are
            removed. Defaults to ``False`` so callers can interpolate.

    Returns:
        A DataFrame indexed by ``DatetimeIndex`` (UTC). Empty input yields an
        empty DataFrame with a properly typed index.
    """
    if not points:
        return pd.DataFrame(
            {VALUE_COLUMN: pd.Series(dtype="float64")},
            index=pd.DatetimeIndex([], tz="UTC", name=TIMESTAMP_COLUMN),
        )

    label_keys = _collect_label_keys(points)

    n = len(points)
    timestamps = np.empty(n, dtype=np.int64)
    values = np.empty(n, dtype=np.float64)
    for i, point in enumerate(points):
        timestamps[i] = point.timestamp
        values[i] = point.value

    # Mask infinities to NaN (gives callers a uniform missing-value sentinel).
    values[np.isinf(values)] = np.nan

    # ``pd.to_datetime(..., unit='s')`` may silently coerce out-of-range values
    # in newer pandas; validate the seconds range against ns-precision bounds
    # (about ±2.92e11 seconds, i.e. years 1677-2262).
    _MAX_SECS = 9_223_372_036  # 2**63 ns / 1e9, rounded down
    if timestamps.size and (
        timestamps.max() > _MAX_SECS or timestamps.min() < -_MAX_SECS
    ):
        raise ValueError(
            f"timestamp out of range: must be within ±{_MAX_SECS} seconds of epoch"
        )
    try:
        index = pd.to_datetime(timestamps, unit="s", utc=True)
    except (OverflowError, ValueError, pd.errors.OutOfBoundsDatetime) as exc:
        raise ValueError(f"timestamp out of range: {exc}") from exc
    index.name = TIMESTAMP_COLUMN

    data: dict[str, object] = {VALUE_COLUMN: values}
    if label_keys:
        # Build label columns in one pass each, defaulting to empty strings.
        for key in label_keys:
            column = [point.labels.get(key, "") for point in points]
            data[f"{LABEL_PREFIX}{key}"] = column

    df = pd.DataFrame(data, index=index)

    if sort:
        df = df.sort_index(kind="mergesort")
    if drop_non_finite:
        df = df[np.isfinite(df[VALUE_COLUMN])]

    return df


def dataframe_to_data_points(
    df: pd.DataFrame,
    *,
    value_column: str = VALUE_COLUMN,
) -> List[analytics_pb2.MetricDataPoint]:
    """Convert a DataFrame back to a list of ``MetricDataPoint`` messages.

    The DataFrame must be indexed by a ``DatetimeIndex`` (timezone-aware or
    naive; naive indexes are interpreted as UTC). Any column whose name starts
    with ``label_`` is encoded into the ``labels`` map (with the prefix
    stripped). Non-finite values are dropped because they cannot round-trip
    through the wire format unambiguously.
    """
    if df.empty:
        return []
    if not isinstance(df.index, pd.DatetimeIndex):
        raise TypeError("DataFrame must be indexed by a DatetimeIndex")
    if value_column not in df.columns:
        raise KeyError(f"missing value column '{value_column}'")

    index = df.index
    if index.tz is None:
        index = index.tz_localize("UTC")

    label_columns = [c for c in df.columns if c.startswith(LABEL_PREFIX)]

    points: List[analytics_pb2.MetricDataPoint] = []
    values = df[value_column].to_numpy(dtype=np.float64, copy=False)
    # ``DatetimeIndex`` resolution may be ns/us/ms/s in pandas 2.x; convert to
    # seconds explicitly. ``.astype`` does not allow stripping tz, so we drop
    # the timezone first (we already normalised to UTC above).
    seconds = index.tz_convert("UTC").tz_localize(None).astype("datetime64[s]").astype("int64")

    label_arrays = {col: df[col].astype(str).to_numpy(copy=False) for col in label_columns}

    for i, value in enumerate(values):
        if not math.isfinite(value):
            continue
        labels: dict[str, str] = {}
        for col, arr in label_arrays.items():
            label_value = arr[i]
            if label_value:
                labels[col[len(LABEL_PREFIX):]] = label_value
        points.append(
            analytics_pb2.MetricDataPoint(
                timestamp=int(seconds[i]),
                value=float(value),
                labels=labels,
            )
        )

    return points


def data_points_to_pairs(
    points: Iterable[analytics_pb2.MetricDataPoint],
    *,
    drop_non_finite: bool = False,
) -> List[tuple[int, float]]:
    """Convert protobuf data points to ``(unix_seconds, value)`` tuples.

    Used by predefined functions that take raw ``Sequence[Tuple[int, float]]``
    inputs (linear regression, ARIMA, …) and don't need a DataFrame.
    """
    pairs: List[tuple[int, float]] = []
    for point in points:
        value = _normalize_value(point.value)
        if drop_non_finite and not math.isfinite(value):
            continue
        pairs.append((int(point.timestamp), value))
    return pairs


def _collect_label_keys(
    points: Sequence[analytics_pb2.MetricDataPoint],
) -> List[str]:
    """Return the union of label keys across ``points``, sorted for stability."""
    seen: set[str] = set()
    for point in points:
        seen.update(point.labels.keys())
    return sorted(seen)


def _normalize_value(value: float) -> float:
    """Coerce ±Infinity to NaN so downstream code can treat all gaps uniformly."""
    if math.isinf(value):
        return float("nan")
    return float(value)
