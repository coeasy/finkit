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
    """Expose native numeric results as NumPy arrays consistently."""

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
    array = np.asarray(values)
    if array.ndim != 1:
        raise InvalidParameterError("expected a one-dimensional numeric array")
    if array.dtype != np.float64 or not array.flags.c_contiguous:
        array = np.ascontiguousarray(array, dtype=np.float64)
    return array


def _as_contiguous_float_array(values):
    array = np.asarray(values)
    if array.ndim != 1:
        raise InvalidParameterError("expected a one-dimensional numeric array")
    if array.dtype == np.float32:
        return np.ascontiguousarray(array, dtype=np.float32)
    if array.dtype == np.float64:
        return np.ascontiguousarray(array, dtype=np.float64)
    return np.ascontiguousarray(array, dtype=np.float64)


def _validate_out(out, source):
    if not isinstance(out, np.ndarray):
        raise InvalidParameterError("out must be a NumPy ndarray")
    if out.ndim != 1 or out.shape != source.shape:
        raise InvalidParameterError("out must be one-dimensional and match input shape")
    if out.dtype != source.dtype:
        raise InvalidParameterError("out dtype must match the normalized input dtype")
    if not out.flags.c_contiguous or not out.flags.writeable:
        raise InvalidParameterError("out must be writable and C-contiguous")
    if np.shares_memory(out, source):
        raise InvalidParameterError("out must not overlap input for this kernel")
    return out


def _as_contiguous_reduction_input(values):
    array = _as_contiguous_float_array(values)
    if array.dtype == np.float32:
        return array, np.float32
    return array, float


if hasattr(_native, "_fast_sma"):

    def sma(close, timeperiod=14, out=None):
        close = _as_contiguous_float_array(close)
        if out is not None:
            out = _validate_out(out, close)
            if close.dtype == np.float32 and hasattr(_native, "_fast_sma_f32_into"):
                _native._fast_sma_f32_into(close, out, timeperiod)
            else:
                _native._fast_sma_into(close, out, timeperiod)
            return out
        if close.dtype == np.float32 and hasattr(_native, "_fast_sma_f32"):
            return _native._fast_sma_f32(close, timeperiod)
        return _native._fast_sma(close, timeperiod)

    sma = _translate_native_errors("sma", sma)

if hasattr(_native, "_fast_ema"):

    def ema(close, timeperiod=14, out=None):
        close = _as_contiguous_float_array(close)
        if out is not None:
            out = _validate_out(out, close)
            if close.dtype == np.float32 and hasattr(_native, "_fast_ema_f32_into"):
                _native._fast_ema_f32_into(close, out, timeperiod)
            else:
                _native._fast_ema_into(close, out, timeperiod)
            return out
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
    return _typed_reduce(values, "_reduce_sum_f32", "_reduce_sum_f64")


def reduce_mean(values):
    return _typed_reduce(values, "_reduce_mean_f32", "_reduce_mean_f64")


def reduce_min(values):
    return _typed_reduce(values, "_reduce_min_f32", "_reduce_min_f64")


def reduce_max(values):
    return _typed_reduce(values, "_reduce_max_f32", "_reduce_max_f64")


def reduce_stddev(values):
    return _typed_reduce(values, "_reduce_stddev_f32", "_reduce_stddev_f64")


if "sar" in globals():
    _sar_impl = sar

    def sar(high, low, acceleration=0.02, maximum=0.2):
        result = _sar_impl(high, low, acceleration=acceleration, maximum=maximum)
        if isinstance(result, tuple):
            return result[0]
        return result


if "bollinger_bands" in globals():
    _bollinger_bands_impl = bollinger_bands

    def bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0):
        if matype != 0:
            raise ValueError("bollinger_bands currently supports matype=0 only")
        return _bollinger_bands_impl(
            close, timeperiod=timeperiod, nbdevup=nbdevup, nbdevdn=nbdevdn
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
        return _std_dev_impl(close, timeperiod=timeperiod, nbdev=nbdev)


if "correlation" in globals():
    _correlation_impl = correlation

    def correl(input_a, input_b, timeperiod=30):
        return _correlation_impl(input_a, input_b, timeperiod=timeperiod)


def register_accessor():
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
if "correl" in globals() and "correl" not in __all__:
    __all__.append("correl")
