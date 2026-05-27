"""End-to-end tests for the gRPC service implementation."""

from __future__ import annotations

import json
import math

import grpc
import pytest

from seer import analytics_pb2, analytics_pb2_grpc
from seer.service import AnalyticsServiceImpl


@pytest.fixture
async def grpc_channel():
    server = grpc.aio.server()
    analytics_pb2_grpc.add_AnalyticsServiceServicer_to_server(AnalyticsServiceImpl(), server)
    port = server.add_insecure_port("127.0.0.1:0")
    await server.start()

    channel = grpc.aio.insecure_channel(f"127.0.0.1:{port}")
    try:
        yield channel
    finally:
        await channel.close()
        await server.stop(grace=None)


async def test_run_forecast_returns_predicted_points(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    data = [
        analytics_pb2.MetricDataPoint(timestamp=i, value=float(2 * i + 1))
        for i in range(10)
    ]

    response = await stub.RunForecast(
        analytics_pb2.ForecastRequest(
            function_name="linear_regression",
            data=data,
            horizon=3,
            parameters="{}",
        )
    )

    assert len(response.points) == 3
    assert response.metadata.function_name == "linear_regression"
    assert response.metadata.execution_time_ms >= 0
    for point, expected in zip(response.points, [21.0, 23.0, 25.0]):
        assert math.isclose(point.predicted_value, expected, rel_tol=1e-6)


async def test_run_forecast_unknown_function_returns_not_found(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    with pytest.raises(grpc.aio.AioRpcError) as exc:
        await stub.RunForecast(
            analytics_pb2.ForecastRequest(
                function_name="does_not_exist",
                data=[],
                horizon=1,
                parameters="",
            )
        )
    assert exc.value.code() == grpc.StatusCode.NOT_FOUND


async def test_run_forecast_invalid_argument_when_data_too_small(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    with pytest.raises(grpc.aio.AioRpcError) as exc:
        await stub.RunForecast(
            analytics_pb2.ForecastRequest(
                function_name="linear_regression",
                data=[analytics_pb2.MetricDataPoint(timestamp=0, value=1.0)],
                horizon=1,
                parameters="",
            )
        )
    assert exc.value.code() == grpc.StatusCode.INVALID_ARGUMENT


async def test_list_predefined_functions(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    response = await stub.ListPredefinedFunctions(analytics_pb2.Empty())
    names = [fn.name for fn in response.functions]
    assert "linear_regression" in names
    for fn in response.functions:
        # parameters_schema must be valid JSON
        json.loads(fn.parameters_schema)


async def test_health_check_reports_runtime(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    response = await stub.HealthCheck(analytics_pb2.Empty())
    assert response.status == analytics_pb2.HEALTH_STATE_HEALTHY
    assert response.python_version.count(".") == 2
    assert "numpy" in response.packages


async def test_validate_function_accepts_safe_code(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    response = await stub.ValidateFunction(
        analytics_pb2.FunctionCode(
            name="safe",
            code="def forecast(data, horizon, params):\n    return []\n",
        )
    )
    assert response.valid
    assert response.error_message == ""


async def test_validate_function_rejects_dangerous_code(grpc_channel) -> None:
    stub = analytics_pb2_grpc.AnalyticsServiceStub(grpc_channel)
    response = await stub.ValidateFunction(
        analytics_pb2.FunctionCode(
            name="unsafe",
            code="import os\ndef forecast(data, horizon, params):\n    return []\n",
        )
    )
    assert not response.valid
    assert "os" in response.error_message
