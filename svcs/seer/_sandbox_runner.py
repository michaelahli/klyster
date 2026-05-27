"""Subprocess entry point for sandboxed custom function execution.

Reads a JSON payload from stdin, applies resource limits, executes the user
code in a restricted namespace, and writes a JSON response to stdout.
"""

from __future__ import annotations

import json
import resource
import sys
import traceback
from typing import Any, Dict


def _apply_limits(payload: Dict[str, Any]) -> None:
    memory_limit = int(payload.get("memory_limit", 0))
    if memory_limit > 0:
        try:
            resource.setrlimit(resource.RLIMIT_AS, (memory_limit, memory_limit))
        except (ValueError, OSError):
            pass

    cpu_seconds = int(payload.get("cpu_seconds", 0))
    if cpu_seconds > 0:
        try:
            resource.setrlimit(resource.RLIMIT_CPU, (cpu_seconds, cpu_seconds))
        except (ValueError, OSError):
            pass


def _build_namespace() -> Dict[str, Any]:
    safe_builtins = {
        name: getattr(__builtins__, name)
        if hasattr(__builtins__, name)
        else __builtins__[name]
        for name in (
            "abs",
            "all",
            "any",
            "bool",
            "dict",
            "enumerate",
            "filter",
            "float",
            "int",
            "len",
            "list",
            "map",
            "max",
            "min",
            "range",
            "round",
            "set",
            "sorted",
            "str",
            "sum",
            "tuple",
            "zip",
            "True",
            "False",
            "None",
            "isinstance",
            "Exception",
            "ValueError",
            "TypeError",
            "ZeroDivisionError",
            "print",
        )
        if hasattr(__builtins__, name) or (isinstance(__builtins__, dict) and name in __builtins__)
    }
    return {"__builtins__": safe_builtins}


def main() -> int:
    payload = json.loads(sys.stdin.read())
    _apply_limits(payload)

    namespace = _build_namespace()

    try:
        exec(compile(payload["code"], "<custom_function>", "exec"), namespace)
        forecast_fn = namespace.get("forecast")
        if not callable(forecast_fn):
            raise RuntimeError("custom code did not define a callable `forecast`")

        data = [tuple(point) for point in payload["data"]]
        result = forecast_fn(data, payload["horizon"], payload["params"])
        normalised = [list(point) for point in result]
    except MemoryError:
        _emit({"ok": False, "error": "memory limit exceeded"})
        return 0
    except Exception as exc:  # noqa: BLE001 - report any user-side error
        _emit(
            {
                "ok": False,
                "error": f"{type(exc).__name__}: {exc}",
                "traceback": traceback.format_exc(),
            }
        )
        return 0

    usage = resource.getrusage(resource.RUSAGE_SELF)
    # macOS reports max_rss in bytes; Linux reports kilobytes.
    max_rss_bytes = usage.ru_maxrss if sys.platform == "darwin" else usage.ru_maxrss * 1024
    _emit(
        {
            "ok": True,
            "forecast": normalised,
            "max_rss_bytes": int(max_rss_bytes),
        }
    )
    return 0


def _emit(payload: Dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(payload))
    sys.stdout.flush()


if __name__ == "__main__":
    sys.exit(main())
