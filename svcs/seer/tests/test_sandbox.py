"""Tests for the custom function sandbox + validator."""

from __future__ import annotations

import textwrap

import pytest

from seer.sandbox import (
    SandboxError,
    SandboxTimeout,
    SandboxValidationError,
    execute_custom_function,
)
from seer.validator import validate_custom_function


VALID_CODE = textwrap.dedent(
    """
    def forecast(data, horizon, params):
        if not data:
            return []
        last_ts, last_value = data[-1]
        return [(last_ts + i + 1, last_value, last_value, last_value) for i in range(horizon)]
    """
)


def test_validator_accepts_well_formed_function():
    outcome = validate_custom_function(VALID_CODE)
    assert outcome.valid, outcome.error_message


def test_validator_rejects_missing_function():
    outcome = validate_custom_function("def other(): return 1\n")
    assert not outcome.valid


def test_validator_rejects_wrong_signature():
    outcome = validate_custom_function("def forecast(x, y): return []\n")
    assert not outcome.valid


def test_validator_rejects_async_function():
    outcome = validate_custom_function(
        "async def forecast(data, horizon, params): return []\n"
    )
    assert not outcome.valid


def test_validator_rejects_blacklisted_imports():
    code = textwrap.dedent(
        """
        import os
        def forecast(data, horizon, params):
            return []
        """
    )
    outcome = validate_custom_function(code)
    assert not outcome.valid
    assert "os" in (outcome.error_message or "")


def test_validator_rejects_subprocess_import():
    code = textwrap.dedent(
        """
        import subprocess
        def forecast(data, horizon, params):
            return []
        """
    )
    outcome = validate_custom_function(code)
    assert not outcome.valid


def test_validator_rejects_eval_call():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            return eval("[]")
        """
    )
    outcome = validate_custom_function(code)
    assert not outcome.valid
    assert "eval" in (outcome.error_message or "")


def test_validator_rejects_dunder_access():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            return data.__class__.__bases__
        """
    )
    outcome = validate_custom_function(code)
    assert not outcome.valid


def test_validator_rejects_syntax_error():
    outcome = validate_custom_function("def forecast(data, horizon, params)\n")
    assert not outcome.valid
    assert "syntax" in (outcome.error_message or "").lower()


def test_validator_rejects_empty_code():
    outcome = validate_custom_function("")
    assert not outcome.valid


def test_sandbox_runs_valid_function():
    data = [(i, float(i)) for i in range(5)]
    result = execute_custom_function(VALID_CODE, data, horizon=3, params={})
    assert len(result.forecast) == 3
    assert all(point.predicted_value == 4.0 for point in result.forecast)
    assert result.duration_ms >= 0
    assert result.max_rss_bytes > 0


def test_sandbox_propagates_user_value_error():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            raise ValueError("nope")
        """
    )
    with pytest.raises(SandboxError) as exc:
        execute_custom_function(code, [(0, 1.0)], horizon=1)
    assert "nope" in str(exc.value)


def test_sandbox_rejects_invalid_code_before_execution():
    with pytest.raises(SandboxValidationError):
        execute_custom_function(
            "import os\ndef forecast(data, horizon, params):\n    return []\n",
            [(0, 1.0)],
            horizon=1,
        )


def test_sandbox_kills_runaway_function():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            i = 0
            while True:
                i += 1
        """
    )
    with pytest.raises(SandboxTimeout):
        execute_custom_function(code, [(0, 1.0)], horizon=1, timeout_seconds=0.5)


def test_sandbox_validates_returned_shape():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            return [{"ts": 0, "v": 1.0}]
        """
    )
    with pytest.raises(SandboxError):
        execute_custom_function(code, [(0, 1.0)], horizon=1)


def test_sandbox_accepts_two_tuple_results():
    code = textwrap.dedent(
        """
        def forecast(data, horizon, params):
            return [(0, 1.0), (1, 2.0)]
        """
    )
    result = execute_custom_function(code, [(0, 1.0)], horizon=2)
    assert len(result.forecast) == 2
    assert result.forecast[0].predicted_value == 1.0
    assert result.forecast[0].lower_bound == result.forecast[0].upper_bound == 1.0
