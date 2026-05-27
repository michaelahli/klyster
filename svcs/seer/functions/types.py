"""Shared types for forecasting functions."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Sequence, Tuple


@dataclass(frozen=True)
class ForecastPoint:
    """A single forecast prediction with confidence bounds."""

    timestamp: int
    predicted_value: float
    lower_bound: float
    upper_bound: float


# A forecasting function takes historical (timestamp, value) pairs, a horizon,
# and a parameter dict, and returns a list of forecast points.
ForecastFunction = Callable[
    [Sequence[Tuple[int, float]], int, Dict[str, Any]],
    List[ForecastPoint],
]


@dataclass(frozen=True)
class FunctionDescriptor:
    """Metadata + callable for a forecasting function."""

    name: str
    description: str
    parameters_schema: Dict[str, Any]
    callable: ForecastFunction
