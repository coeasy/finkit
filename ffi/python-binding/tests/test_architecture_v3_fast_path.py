import numpy as np
import pytest

import finkit


def test_hot_indicators_return_ndarray_directly():
    close = np.linspace(10.0, 30.0, 128, dtype=np.float64)
    volume = np.linspace(100.0, 500.0, 128, dtype=np.float64)
    high = close + 1.0
    low = close - 1.0

    sma = finkit.sma(close, 14)
    ema = finkit.ema(close, 14)
    wma = finkit.wma(close, 14)
    obv = finkit.obv(close, volume)
    vwap = finkit.vwap(high, low, close, volume)

    assert isinstance(sma, np.ndarray)
    assert isinstance(ema, np.ndarray)
    assert isinstance(wma, np.ndarray)
    assert isinstance(obv, np.ndarray)
    assert isinstance(vwap, np.ndarray)
    assert sma.dtype == np.float64
    assert ema.dtype == np.float64
    assert wma.dtype == np.float64
    assert obv.dtype == np.float64
    assert vwap.dtype == np.float64
    assert len(sma) == len(close)
    assert len(vwap) == len(close)


def test_architecture_v3_benchmark_surface_uses_direct_ndarrays():
    n = 256
    close = np.linspace(10.0, 30.0, n, dtype=np.float64)
    open_ = close - 0.1
    high = close + 1.0
    low = close - 1.0
    volume = np.linspace(100.0, 500.0, n, dtype=np.float64)

    single_outputs = [
        finkit.dema(close, 20),
        finkit.tema(close, 20),
        finkit.kama(close, 20),
        finkit.midpoint(close, 14),
        finkit.midprice(high, low, 14),
        finkit.rsi(close, 14),
        finkit.mom(close, 10),
        finkit.roc(close, 10),
        finkit.adx(high, low, close, 14),
        finkit.cci(high, low, close, 14),
        finkit.willr(high, low, close, 14),
        finkit.cmo(close, 14),
        finkit.mfi(high, low, close, volume, 14),
        finkit.plus_di(high, low, close, 14),
        finkit.minus_di(high, low, close, 14),
        finkit.ad(high, low, close, volume),
        finkit.adosc(high, low, close, volume, 3, 10),
        finkit.atr(high, low, close, 14),
        finkit.natr(high, low, close, 14),
        finkit.trange(high, low, close),
        finkit.stddev(close, 20, 1.0),
        finkit.var(close, 20, 1.0),
        finkit.correl(high, low, 30),
        finkit.bop(open_, high, low, close),
        finkit.sar(high, low, 0.02, 0.2),
    ]
    for result in single_outputs:
        assert isinstance(result, np.ndarray)
        assert result.dtype == np.float64
        assert result.shape == close.shape

    bbands = finkit.bollinger_bands(close, 20, 2.0, 2.0, 0)
    macd = finkit.macd(close, 12, 26, 9)
    stoch = finkit.stoch(high, low, close, 5, 3, 0, 3, 0)
    for result in (*bbands, *macd, *stoch):
        assert isinstance(result, np.ndarray)
        assert result.dtype == np.float64
        assert result.shape == close.shape


def test_fast_moving_averages_preserve_existing_numerical_contract():
    close = np.arange(1.0, 65.0, dtype=np.float64)
    sma = finkit.sma(close, 5)
    ema = finkit.ema(close, 5)
    wma = finkit.wma(close, 5)

    assert np.isnan(sma[:4]).all()
    assert np.isnan(ema[:4]).all()
    assert np.isnan(wma[:4]).all()
    assert np.isclose(sma[4], 3.0)
    assert np.isclose(ema[4], 3.0)
    assert np.isclose(wma[4], 55.0 / 15.0)


def test_float32_sma_and_ema_stay_float32():
    close = np.linspace(10.0, 30.0, 128, dtype=np.float32)
    sma = finkit.sma(close, 14)
    ema = finkit.ema(close, 14)

    assert sma.dtype == np.float32
    assert ema.dtype == np.float32
    assert np.isnan(sma[:13]).all()
    assert np.isnan(ema[:13]).all()
    assert np.isfinite(sma[13:]).all()
    assert np.isfinite(ema[13:]).all()


def test_moving_averages_reuse_caller_owned_output_buffers():
    close = np.arange(1.0, 65.0, dtype=np.float64)
    expected_sma = finkit.sma(close, 5)
    expected_ema = finkit.ema(close, 5)
    expected_wma = finkit.wma(close, 5)
    sma_out = np.empty_like(close)
    ema_out = np.empty_like(close)
    wma_out = np.empty_like(close)

    assert finkit.sma(close, 5, out=sma_out) is sma_out
    assert finkit.ema(close, 5, out=ema_out) is ema_out
    assert finkit.wma(close, 5, out=wma_out) is wma_out

    np.testing.assert_allclose(sma_out, expected_sma, equal_nan=True)
    np.testing.assert_allclose(ema_out, expected_ema, equal_nan=True)
    np.testing.assert_allclose(wma_out, expected_wma, equal_nan=True)


def test_volume_hot_paths_reuse_caller_owned_output_buffers():
    close = np.linspace(10.0, 30.0, 128, dtype=np.float64)
    volume = np.linspace(100.0, 500.0, 128, dtype=np.float64)
    high = close + 1.0
    low = close - 1.0

    expected_obv = finkit.obv(close, volume)
    expected_vwap = finkit.vwap(high, low, close, volume)
    obv_out = np.empty_like(close)
    vwap_out = np.empty_like(close)

    assert finkit.obv(close, volume, out=obv_out) is obv_out
    assert finkit.vwap(high, low, close, volume, out=vwap_out) is vwap_out
    np.testing.assert_allclose(obv_out, expected_obv, equal_nan=True)
    np.testing.assert_allclose(vwap_out, expected_vwap, equal_nan=True)

    with pytest.raises(finkit.InvalidParameterError, match="overlap"):
        finkit.obv(close, volume, out=volume)
    with pytest.raises(finkit.InvalidParameterError, match="overlap"):
        finkit.vwap(high, low, close, volume, out=low)


def test_reusable_output_preserves_float32_and_rejects_unsafe_buffers():
    close = np.linspace(10.0, 30.0, 128, dtype=np.float32)
    out = np.empty_like(close)
    result = finkit.ema(close, 14, out=out)
    assert result is out
    assert out.dtype == np.float32

    with pytest.raises(finkit.InvalidParameterError, match="dtype"):
        finkit.sma(close, 14, out=np.empty(close.shape, dtype=np.float64))

    with pytest.raises(finkit.InvalidParameterError, match="shape"):
        finkit.sma(close, 14, out=np.empty(close.size - 1, dtype=np.float32))

    backing = np.empty(close.size * 2, dtype=np.float32)
    non_contiguous = backing[::2]
    assert not non_contiguous.flags.c_contiguous
    with pytest.raises(finkit.InvalidParameterError, match="C-contiguous"):
        finkit.sma(close, 14, out=non_contiguous)

    with pytest.raises(finkit.InvalidParameterError, match="overlap"):
        finkit.sma(close, 14, out=close)


def test_scalar_reductions_preserve_float32_type():
    data = np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float32)

    assert isinstance(finkit.reduce_sum(data), np.float32)
    assert isinstance(finkit.reduce_mean(data), np.float32)
    assert isinstance(finkit.reduce_min(data), np.float32)
    assert isinstance(finkit.reduce_max(data), np.float32)
    assert isinstance(finkit.reduce_stddev(data), np.float32)
    assert finkit.reduce_sum(data) == np.float32(10.0)
    assert finkit.reduce_mean(data) == np.float32(2.5)


def test_scalar_reductions_use_float64_without_input_copy_requirement():
    data = np.array([1.0, 2.0, 3.0, 4.0], dtype=np.float64)

    assert isinstance(finkit.reduce_sum(data), float)
    assert isinstance(finkit.reduce_mean(data), float)
    assert finkit.reduce_min(data) == 1.0
    assert finkit.reduce_max(data) == 4.0
    assert np.isclose(finkit.reduce_stddev(data), np.std(data))


def test_non_contiguous_input_is_normalized_at_package_boundary():
    base = np.arange(20.0, dtype=np.float64)
    view = base[::2]
    assert not view.flags.c_contiguous

    result = finkit.sma(view, 3)
    assert isinstance(result, np.ndarray)
    assert len(result) == len(view)
    assert np.isnan(result[:2]).all()
