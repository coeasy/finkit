import numpy as np
import pytest

import finkit as ta


def sample(n=256):
    x = np.arange(n, dtype=np.float64)
    close = 100.0 + x * 0.05 + np.sin(x * 0.07)
    open_ = close - 0.2
    high = close + 1.0
    low = close - 1.0
    volume = 1000.0 + x * 3.0
    return open_, high, low, close, volume


def test_core_numeric_calls_return_ndarray_without_list_adapter():
    _, high, low, close, volume = sample()
    single = [
        ta.sma(close, timeperiod=20),
        ta.ema(close, timeperiod=20),
        ta.rsi(close, timeperiod=14),
        ta.atr(high, low, close, timeperiod=14),
        ta.obv(close, volume),
    ]
    assert all(isinstance(value, np.ndarray) for value in single)
    macd = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
    assert isinstance(macd, tuple) and len(macd) == 3
    assert all(isinstance(value, np.ndarray) for value in macd)


def test_documented_statistics_aliases_exist():
    _, high, low, close, _ = sample()
    assert isinstance(ta.stddev(close, timeperiod=20, nbdev=1.0), np.ndarray)
    assert isinstance(ta.correl(high, low, timeperiod=30), np.ndarray)


def test_bbands_accepts_talib_matype_zero_and_rejects_unsupported_types():
    _, _, _, close, _ = sample()
    upper, middle, lower = ta.bollinger_bands(
        close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0
    )
    assert all(isinstance(value, np.ndarray) for value in (upper, middle, lower))
    with pytest.raises(ta.InvalidParameterError):
        ta.bollinger_bands(close, timeperiod=20, matype=1)


def test_stoch_accepts_talib_matype_zero_contract():
    _, high, low, close, _ = sample()
    k, d = ta.stoch(
        high,
        low,
        close,
        fastk_period=5,
        slowk_period=3,
        slowk_matype=0,
        slowd_period=3,
        slowd_matype=0,
    )
    assert isinstance(k, np.ndarray)
    assert isinstance(d, np.ndarray)
    with pytest.raises(ta.InvalidParameterError):
        ta.stoch(high, low, close, slowk_matype=1)


def test_sar_public_contract_is_single_series_and_af_is_explicit():
    _, high, low, _, _ = sample()
    sar = ta.sar(high, low, acceleration=0.02, maximum=0.2)
    assert isinstance(sar, np.ndarray)
    sar2, af = ta.sar_with_af(high, low, acceleration=0.02, maximum=0.2)
    np.testing.assert_allclose(sar, sar2, equal_nan=True)
    assert isinstance(af, np.ndarray)


def test_compiled_formula_exposes_explicit_owned_alias():
    open_, high, low, close, volume = sample()
    plan = ta.CompiledFormula("MA(CLOSE,20)")
    owned = plan.eval_owned(open_, high, low, close, volume)["__result__"]
    regular = plan.eval(open_, high, low, close, volume)["__result__"]
    np.testing.assert_allclose(owned, regular, equal_nan=True)
