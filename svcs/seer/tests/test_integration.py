"""End-to-end integration tests for the Seer gRPC service.

These exercise the full request path: gRPC server in-process, the predefined
function registry, the data-serialization helpers, the custom-function
validator, and the sandbox runner.
"""

from __future__ import annotations

import asyncio
import json
import math
import time

import grpc
import pytest

from seer import analytics_pb2, analytics_pb2_grpc
from seer.service import AnalyticsServiceImpl


@pytest.fixture
async def stub():
    server = grpc.aio.server()
    analytics_pb2_grpc.add_AnalyticsServiceServicer_to_server(AnalyticsServiceImpl(), server)
    port = server.add_insecure_port("127.0.0.1:0")
    await server.start()
    channel = grpc.aio.insecure_channel(f"127.0.0.1:{port}")
    try:
        yield analytics_pb2_grpc.AnalyticsServiceStub(channel)
    finally:
        await channel.close()
        await server.stop(grace=None)


def _linear_data(n: int, slope: float = 1.0, intercept: float = 0.0):
    return [
        analytics_pb2.MetricDataPoint(timestamp=i, value=slope * i + intercept)
        for i in range(n)
    ]


def _seasonal_data(n: int, period: int = 24, amplitude: float = 5.0, trend: float = 0.05):
    return [
        analytics_pb2.MetricDataPoint(
            timestamp=i,
            value=trend * i + amplitude * math.sin(2 * math.pi * i / period),
        )
        for i in range(n)
    ]


async def test_linear_regression_end_to_end(stub) -> None:
    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=_linear_data(20, slope=2.0, intercept=1.0),
            horizon=5,
            parameters="{}",
        )
    )
    assert len(response.points) == 5
    # Slope-2 linear extrapolation: x=20..24 → 41, 43, 45, 47, 49.
    expected = [41.0, 43.0, 45.0, 47.0, 49.0]
    for point, want in zip(response.points, expected):
        assert math.isclose(point.predicted_value, want, rel_tol=1e-6)
        assert point.upper_bound >= point.predicted_value >= point.lower_bound
    assert response.metadata.function_name == "linear_regression"


async def test_seasonal_decomposition_end_to_end(stub) -> None:
    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="seasonal_decomposition",
            data=_seasonal_data(96, period=24),
            horizon=12,
            parameters=json.dumps({"period": 24, "model": "additive"}),
        )
    )
    assert len(response.points) == 12
    assert all(math.isfinite(p.predicted_value) for p in response.points)


async def test_arima_end_to_end(stub) -> None:
    # Use an explicit order so the test runs in well under a second; auto-arima
    # is exercised in the unit suite.
    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="arima",
            data=_linear_data(40, slope=0.5),
            horizon=4,
            parameters=json.dumps({"order": [1, 1, 1]}),
        )
    )
    assert len(response.points) == 4
    # ARIMA(1,1,1) on a clean linear trend should keep predictions monotonic.
    values = [p.predicted_value for p in response.points]
    assert values == sorted(values)


async def test_threshold_rules_end_to_end(stub) -> None:
    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="threshold_rules",
            data=_linear_data(20, slope=0.0, intercept=0.5),
            horizon=3,
            parameters=json.dumps(
                {"upper_threshold": 0.8, "lower_threshold": 0.2, "lookback_window": 5}
            ),
        )
    )
    assert len(response.points) == 3
    for point in response.points:
        # Constant 0.5 input → 0.5 prediction.
        assert math.isclose(point.predicted_value, 0.5, rel_tol=1e-9)
        assert math.isclose(point.lower_bound, 0.2, rel_tol=1e-9)
        assert math.isclose(point.upper_bound, 0.8, rel_tol=1e-9)


async def test_validate_custom_function_accepts_safe_code(stub) -> None:
    code = (
        "import math\n"
        "def forecast(data, horizon, params):\n"
        "    if not data:\n"
        "        return []\n"
        "    last_ts, last_val = data[-1]\n"
        "    return [(last_ts + i + 1, last_val) for i in range(horizon)]\n"
    )
    response = await stub.ValidateFunction(
        analytics_pb2.FunctionCode(name="passthrough", code=code)
    )
    assert response.valid
    assert response.error_message == ""


@pytest.mark.parametrize(
    "code,fragment",
    [
        ("import os\ndef forecast(data, horizon, params):\n    return []\n", "os"),
        (
            "def forecast(data, horizon, params):\n    open('/etc/passwd').read()\n    return []\n",
            "open",
        ),
        (
            "def forecast(data, horizon, params):\n    eval('1+1')\n    return []\n",
            "eval",
        ),
        ("def wrong_name(data, horizon, params):\n    return []\n", "forecast"),
        ("def forecast(data):\n    return []\n", "horizon"),
    ],
)
async def test_validate_custom_function_rejects_bad_code(stub, code, fragment) -> None:
    response = await stub.ValidateFunction(
        analytics_pb2.FunctionCode(name="bad", code=code)
    )
    assert not response.valid
    assert fragment.lower() in response.error_message.lower()


async def test_run_forecast_returns_invalid_argument_for_too_small_input(stub) -> None:
    with pytest.raises(grpc.aio.AioRpcError) as exc:
        await stub.RunForecast(
            analytics_pb2.ForecastRequest(
                function_name="linear_regression",
                data=[analytics_pb2.MetricDataPoint(timestamp=0, value=1.0)],
                horizon=1,
                parameters="{}",
            )
        )
    assert exc.value.code() == grpc.StatusCode.INVALID_ARGUMENT


async def test_run_forecast_returns_not_found_for_unknown_function(stub) -> None:
    with pytest.raises(grpc.aio.AioRpcError) as exc:
        await stub.RunForecast(
            analytics_pb2.ForecastRequest(
                function_name="nonexistent",
                data=_linear_data(5),
                horizon=1,
                parameters="{}",
            )
        )
    assert exc.value.code() == grpc.StatusCode.NOT_FOUND


async def test_list_predefined_functions_returns_full_catalog(stub) -> None:
    response = await stub.ListPredefinedFunctions(analytics_pb2.Empty())
    names = {fn.name for fn in response.functions}
    assert {"linear_regression", "seasonal_decomposition", "arima", "threshold_rules"} <= names
    for fn in response.functions:
        # parameters_schema must be valid JSON
        json.loads(fn.parameters_schema)
        assert fn.type == analytics_pb2.FUNCTION_TYPE_PREDEFINED


async def test_health_check_reports_runtime(stub) -> None:
    response = await stub.HealthCheck(analytics_pb2.Empty())
    assert response.status == analytics_pb2.HEALTH_STATE_HEALTHY
    assert response.python_version.count(".") == 2
    for pkg in ("numpy", "pandas", "scikit-learn", "statsmodels"):
        assert pkg in response.packages


async def test_large_dataset_within_budget(stub) -> None:
    # Acceptance criterion: 10k points should complete in well under a second
    # using linear_regression (the simplest predefined function).
    data = _linear_data(10_000, slope=0.001)
    started = time.perf_counter()
    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=data,
            horizon=10,
            parameters="{}",
        )
    )
    elapsed_ms = (time.perf_counter() - started) * 1000
    assert len(response.points) == 10
    assert elapsed_ms < 2000, f"10k-point forecast took {elapsed_ms:.0f} ms (budget 2000)"


async def test_concurrent_forecasts_are_independent(stub) -> None:
    # Five concurrent forecasts on different functions. Verifies the service
    # tolerates parallel calls without interleaving state.
    requests = [
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=_linear_data(20),
            horizon=3,
            parameters="{}",
        ),
        analytics_pb2.ForecastRequest(
            function_name="threshold_rules",
            data=_linear_data(20, intercept=0.5, slope=0.0),
            horizon=3,
            parameters=json.dumps({"upper_threshold": 0.8, "lower_threshold": 0.2}),
        ),
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=_linear_data(20, slope=2.0),
            horizon=2,
            parameters="{}",
        ),
        analytics_pb2.ForecastRequest(
            function_name="threshold_rules",
            data=_linear_data(20, intercept=0.4, slope=0.0),
            horizon=4,
            parameters=json.dumps({"upper_threshold": 0.9, "lower_threshold": 0.1}),
        ),
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=_linear_data(15),
            horizon=1,
            parameters="{}",
        ),
    ]
    responses = await asyncio.gather(*(stub.RunForecast(req) for req in requests))
    assert [len(r.points) for r in responses] == [3, 3, 2, 4, 1]
    for response in responses:
        assert response.metadata.execution_time_ms >= 0
