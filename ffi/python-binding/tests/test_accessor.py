"""Tests for pandas/polars DataFrame TA accessor (df.ta.*) and batch strategy runner.

These tests define the target API contract. They skip gracefully when alpha_ta or
the accessor extension is not yet installed/built.
"""

from __future__ import annotations

import numpy as np
import pytest

pytest.importorskip("pandas")
import pandas as pd


# ---------------------------------------------------------------------------
# Fixtures & helpers
# ---------------------------------------------------------------------------

def _import_alpha_ta():
    try:
        import alpha_ta as ta
        return ta
    except ImportError:
        pytest.skip("alpha_ta not installed — build with: maturin develop")


def _make_ohlcv_df(n: int = 100, seed: int = 42) -> pd.DataFrame:
    rng = np.random.default_rng(seed)
    close = np.cumsum(rng.standard_normal(n)) + 100.0
    return pd.DataFrame(
        {
            "open": close + rng.uniform(-0.5, 0.5, n),
            "high": close + rng.uniform(0.5, 2.0, n),
            "low": close - rng.uniform(0.5, 2.0, n),
            "close": close,
            "volume": rng.uniform(1000, 5000, n),
        }
    )


def _get_accessor(df: pd.DataFrame):
    """Return df.ta accessor or skip if not registered."""
    ta = _import_alpha_ta()

    # Allow explicit registration helper from the binding (future API).
    register = getattr(ta, "register_accessor", None)
    if register is not None:
        register()

    accessor = getattr(df, "ta", None)
    if accessor is None:
        pytest.skip("df.ta accessor not implemented yet")

    return ta, accessor


# ---------------------------------------------------------------------------
# df.ta.rsi(14) style single-indicator access
# ---------------------------------------------------------------------------

@pytest.fixture
def ohlcv_df() -> pd.DataFrame:
    return _make_ohlcv_df()


def test_ta_accessor_exists(ohlcv_df: pd.DataFrame):
    """DataFrame should expose a `.ta` namespace."""
    _, accessor = _get_accessor(ohlcv_df)
    assert accessor is not None


def test_ta_accessor_rsi(ohlcv_df: pd.DataFrame):
    """df.ta.rsi(14) should return RSI aligned with ta.rsi(close, 14)."""
    ta, accessor = _get_accessor(ohlcv_df)

    rsi_via_accessor = accessor.rsi(14)
    rsi_direct = ta.rsi(ohlcv_df["close"].values, timeperiod=14)

    if isinstance(rsi_via_accessor, pd.Series):
        rsi_values = rsi_via_accessor.values
    else:
        rsi_values = np.asarray(rsi_via_accessor)

    assert len(rsi_values) == len(ohlcv_df)
    np.testing.assert_allclose(rsi_values, rsi_direct, rtol=1e-10, equal_nan=True)


def test_ta_accessor_rsi_custom_column(ohlcv_df: pd.DataFrame):
    """Accessor should accept an explicit column name when needed."""
    ta, accessor = _get_accessor(ohlcv_df)

    rsi_fn = getattr(accessor, "rsi", None)
    if rsi_fn is None:
        pytest.skip("accessor.rsi not implemented")

    # column= keyword is optional in the target API
    try:
        rsi_via_accessor = accessor.rsi(14, column="close")
    except TypeError:
        rsi_via_accessor = accessor.rsi(14)

    rsi_direct = ta.rsi(ohlcv_df["close"].values, timeperiod=14)

    if isinstance(rsi_via_accessor, pd.Series):
        rsi_values = rsi_via_accessor.values
    else:
        rsi_values = np.asarray(rsi_via_accessor)

    np.testing.assert_allclose(rsi_values, rsi_direct, rtol=1e-10, equal_nan=True)


def test_ta_accessor_sma(ohlcv_df: pd.DataFrame):
    """df.ta.sma(20) should match the underlying ta.sma call."""
    ta, accessor = _get_accessor(ohlcv_df)

    sma_fn = getattr(accessor, "sma", None)
    if sma_fn is None:
        pytest.skip("accessor.sma not implemented")

    sma_via_accessor = sma_fn(20)
    sma_direct = ta.sma(ohlcv_df["close"].values, timeperiod=20)

    if isinstance(sma_via_accessor, pd.Series):
        sma_values = sma_via_accessor.values
    else:
        sma_values = np.asarray(sma_via_accessor)

    np.testing.assert_allclose(sma_values, sma_direct, rtol=1e-10, equal_nan=True)


# ---------------------------------------------------------------------------
# df.ta.strategy([...]) batch computation
# ---------------------------------------------------------------------------

STRATEGY_REQUESTS = [
    ("sma", [14]),
    ("ema", [14]),
    ("rsi", [14]),
]


def test_ta_strategy_batch(ohlcv_df: pd.DataFrame):
    """df.ta.strategy([...]) should batch-compute indicators and merge columns."""
    ta, accessor = _get_accessor(ohlcv_df)

    strategy_fn = getattr(accessor, "strategy", None)
    if strategy_fn is None:
        pytest.skip("accessor.strategy not implemented")

    result = strategy_fn(STRATEGY_REQUESTS)

    # Target: returns DataFrame with original OHLCV + indicator columns
    if isinstance(result, pd.DataFrame):
        assert len(result) == len(ohlcv_df)
        assert "close" in result.columns or "rsi_14" in result.columns
    elif isinstance(result, dict):
        assert "rsi_14" in result or any("rsi" in k for k in result)
    else:
        pytest.fail(f"strategy() should return DataFrame or dict, got {type(result)}")

    # Cross-check RSI against direct call when batch keys are exposed
    batch_rsi = None
    if isinstance(result, pd.DataFrame):
        for col in ("rsi_14", "RSI_14", "rsi"):
            if col in result.columns:
                batch_rsi = result[col].values
                break
    elif isinstance(result, dict):
        batch_rsi = result.get("rsi_14")

    if batch_rsi is not None:
        direct_rsi = ta.rsi(ohlcv_df["close"].values, timeperiod=14)
        np.testing.assert_allclose(
            np.asarray(batch_rsi), direct_rsi, rtol=1e-10, equal_nan=True
        )


def test_ta_strategy_matches_compute_indicators(ohlcv_df: pd.DataFrame):
    """strategy() results should be consistent with ta.compute_indicators()."""
    ta, accessor = _get_accessor(ohlcv_df)

    strategy_fn = getattr(accessor, "strategy", None)
    if strategy_fn is None:
        pytest.skip("accessor.strategy not implemented")

    compute_fn = getattr(ta, "compute_indicators", None)
    if compute_fn is None:
        pytest.skip("ta.compute_indicators not available")

    strategy_result = strategy_fn(STRATEGY_REQUESTS)
    batch_result = compute_fn(
        close=ohlcv_df["close"].values,
        requests=STRATEGY_REQUESTS,
    )

    def _extract_rsi(data):
        if isinstance(data, pd.DataFrame):
            for col in ("rsi_14", "RSI_14", "rsi"):
                if col in data.columns:
                    return np.asarray(data[col])
        if isinstance(data, dict) and "rsi_14" in data:
            return np.asarray(data["rsi_14"])
        return None

    strat_rsi = _extract_rsi(strategy_result)
    batch_rsi = batch_result.get("rsi_14")

    if strat_rsi is not None and batch_rsi is not None:
        np.testing.assert_allclose(strat_rsi, batch_rsi, rtol=1e-10, equal_nan=True)


def test_ta_strategy_ohlcv_indicators(ohlcv_df: pd.DataFrame):
    """Batch runner should pass OHLCV columns for indicators that need them."""
    _, accessor = _get_accessor(ohlcv_df)

    strategy_fn = getattr(accessor, "strategy", None)
    if strategy_fn is None:
        pytest.skip("accessor.strategy not implemented")

    requests = [("atr", [14]), ("mfi", [14])]
    result = strategy_fn(requests)

    if isinstance(result, pd.DataFrame):
        keys = set(result.columns)
    elif isinstance(result, dict):
        keys = set(result.keys())
    else:
        pytest.fail(f"Unexpected strategy() return type: {type(result)}")

    assert any("atr" in k.lower() for k in keys) or any("mfi" in k.lower() for k in keys)
