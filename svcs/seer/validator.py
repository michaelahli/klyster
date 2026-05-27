"""Static validator for user-supplied forecasting functions.

Defends against the most common abuse patterns we expect from analyst-authored
code: blacklisted imports, attempts to escape via dunder attributes, and
explicit calls to `eval` / `exec` / `__import__` / `open`.

This is a defence-in-depth layer, not a security boundary. The actual
isolation (memory, cpu, time) is enforced by `seer.sandbox` running the code
in a subprocess.
"""

from __future__ import annotations

import ast
from dataclasses import dataclass, field
from typing import List, Set

REQUIRED_FUNCTION = "forecast"
REQUIRED_ARGS = ("data", "horizon", "params")

ALLOWED_TOP_LEVEL_MODULES: Set[str] = {
    "math",
    "statistics",
    "datetime",
    "typing",
    "json",
    "numpy",
    "pandas",
    "scipy",
    "sklearn",
    "statsmodels",
    "pmdarima",
}

FORBIDDEN_BUILTINS: Set[str] = {
    "eval",
    "exec",
    "compile",
    "open",
    "input",
    "breakpoint",
    "__import__",
    "globals",
    "locals",
    "vars",
}


@dataclass
class ValidationOutcome:
    """Result of validating a custom function source."""

    valid: bool
    error_message: str | None = None
    warnings: List[str] = field(default_factory=list)


def validate_custom_function(code: str) -> ValidationOutcome:
    """Statically check that `code` defines a safe `forecast` function.

    Args:
        code: Python source string.

    Returns:
        `ValidationOutcome` with `valid=True` when no violations are
        detected. Warnings hold non-fatal observations.
    """
    if not code or not code.strip():
        return ValidationOutcome(valid=False, error_message="code is empty")

    try:
        tree = ast.parse(code)
    except SyntaxError as exc:
        return ValidationOutcome(valid=False, error_message=f"syntax error: {exc.msg}")

    forecast_fn = _find_forecast_function(tree)
    if forecast_fn is None:
        return ValidationOutcome(
            valid=False,
            error_message=(
                f"missing top-level function `{REQUIRED_FUNCTION}"
                f"({', '.join(REQUIRED_ARGS)})`"
            ),
        )

    signature_error = _check_signature(forecast_fn)
    if signature_error is not None:
        return ValidationOutcome(valid=False, error_message=signature_error)

    visitor = _SafetyVisitor()
    visitor.visit(tree)
    if visitor.violations:
        return ValidationOutcome(
            valid=False,
            error_message=visitor.violations[0],
            warnings=visitor.violations[1:],
        )

    return ValidationOutcome(valid=True, warnings=visitor.advisories)


def _find_forecast_function(tree: ast.Module) -> ast.FunctionDef | ast.AsyncFunctionDef | None:
    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == REQUIRED_FUNCTION:
            return node
    return None


def _check_signature(node: ast.FunctionDef | ast.AsyncFunctionDef) -> str | None:
    args = node.args
    positional = [arg.arg for arg in args.args]
    if positional[: len(REQUIRED_ARGS)] != list(REQUIRED_ARGS):
        return (
            f"`{REQUIRED_FUNCTION}` must accept positional arguments "
            f"({', '.join(REQUIRED_ARGS)}), got ({', '.join(positional)})"
        )
    if isinstance(node, ast.AsyncFunctionDef):
        return f"`{REQUIRED_FUNCTION}` must be a regular function, not async"
    return None


class _SafetyVisitor(ast.NodeVisitor):
    """Collect static safety violations and advisories."""

    def __init__(self) -> None:
        self.violations: List[str] = []
        self.advisories: List[str] = []

    def visit_Import(self, node: ast.Import) -> None:
        for alias in node.names:
            self._check_module(alias.name, node)
        self.generic_visit(node)

    def visit_ImportFrom(self, node: ast.ImportFrom) -> None:
        if node.module is None:
            self.violations.append("relative imports are not allowed")
        else:
            self._check_module(node.module, node)
        self.generic_visit(node)

    def _check_module(self, module: str, node: ast.AST) -> None:
        top = module.split(".", 1)[0]
        if top not in ALLOWED_TOP_LEVEL_MODULES:
            self.violations.append(
                f"import of '{module}' is not allowed (line {getattr(node, 'lineno', '?')})"
            )

    def visit_Call(self, node: ast.Call) -> None:
        if isinstance(node.func, ast.Name) and node.func.id in FORBIDDEN_BUILTINS:
            self.violations.append(
                f"call to forbidden builtin `{node.func.id}` "
                f"(line {getattr(node, 'lineno', '?')})"
            )
        self.generic_visit(node)

    def visit_Attribute(self, node: ast.Attribute) -> None:
        if node.attr.startswith("__") and node.attr.endswith("__"):
            self.violations.append(
                f"access to dunder attribute `{node.attr}` is not allowed "
                f"(line {getattr(node, 'lineno', '?')})"
            )
        self.generic_visit(node)


__all__ = [
    "ALLOWED_TOP_LEVEL_MODULES",
    "FORBIDDEN_BUILTINS",
    "REQUIRED_ARGS",
    "REQUIRED_FUNCTION",
    "ValidationOutcome",
    "validate_custom_function",
]
