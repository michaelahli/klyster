"""Analytics service implementation."""

from __future__ import annotations

import json
import logging
import time
from typing import Any, Dict

import grpc

from seer import analytics_pb2, analytics_pb2_grpc
from seer.functions import PREDEFINED, get_function
from seer.serialization import data_points_to_pairs


class AnalyticsServiceImpl(analytics_pb2_grpc.AnalyticsServiceServicer):
    """Implementation of AnalyticsService."""

    def __init__(self) -> None:
        """Initialize service."""
        self.logger = logging.getLogger(__name__)

    async def RunForecast(self, request, context):
        """Run a forecast using the requested predefined function."""
        self.logger.info(
            "RunForecast called: function=%s data_points=%d horizon=%d",
            request.function_name,
            len(request.data),
            request.horizon,
        )

        try:
            forecast_fn = get_function(request.function_name)
        except KeyError:
            await context.abort(
                grpc.StatusCode.NOT_FOUND,
                f"unknown function '{request.function_name}'",
            )
            return analytics_pb2.ForecastResponse()

        params = _parse_parameters(request.parameters)
        data = data_points_to_pairs(request.data)

        started = time.perf_counter()
        try:
            forecast_points = forecast_fn(data, request.horizon, params)
        except ValueError as exc:
            await context.abort(grpc.StatusCode.INVALID_ARGUMENT, str(exc))
            return analytics_pb2.ForecastResponse()
        except Exception as exc:  # pragma: no cover - defensive
            self.logger.exception("forecast function raised an unexpected error")
            await context.abort(grpc.StatusCode.INTERNAL, f"forecast failed: {exc}")
            return analytics_pb2.ForecastResponse()
        elapsed_ms = int((time.perf_counter() - started) * 1000)

        response = analytics_pb2.ForecastResponse(
            points=[
                analytics_pb2.ForecastPoint(
                    timestamp=pt.timestamp,
                    predicted_value=pt.predicted_value,
                    lower_bound=pt.lower_bound,
                    upper_bound=pt.upper_bound,
                )
                for pt in forecast_points
            ],
            metadata=analytics_pb2.ForecastMetadata(
                function_name=request.function_name,
                execution_time_ms=elapsed_ms,
                parameters=json.dumps(params),
                quality_metrics={},
            ),
        )
        return response

    async def ValidateFunction(self, request, context):
        """Validate custom function code via static analysis."""
        self.logger.info("ValidateFunction called: name=%s", request.name)
        from seer.validator import validate_custom_function

        outcome = validate_custom_function(request.code)
        return analytics_pb2.ValidationResult(
            valid=outcome.valid,
            error_message=outcome.error_message,
            warnings=outcome.warnings,
        )

    async def ListPredefinedFunctions(self, request, context):
        """Return the catalog of predefined forecasting functions."""
        self.logger.debug("ListPredefinedFunctions called")
        functions = [
            analytics_pb2.FunctionInfo(
                name=descriptor.name,
                type=analytics_pb2.FUNCTION_TYPE_PREDEFINED,
                description=descriptor.description,
                parameters_schema=json.dumps(descriptor.parameters_schema),
            )
            for descriptor in PREDEFINED.values()
        ]
        return analytics_pb2.FunctionList(functions=functions)

    async def HealthCheck(self, request, context):
        """Report service health and Python runtime metadata."""
        self.logger.debug("HealthCheck called")
        import sys

        return analytics_pb2.HealthStatus(
            status=analytics_pb2.HEALTH_STATE_HEALTHY,
            python_version=f"{sys.version_info.major}.{sys.version_info.minor}.{sys.version_info.micro}",
            packages=_collect_package_versions(),
            message="ok",
        )


def _parse_parameters(raw: str) -> Dict[str, Any]:
    if not raw:
        return {}
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ValueError(f"parameters must be valid JSON: {exc}") from exc
    if not isinstance(parsed, dict):
        raise ValueError("parameters must decode to a JSON object")
    return parsed


def _collect_package_versions() -> Dict[str, str]:
    versions: Dict[str, str] = {}
    for name in ("numpy", "pandas", "scikit-learn", "statsmodels"):
        module_name = name.replace("-", "_")
        try:
            module = __import__(module_name)
        except ImportError:
            versions[name] = "missing"
            continue
        versions[name] = getattr(module, "__version__", "unknown")
    return versions
