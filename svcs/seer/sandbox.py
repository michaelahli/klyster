"""Run user-supplied forecasting functions in a resource-limited subprocess."""

from __future__ import annotations

import json
import logging
import os
import resource
import subprocess
import sys
import textwrap
import time
from dataclasses import dataclass
from typing import Any, Dict, List, Sequence, Tuple

from seer.functions.types import ForecastPoint
from seer.validator import ValidationOutcome, validate_custom_function

DEFAULT_TIMEOUT_SECONDS = 60.0
DEFAULT_MEMORY_LIMIT_BYTES = 512 * 1024 * 1024  # 512 MiB

_LOGGER = logging.getLogger(__name__)


@dataclass
class SandboxResult:
    """Outcome of executing a custom function."""

    forecast: List[ForecastPoint]
    duration_ms: int
    max_rss_bytes: int


class SandboxError(RuntimeError):
    """Raised when sandboxed execution fails."""


class SandboxTimeout(SandboxError):
    """Raised when the user code exceeded the wall-time budget."""


class SandboxValidationError(SandboxError):
    """Raised when validation rejects the user code before execution."""

    def __init__(self, outcome: ValidationOutcome):
        super().__init__(outcome.error_message or "validation failed")
        self.outcome = outcome


def execute_custom_function(
    code: str,
    data: Sequence[Tuple[int, float]],
    horizon: int,
    params: Dict[str, Any] | None = None,
    *,
    timeout_seconds: float = DEFAULT_TIMEOUT_SECONDS,
    memory_limit_bytes: int = DEFAULT_MEMORY_LIMIT_BYTES,
) -> SandboxResult:
    """Validate and run a user-supplied forecasting function.

    Args:
        code: Python source string defining `forecast(data, horizon, params)`.
        data: Historical observations as (timestamp, value) pairs.
        horizon: Number of points to predict.
        params: Optional parameter dict forwarded to the user function.
        timeout_seconds: Wall-time budget; when exceeded the subprocess is
            killed and `SandboxTimeout` is raised.
        memory_limit_bytes: RLIMIT_AS applied to the subprocess.

    Returns:
        `SandboxResult` carrying forecast points, duration, and peak RSS.

    Raises:
        SandboxValidationError: when static validation rejects the code.
        SandboxTimeout: when execution exceeds `timeout_seconds`.
        SandboxError: when the subprocess fails or returns invalid output.
    """
    outcome = validate_custom_function(code)
    if not outcome.valid:
        raise SandboxValidationError(outcome)

    payload = {
        "code": code,
        "data": [list(point) for point in data],
        "horizon": int(horizon),
        "params": params or {},
        "memory_limit": int(memory_limit_bytes),
        "cpu_seconds": int(timeout_seconds) + 5,
    }

    runner_path = os.path.join(os.path.dirname(__file__), "_sandbox_runner.py")
    cmd = [sys.executable, runner_path]
    started = time.perf_counter()
    try:
        completed = subprocess.run(
            cmd,
            input=json.dumps(payload),
            capture_output=True,
            text=True,
            timeout=timeout_seconds,
            check=False,
        )
    except subprocess.TimeoutExpired as exc:
        raise SandboxTimeout(
            f"custom function exceeded {timeout_seconds:.1f}s wall-time budget"
        ) from exc
    duration_ms = int((time.perf_counter() - started) * 1000)

    if completed.returncode != 0:
        stderr = completed.stderr.strip() or completed.stdout.strip()
        raise SandboxError(
            f"sandbox subprocess exited with status {completed.returncode}: {stderr}"
        )

    try:
        response = json.loads(completed.stdout)
    except json.JSONDecodeError as exc:
        raise SandboxError(f"sandbox returned invalid JSON: {exc}") from exc

    if not response.get("ok", False):
        raise SandboxError(
            f"custom function raised: {response.get('error', 'unknown error')}"
        )

    points = _coerce_points(response.get("forecast", []))
    max_rss_bytes = int(response.get("max_rss_bytes", 0))
    _LOGGER.info(
        "custom function executed: points=%d duration_ms=%d max_rss_bytes=%d",
        len(points),
        duration_ms,
        max_rss_bytes,
    )
    return SandboxResult(forecast=points, duration_ms=duration_ms, max_rss_bytes=max_rss_bytes)


def _coerce_points(raw: Any) -> List[ForecastPoint]:
    if not isinstance(raw, list):
        raise SandboxError("custom function must return a list of forecast tuples")

    points: List[ForecastPoint] = []
    for index, item in enumerate(raw):
        if not isinstance(item, (list, tuple)) or len(item) not in (2, 4):
            raise SandboxError(
                f"forecast tuple #{index} must have 2 or 4 elements (got {item!r})"
            )
        if len(item) == 2:
            timestamp, predicted = item
            lower = upper = predicted
        else:
            timestamp, predicted, lower, upper = item
        try:
            points.append(
                ForecastPoint(
                    timestamp=int(timestamp),
                    predicted_value=float(predicted),
                    lower_bound=float(lower),
                    upper_bound=float(upper),
                )
            )
        except (TypeError, ValueError) as exc:
            raise SandboxError(f"forecast tuple #{index} has invalid values: {exc}") from exc
    return points


__all__ = [
    "DEFAULT_MEMORY_LIMIT_BYTES",
    "DEFAULT_TIMEOUT_SECONDS",
    "SandboxError",
    "SandboxResult",
    "SandboxTimeout",
    "SandboxValidationError",
    "execute_custom_function",
]
