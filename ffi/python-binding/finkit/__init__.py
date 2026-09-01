"""Finkit — high-performance financial technical analysis library.

The native Rust extension is exposed through this package-level namespace.
Optional pandas integration is registered by the finkit.accessor module.
"""

from __future__ import annotations

from functools import wraps

import numpy as np

from . import finkit as _native
from .finkit import *  # noqa: F401,F403 — re-export native module

KlineData = _native.PyKlineData
KlineChart = _native.PyKlineChart

_native_all = getattr(
    _native,
    "__all__",
    [name for name in dir(_native) if not name.startswith("_")],
)

from finkit.exceptions import (  # noqa: E402,F401
    FinkitError,
    InsufficientDataError,
    InvalidParameterError,
    IndicatorNotFoundError,
)

from finkit.accessor import TaAccessor  # noqa: E402,F401


def _translate_native_errors(name, function):
    """Convert common native validation errors to stable Python exceptions."""

    @wraps(function)
    def wrapped(*args, **kwargs):
        if name == "macd":
            periods = (
                kwargs.get("fastperiod", args[1] if len(args) > 1 else 12),
                kwargs.get("slowperiod", args[2] if len(args) > 2 else 26),
                kwargs.get("signalperiod", args[3] if len(args) > 3 else 9),
            )
            if any(period <= 0 for period in periods):
                raise InvalidParameterError("MACD periods must be greater than 0")
        try:
            return function(*args, **kwargs)
        except OverflowError as exc:
            raise InvalidParameterError(str(exc)) from exc
        except ValueError as exc:
            message = str(exc)
            lowered = message.lower()
            if (
                "input data is empty" in lowered
                or "less than required minimum" in lowered
                or "insufficient data" in lowered
            ):
                raise InsufficientDataError(message) from exc
            if "invalid parameter" in lowered or "period must be greater than" in lowered:
                raise InvalidParameterError(message) from exc
            raise

    return wrapped


def _as_numpy_result(name, function):
    """Expose native numeric results as NumPy arrays consistently.

    The low-level Rust ABI may return Vec values, dictionaries, or tuples.
    Convert nested numeric containers at the package boundary so every public
    numeric API follows the NumPy-facing type contract.
    """

    def convert(value):
        if isinstance(value, dict):
            return {key: convert(item) for key, item in value.items()}
        if isinstance(value, tuple):
            return tuple(convert(item) for item in value)
        if isinstance(value, list):
            if not value or all(
                not isinstance(item, (dict, list, tuple)) for item in value
            ):
                return np.asarray(value)
            return [convert(item) for item in value]
        return value

    @wraps(function)
    def wrapped(*args, **kwargs):
        return convert(function(*args, **kwargs))

    return wrapped


for _name in _native_all:
    _function = globals().get(_name)
    if callable(_function) and not isinstance(_function, type):
        wrapped = _translate_native_errors(_name, _function)
        globals()[_name] = _as_numpy_result(_name, wrapped)


def register_accessor():
    """Explicitly register the df.ta accessor (idempotent)."""
    TaAccessor._register()


__all__ = list(_native_all) + [
    "KlineData",
    "KlineChart",
    "FinkitError",
    "InsufficientDataError",
    "InvalidParameterError",
    "IndicatorNotFoundError",
    "TaAccessor",
    "register_accessor",
]
