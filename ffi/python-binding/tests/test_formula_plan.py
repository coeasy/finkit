"""Regression tests for the reusable formula plan API."""

import numpy as np
import pytest

import finkit


def _ohlcv(size: int = 64):
    close = np.linspace(10.0, 20.0, size, dtype=np.float64)
    open_ = close - 0.25
    high = close + 0.5
    low = close - 0.5
    volume = np.full(size, 1000.0, dtype=np.float64)
    return open_, high, low, close, volume


def test_compiled_formula_reuses_plan_and_returns_numpy_arrays():
    open_, high, low, close, volume = _ohlcv()
    plan = finkit.CompiledFormula("MA(CLOSE, 3)")

    assert plan.source == "MA(CLOSE, 3)"
    first = plan.eval(open_, high, low, close, volume)
    second = plan.eval(open_, high, low, close + 1.0, volume)

    assert isinstance(first["__result__"], np.ndarray)
    assert first["__result__"].shape == close.shape
    assert first["__result__"].dtype == np.float64
    assert not np.array_equal(first["__result__"], second["__result__"])


def test_compiled_formula_rejects_mismatched_lengths():
    open_, high, low, close, volume = _ohlcv()

    with pytest.raises(ValueError, match=r"expected"):
        finkit.CompiledFormula("MA(CLOSE, 3)").eval(
            open_[:-1], high, low, close, volume
        )


def test_indicator_package_api_returns_numpy_array():
    _, _, _, close, _ = _ohlcv()

    result = finkit.sma(close, timeperiod=3)

    assert isinstance(result, np.ndarray)
    assert result.shape == close.shape

def test_legacy_formula_api_also_returns_numpy_arrays():
    open_, high, low, close, volume = _ohlcv()

    result = finkit.formula_eval(
        "MA(CLOSE, 3)", open_, high, low, close, volume
    )

    assert isinstance(result["__result__"], np.ndarray)
    assert result["__result__"].shape == close.shape


def test_compiled_formula_zero_copy_rejects_non_contiguous_inputs():
    open_, high, low, close, volume = _ohlcv(32)
    plan = finkit.CompiledFormula("MA(CLOSE, 3)")
    with pytest.raises((TypeError, ValueError)):
        plan.eval_zero_copy(
            open_[::2], high[::2], low[::2], close[::2], volume[::2]
        )


def test_compiled_formula_eval_range_matches_full_result():
    open_, high, low, close, volume = _ohlcv(64)
    plan = finkit.CompiledFormula("MA(CLOSE, 5)")
    full = plan.eval(open_, high, low, close, volume)["__result__"]
    partial = plan.eval_range(open_, high, low, close, volume, 10, 32)["__result__"]
    np.testing.assert_allclose(partial, full[10:32], equal_nan=True)


def test_compiled_formula_append_bar_and_eval_last():
    open_, high, low, close, volume = _ohlcv(8)
    plan = finkit.CompiledFormula("MA(CLOSE, 3)")
    plan.eval(open_, high, low, close, volume)
    plan.reserve_bars(2)
    plan.append_bar(20.0, 21.0, 19.0, 21.0, 1200.0)
    plan.append_bar(21.0, 22.0, 20.0, 22.0, 1300.0)
    assert plan.eval_last() == pytest.approx(np.mean([close[-1], 21.0, 22.0]))


def test_legacy_zero_copy_api_uses_numpy_result_for_contiguous_arrays():
    open_, high, low, close, volume = _ohlcv(32)
    result = finkit.formula_eval_zero_copy(
        "MA(CLOSE, 3)", open_, high, low, close, volume
    )
    assert isinstance(result["__result__"], np.ndarray)
    np.testing.assert_allclose(
        result["__result__"], finkit.sma(close, timeperiod=3), equal_nan=True
    )


def test_compiled_formula_reset_discards_stream_context():
    open_, high, low, close, volume = _ohlcv(8)
    plan = finkit.CompiledFormula("MA(CLOSE, 3)")
    plan.eval(open_, high, low, close, volume)
    plan.append_bar(20.0, 21.0, 19.0, 21.0, 1200.0)
    plan.reset()
    with pytest.raises(ValueError, match=r"previous eval"):
        plan.eval_last()
