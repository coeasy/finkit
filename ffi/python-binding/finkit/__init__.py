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
    """Expose legacy native numeric results as NumPy arrays consistently."""

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


def _validate_out(out, source, *other_inputs):
    if not isinstance(out, np.ndarray):
        raise InvalidParameterError("out must be a NumPy ndarray")
    if out.ndim != 1 or out.shape != source.shape:
        raise InvalidParameterError("out must be one-dimensional and match input shape")
    if out.dtype != source.dtype:
        raise InvalidParameterError("out dtype must match the normalized input dtype")
    if not out.flags.c_contiguous or not out.flags.writeable:
        raise InvalidParameterError("out must be writable and C-contiguous")
    if any(np.shares_memory(out, values) for values in (source, *other_inputs)):
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

if hasattr(_native, "_fast_wma"):

    def wma(close, timeperiod=14, out=None):
        close = _as_contiguous_float64(close)
        if out is not None:
            out = _validate_out(out, close)
            _native._fast_wma_into(close, out, timeperiod)
            return out
        return _native._fast_wma(close, timeperiod)

    wma = _translate_native_errors("wma", wma)

if hasattr(_native, "_fast_obv"):

    def obv(close, volume, out=None):
        close = _as_contiguous_float64(close)
        volume = _as_contiguous_float64(volume)
        if out is not None:
            out = _validate_out(out, close, volume)
            _native._fast_obv_into(close, volume, out)
            return out
        return _native._fast_obv(close, volume)

    obv = _translate_native_errors("obv", obv)

if hasattr(_native, "_fast_vwap"):

    def vwap(high, low, close, volume, out=None):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        volume = _as_contiguous_float64(volume)
        if out is not None:
            out = _validate_out(out, high, low, close, volume)
            _native._fast_vwap_into(high, low, close, volume, out)
            return out
        return _native._fast_vwap(high, low, close, volume)

    vwap = _translate_native_errors("vwap", vwap)


# Architecture v3 direct-ndarray facade.  The registry-generated functions stay
# available in the native module as the compatibility surface; these public
# wrappers avoid their Vec -> Python list -> np.asarray materialisation cost.
if hasattr(_native, "_fast_unary_period"):

    def _unary_period(operation, values, timeperiod):
        values = _as_contiguous_float64(values)
        return _native._fast_unary_period(operation, values, timeperiod)

    def dema(close, timeperiod=14):
        return _unary_period("dema", close, timeperiod)

    def tema(close, timeperiod=14):
        return _unary_period("tema", close, timeperiod)

    def midpoint(close, timeperiod=14):
        return _unary_period("midpoint", close, timeperiod)

    def rsi(close, timeperiod=14):
        return _unary_period("rsi", close, timeperiod)

    def mom(close, timeperiod=10):
        return _unary_period("mom", close, timeperiod)

    def roc(close, timeperiod=10):
        return _unary_period("roc", close, timeperiod)

    def cmo(close, timeperiod=14):
        return _unary_period("cmo", close, timeperiod)

    for _fast_name in ("dema", "tema", "midpoint", "rsi", "mom", "roc", "cmo"):
        globals()[_fast_name] = _translate_native_errors(
            _fast_name, globals()[_fast_name]
        )

if hasattr(_native, "_fast_unary_period_scale"):

    def stddev(close, timeperiod=20, nbdev=1.0):
        close = _as_contiguous_float64(close)
        return _native._fast_unary_period_scale("stddev", close, timeperiod, nbdev)

    def var(close, timeperiod=5, nbdev=1.0):
        close = _as_contiguous_float64(close)
        return _native._fast_unary_period_scale("var", close, timeperiod, nbdev)

    stddev = _translate_native_errors("stddev", stddev)
    var = _translate_native_errors("var", var)

if hasattr(_native, "_fast_kama"):

    def kama(close, timeperiod=10, fastperiod=2, slowperiod=30):
        close = _as_contiguous_float64(close)
        return _native._fast_kama(close, timeperiod, fastperiod, slowperiod)

    kama = _translate_native_errors("kama", kama)

if hasattr(_native, "_fast_binary_period"):

    def midprice(high, low, timeperiod=14):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        return _native._fast_binary_period("midprice", high, low, timeperiod)

    def correlation(input_a, input_b, timeperiod=14):
        input_a = _as_contiguous_float64(input_a)
        input_b = _as_contiguous_float64(input_b)
        return _native._fast_binary_period("correl", input_a, input_b, timeperiod)

    def correl(input_a, input_b, timeperiod=30):
        return correlation(input_a, input_b, timeperiod=timeperiod)

    midprice = _translate_native_errors("midprice", midprice)
    correlation = _translate_native_errors("correlation", correlation)
    correl = _translate_native_errors("correl", correl)

if hasattr(_native, "_fast_hlc_period"):

    def _hlc_period(operation, high, low, close, timeperiod):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        return _native._fast_hlc_period(operation, high, low, close, timeperiod)

    def adx(high, low, close, timeperiod=14):
        return _hlc_period("adx", high, low, close, timeperiod)

    def cci(high, low, close, timeperiod=14):
        return _hlc_period("cci", high, low, close, timeperiod)

    def willr(high, low, close, timeperiod=14):
        return _hlc_period("willr", high, low, close, timeperiod)

    def plus_di(high, low, close, timeperiod=14):
        return _hlc_period("plus_di", high, low, close, timeperiod)

    def minus_di(high, low, close, timeperiod=14):
        return _hlc_period("minus_di", high, low, close, timeperiod)

    def atr(high, low, close, timeperiod=14):
        return _hlc_period("atr", high, low, close, timeperiod)

    def natr(high, low, close, timeperiod=14):
        return _hlc_period("natr", high, low, close, timeperiod)

    for _fast_name in ("adx", "cci", "willr", "plus_di", "minus_di", "atr", "natr"):
        globals()[_fast_name] = _translate_native_errors(
            _fast_name, globals()[_fast_name]
        )

if hasattr(_native, "_fast_trange"):

    def trange(high, low, close):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        return _native._fast_trange(high, low, close)

    trange = _translate_native_errors("trange", trange)

if hasattr(_native, "_fast_mfi"):

    def mfi(high, low, close, volume, timeperiod=14):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        volume = _as_contiguous_float64(volume)
        return _native._fast_mfi(high, low, close, volume, timeperiod)

    mfi = _translate_native_errors("mfi", mfi)

if hasattr(_native, "_fast_ad"):

    def ad(high, low, close, volume):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        volume = _as_contiguous_float64(volume)
        return _native._fast_ad(high, low, close, volume)

    ad = _translate_native_errors("ad", ad)

if hasattr(_native, "_fast_adosc"):

    def adosc(high, low, close, volume, fastperiod=3, slowperiod=10):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        volume = _as_contiguous_float64(volume)
        return _native._fast_adosc(high, low, close, volume, fastperiod, slowperiod)

    adosc = _translate_native_errors("adosc", adosc)

if hasattr(_native, "_fast_bop"):

    def bop(open, high, low, close):
        open = _as_contiguous_float64(open)
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        return _native._fast_bop(open, high, low, close)

    bop = _translate_native_errors("bop", bop)

if hasattr(_native, "_fast_bbands"):

    def bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0):
        if matype != 0:
            raise ValueError("bollinger_bands currently supports matype=0 only")
        close = _as_contiguous_float64(close)
        return _native._fast_bbands(close, timeperiod, nbdevup, nbdevdn)

    bollinger_bands = _translate_native_errors("bollinger_bands", bollinger_bands)

elif "bollinger_bands" in globals():
    _bollinger_bands_impl = bollinger_bands

    def bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0):
        if matype != 0:
            raise ValueError("bollinger_bands currently supports matype=0 only")
        return _bollinger_bands_impl(
            close, timeperiod=timeperiod, nbdevup=nbdevup, nbdevdn=nbdevdn
        )

if hasattr(_native, "_fast_sar"):

    def sar(high, low, acceleration=0.02, maximum=0.2):
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        return _native._fast_sar(high, low, acceleration, maximum)

    sar = _translate_native_errors("sar", sar)

elif "sar" in globals():
    _sar_impl = sar

    def sar(high, low, acceleration=0.02, maximum=0.2):
        result = _sar_impl(high, low, acceleration=acceleration, maximum=maximum)
        if isinstance(result, tuple):
            return result[0]
        return result

if hasattr(_native, "_fast_macd"):

    def macd(close, fastperiod=12, slowperiod=26, signalperiod=9):
        close = _as_contiguous_float64(close)
        return _native._fast_macd(close, fastperiod, slowperiod, signalperiod)

    macd = _translate_native_errors("macd", macd)

if hasattr(_native, "_fast_stoch"):

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
        high = _as_contiguous_float64(high)
        low = _as_contiguous_float64(low)
        close = _as_contiguous_float64(close)
        return _native._fast_stoch(
            high, low, close, fastk_period, slowk_period, slowd_period
        )

    stoch = _translate_native_errors("stoch", stoch)

elif "stoch" in globals():
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
