import numpy as np

import finkit


def test_hot_indicators_return_ndarray_directly():
    close = np.linspace(10.0, 30.0, 128, dtype=np.float64)
    volume = np.linspace(100.0, 500.0, 128, dtype=np.float64)
    high = close + 1.0
    low = close - 1.0

    sma = finkit.sma(close, 14)
    ema = finkit.ema(close, 14)
    obv = finkit.obv(close, volume)
    vwap = finkit.vwap(high, low, close, volume)

    assert isinstance(sma, np.ndarray)
    assert isinstance(ema, np.ndarray)
    assert isinstance(obv, np.ndarray)
    assert isinstance(vwap, np.ndarray)
    assert sma.dtype == np.float64
    assert ema.dtype == np.float64
    assert obv.dtype == np.float64
    assert vwap.dtype == np.float64
    assert len(sma) == len(close)
    assert len(vwap) == len(close)


def test_fast_sma_and_ema_preserve_existing_numerical_contract():
    close = np.arange(1.0, 65.0, dtype=np.float64)
    sma = finkit.sma(close, 5)
    ema = finkit.ema(close, 5)

    assert np.isnan(sma[:4]).all()
    assert np.isnan(ema[:4]).all()
    assert np.isclose(sma[4], 3.0)
    assert np.isclose(ema[4], 3.0)


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
