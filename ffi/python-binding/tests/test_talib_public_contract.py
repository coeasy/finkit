import numpy as np
import pytest

import finkit as ta


def sample(n=256):
    x = np.arange(n, dtype=np.float64)
    close = 100.0 + 0.03 * x + np.sin(x * 0.13)
    open_ = close - 0.15
    high = close + 0.8
    low = close - 0.9
    volume = 1_000_000.0 + x * 17.0
    return open_, high, low, close, volume


def assert_array(value, n):
    assert type(value) is np.ndarray
    assert value.dtype == np.float64
    assert value.shape == (n,)


def test_native_core_indicator_results_are_numpy_direct():
    _, high, low, close, volume = sample()
    n = close.size

    # These cover single-, double-, and triple-output binding shapes.
    assert_array(ta.sma(close, timeperiod=20), n)
    assert_array(ta.ema(close, timeperiod=20), n)
    assert_array(ta.rsi(close, timeperiod=14), n)
    assert_array(ta.atr(high, low, close, timeperiod=14), n)
    assert_array(ta.obv(close, volume), n)

    macd = ta.macd(close, fastperiod=12, slowperiod=26, signalperiod=9)
    assert isinstance(macd, tuple) and len(macd) == 3
    for value in macd:
        assert_array(value, n)


def test_sar_public_contract_matches_talib_shape():
    _, high, low, close, _ = sample()
    result = ta.sar(high, low, acceleration=0.02, maximum=0.2)
    assert_array(result, close.size)


def test_bbands_accepts_talib_matype_zero_and_rejects_unimplemented_types():
    _, _, _, close, _ = sample()
    result = ta.bollinger_bands(
        close,
        timeperiod=20,
        nbdevup=2.0,
        nbdevdn=2.0,
        matype=0,
    )
    assert isinstance(result, tuple) and len(result) == 3
    for value in result:
        assert_array(value, close.size)

    with pytest.raises(ValueError, match="matype=0"):
        ta.bollinger_bands(close, timeperiod=20, matype=1)


def test_stoch_accepts_talib_matype_keywords():
    _, high, low, close, _ = sample()
    result = ta.stoch(
        high,
        low,
        close,
        fastk_period=5,
        slowk_period=3,
        slowk_matype=0,
        slowd_period=3,
        slowd_matype=0,
    )
    assert isinstance(result, tuple) and len(result) == 2
    for value in result:
        assert_array(value, close.size)

    with pytest.raises(ValueError, match="slowk_matype=0"):
        ta.stoch(high, low, close, slowk_matype=1)


def test_stddev_and_correl_aliases_are_public_and_numpy_backed():
    _, high, low, close, _ = sample()
    assert callable(ta.stddev)
    assert callable(ta.correl)
    assert_array(ta.stddev(close, timeperiod=20, nbdev=1.0), close.size)
    assert_array(ta.correl(high, low, timeperiod=30), close.size)


def test_batch_compute_returns_ndarrays_without_python_list_boundary():
    open_, high, low, close, volume = sample()
    result = ta.compute_indicators(
        close,
        [("sma", [20.0]), ("macd", [12.0, 26.0, 9.0])],
        open=open_,
        high=high,
        low=low,
        volume=volume,
    )
    assert result
    for value in result.values():
        if not isinstance(value, str):
            assert type(value) is np.ndarray
