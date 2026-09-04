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


def _as_contiguous_float64(values):
    """Borrow an existing C-contiguous float64 array, copying only when required."""
    array = np.asarray(values)
    if array.ndim != 1:
        raise InvalidParameterError("expected a one-dimensional numeric array")
    if array.dtype != np.float64 or not array.flags.c_contiguous:
        array = np.ascontiguousarray(array, dtype=np.float64)
    return array


def _as_contiguous_float_array(values):
    """Preserve native float32/float64 arrays for typed vector hot paths."""
    array = np.asarray(values)
    if array.ndim != 1:
        raise InvalidParameterError("expected a one-dimensional numeric array")
    if array.dtype == np.float32:
        return np.ascontiguousarray(array, dtype=np.float32)
    if array.dtype == np.float64:
        return np.ascontiguousarray(array, dtype=np.float64)
    return np.ascontiguousarray(array, dtype=np.float64)


def _as_contiguous_reduction_input(values):
    """Keep float32/float64 reductions on their native typed kernels."""
    array = _as_contiguous_float_array(values)
    if array.dtype == np.float32:
        return array, np.float32
    return array, float


# Architecture 3.0 P0: keep the established package API but route the hottest
# single-output indicators through native functions that return NumPy arrays
# directly. For already-contiguous float32/float64 inputs the Rust binding borrows
# the NumPy memory and there is no input conversion/copy.
if hasattr(_native, "_fast_sma"):

    def sma(close, timeperiod=14):
        close = _as_contiguous_float_array(close)
        if close.dtype == np.float32 and hasattr(_native, "_fast_sma_f32"):
            return _native._fast_sma_f32(close, timeperiod)
        return _native._fast_sma(close, timeperiod)

    sma = _translate_native_errors("sma", sma)

if hasattr(_native, "_fast_ema"):

    def ema(close, timeperiod=14):
        close = _as_contiguous_float_array(close)
        if close.dtype == np.float32 and hasattr(_native, "_fast_ema_f32"):
            return _native._fast_ema_f32(close, timeperiod)
        return _native._fast_ema(close, timeperiod)

    ema = _translate_native_errors("ema", ema)

if hasattr(_native, "_fast_obv"):

    def obv(close, volume):
        return _native._fast_obv(
            _as_contiguous_float64(close),
            _as_contiguous_float64(volume),
        )

    obv = _translate_native_errors("obv", obv)

if hasattr(_native, "_fast_vwap"):

    def vwap(high, low, close, volume):
        return _native._fast_vwap(
            _as_contiguous_float64(high),
            _as_contiguous_float64(low),
            _as_contiguous_float64(close),
            _as_contiguous_float64(volume),
        )

    vwap = _translate_native_errors("vwap", vwap)


def _typed_reduce(values, f32_name, f64_name):
    array, scalar_type = _as_contiguous_reduction_input(values)
    native = getattr(_native, f32_name if array.dtype == np.float32 else f64_name)
    result = native(array)
    return scalar_type(result)


def reduce_sum(values):
    """Allocation-free scalar sum preserving float32 vs float64 input type."""
    return _typed_reduce(values, "_reduce_sum_f32", "_reduce_sum_f64")


def reduce_mean(values):
    """Allocation-free scalar arithmetic mean preserving input floating type."""
    return _typed_reduce(values, "_reduce_mean_f32", "_reduce_mean_f64")


def reduce_min(values):
    """Allocation-free scalar minimum preserving input floating type."""
    return _typed_reduce(values, "_reduce_min_f32", "_reduce_min_f64")


def reduce_max(values):
    """Allocation-free scalar maximum preserving input floating type."""
    return _typed_reduce(values, "_reduce_max_f32", "_reduce_max_f64")


def reduce_stddev(values):
    """Allocation-free population standard deviation preserving floating type."""
    return _typed_reduce(values, "_reduce_stddev_f32", "_reduce_stddev_f64")


# TA-Lib-compatible public boundary. Keep compatibility handling in Python so the
# native hot kernels do not carry keyword/shape policy branches on every call.
if "sar" in globals():
    _sar_impl = sar

    def sar(high, low, acceleration=0.02, maximum=0.2):
        """Parabolic SAR with the single-array TA-Lib public result shape."""
        result = _sar_impl(high, low, acceleration=acceleration, maximum=maximum)
        if isinstance(result, tuple):
            return result[0]
        return result


if "bollinger_bands" in globals():
    _bollinger_bands_impl = bollinger_bands

    def bollinger_bands(
        close,
        timeperiod=20,
        nbdevup=2.0,
        nbdevdn=2.0,
        matype=0,
    ):
        """TA-Lib-compatible BBANDS keyword surface for the supported SMA mode."""
        if matype != 0:
            raise ValueError("bollinger_bands currently supports matype=0 only")
        return _bollinger_bands_impl(
            close,
            timeperiod=timeperiod,
            nbdevup=nbdevup,
            nbdevdn=nbdevdn,
        )


if "stoch" in globals():
    _stoch_impl = stoch

    def stoch(
        high,
        low,
        close,
        fastk_period=5,
        slowk_period=3,
        slowk_matype=0,
        slowd_period=3,
        slowd_matype=0,
    ):
        """TA-Lib-compatible STOCH keyword surface for SMA smoothing."""
        if slowk_matype != 0:
            raise ValueError("stoch currently supports slowk_matype=0 only")
        if slowd_matype != 0:
            raise ValueError("stoch currently supports slowd_matype=0 only")
        return _stoch_impl(
            high,
            low,
            close,
            fastk_period=fastk_period,
            slowk_period=slowk_period,
            slowd_period=slowd_period,
        )


if "std_dev" in globals():
    _std_dev_impl = std_dev

    def stddev(close, timeperiod=20, nbdev=1.0):
        """TA-Lib spelling alias for Finkit's standard-deviation indicator."""
        return _std_dev_impl(close, timeperiod=timeperiod, nbdev=nbdev)


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
    "reduce_sum",
    "reduce_mean",
    "reduce_min",
    "reduce_max",
    "reduce_stddev",
]
if "stddev" in globals() and "stddev" not in __all__:
    __all__.append("stddev")
