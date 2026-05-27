"""Forecasting functions for the Seer analytics sidecar."""

from __future__ import annotations

from typing import Callable, Dict

from seer.functions.arima import arima_forecast
from seer.functions.linear_regression import linear_regression_forecast
from seer.functions.seasonal_decomposition import seasonal_decomposition_forecast
from seer.functions.threshold_rules import threshold_rules_forecast
from seer.functions.types import ForecastFunction, ForecastPoint, FunctionDescriptor

PREDEFINED: Dict[str, FunctionDescriptor] = {
    "linear_regression": FunctionDescriptor(
        name="linear_regression",
        description="Linear regression forecast with residual-based confidence intervals.",
        parameters_schema={
            "confidence_interval": {
                "type": "number",
                "minimum": 0.5,
                "maximum": 0.999,
                "default": 0.95,
                "description": "Two-sided confidence level for prediction bounds.",
            }
        },
        callable=linear_regression_forecast,
    ),
    "seasonal_decomposition": FunctionDescriptor(
        name="seasonal_decomposition",
        description=(
            "Seasonal decomposition with linear trend extrapolation. "
            "Uses statsmodels.tsa.seasonal.seasonal_decompose under the hood."
        ),
        parameters_schema={
            "period": {
                "type": "integer",
                "minimum": 2,
                "description": "Length of the seasonal period in samples (required).",
            },
            "model": {
                "type": "string",
                "enum": ["additive", "multiplicative"],
                "default": "additive",
            },
            "confidence_interval": {
                "type": "number",
                "minimum": 0.5,
                "maximum": 0.999,
                "default": 0.95,
            },
        },
        callable=seasonal_decomposition_forecast,
    ),
    "arima": FunctionDescriptor(
        name="arima",
        description=(
            "ARIMA forecast. Order auto-selected via pmdarima when omitted, "
            "otherwise pinned to the supplied (p,d,q)."
        ),
        parameters_schema={
            "order": {
                "type": "array",
                "items": {"type": "integer", "minimum": 0},
                "minItems": 3,
                "maxItems": 3,
                "description": "Optional (p, d, q) order; auto-selected if omitted.",
            },
            "seasonal_order": {
                "type": "array",
                "items": {"type": "integer", "minimum": 0},
                "minItems": 4,
                "maxItems": 4,
                "description": "Optional (P, D, Q, m) seasonal order.",
            },
            "confidence_interval": {
                "type": "number",
                "minimum": 0.5,
                "maximum": 0.999,
                "default": 0.95,
            },
        },
        callable=arima_forecast,
    ),
    "threshold_rules": FunctionDescriptor(
        name="threshold_rules",
        description=(
            "Deterministic baseline: predicts the rolling average over the "
            "lookback window and exposes user-supplied scaling thresholds."
        ),
        parameters_schema={
            "upper_threshold": {
                "type": "number",
                "description": "Scale-up cue (required).",
            },
            "lower_threshold": {
                "type": "number",
                "description": "Scale-down cue (required, < upper_threshold).",
            },
            "lookback_window": {
                "type": "integer",
                "minimum": 1,
                "default": 10,
            },
        },
        callable=threshold_rules_forecast,
    ),
}


def get_function(name: str) -> ForecastFunction:
    """Look up a predefined forecasting function by name.

    Raises:
        KeyError: if the function is unknown.
    """
    descriptor = PREDEFINED.get(name)
    if descriptor is None:
        raise KeyError(name)
    return descriptor.callable


__all__ = [
    "ForecastFunction",
    "ForecastPoint",
    "FunctionDescriptor",
    "PREDEFINED",
    "get_function",
]
