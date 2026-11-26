"""The one-line factor-library entry point, and the contract it must keep.

The M0-3 acceptance criterion is that a trial user can call
``finkit.factor_library("alpha158")`` and get the library. These tests hold the
Python surface to the same numbers the Rust side asserts, because a binding that
silently returned a different library size would look perfectly healthy from
Python while the two surfaces diverged.

Run with::

    python -m pytest ffi/python-binding/tests/test_factor_library.py
"""

import numpy as np
import pytest

import finkit

#: The shipped totals. Pinned in `core/src/factors.rs` too; if the two disagree
#: the binding is not exposing the same libraries the crate builds.
ALPHA158_FACTORS = 158
WORLDQUANT101_FACTORS = 17
SHIPPED_REGISTRY_FACTORS = 184


def _market(size: int = 128):
    """A synthetic market with *varying* volume.

    Volume has to move: several Alpha158 factors and `Alpha6` of the WorldQuant
    set read a correlation against volume, and a constant series has no variance,
    so the kernel correctly returns all-`NaN`. A flat fixture would report that
    correct answer as a broken factor.
    """
    index = np.arange(size, dtype=np.float64)
    close = 100.0 + np.sin(index * 0.11) * 4.0 + index * 0.02
    return {
        "open": close - 0.5,
        "high": close + 1.0,
        "low": close - 1.0,
        "close": close,
        "volume": 10_000.0 + np.cos(index * 0.31) * 800.0,
        "vwap": (close + 1.0 + close - 1.0 + close) / 3.0,
    }


def test_available_libraries_are_advertised():
    assert finkit.available_factor_libraries() == ["alpha158", "worldquant101"]


def test_alpha360_feature_set_has_stable_qlib_shape_and_column_order():
    feature_set = finkit.feature_set("alpha360")
    close = np.arange(1.0, 63.0) * 10.0
    market = {
        "open": close + 1.0,
        "high": close + 2.0,
        "low": close - 2.0,
        "close": close,
        "vwap": close + 0.5,
        "volume": np.arange(1.0, 63.0) * 100.0,
    }

    values = feature_set.evaluate(**market)

    assert finkit.available_feature_sets() == ["alpha158", "alpha360"]
    assert feature_set.name == "alpha360"
    assert len(feature_set) == 360
    assert len(feature_set.names()) == 360
    assert feature_set.names()[:3] == ["CLOSE59", "CLOSE58", "CLOSE57"]
    assert feature_set.names()[59:62] == ["CLOSE0", "OPEN59", "OPEN58"]
    assert feature_set.names()[-1] == "VOLUME0"
    assert values.shape == (len(close), 360)
    assert np.isnan(values[:59, 0]).all()
    assert values[59, 0] == pytest.approx(close[0] / close[59])
    assert values[61, 59] == 1.0
    assert values[61, 60] == pytest.approx(market["open"][2] / close[61])
    assert values[61, -1] == pytest.approx(1.0)


def test_alpha158_feature_set_returns_q_lib_ordered_matrix():
    feature_set = finkit.feature_set("alpha158")
    close = np.arange(1.0, 129.0)
    market = {
        "open": close - 0.5,
        "high": close + 1.0,
        "low": close - 1.0,
        "close": close,
        "vwap": close,
        "volume": close * 100.0,
    }

    values = feature_set.evaluate(**market)

    assert len(feature_set) == 158
    assert len(feature_set.names()) == 158
    assert feature_set.names()[:4] == ["KMID", "KLEN", "KMID2", "KUP"]
    assert values.shape == (len(close), 158)
    assert values[100, feature_set.names().index("OPEN0")] == pytest.approx(
        market["open"][100] / close[100]
    )


def test_alpha360_rejects_unknown_sets_and_misaligned_inputs():
    with pytest.raises(ValueError, match="alpha360"):
        finkit.feature_set("not-a-feature-set")

    features = finkit.feature_set("alpha360")
    aligned = np.arange(3.0)
    with pytest.raises(ValueError, match="open"):
        features.evaluate(
            open=aligned[:2],
            high=aligned,
            low=aligned,
            close=aligned,
            vwap=aligned,
            volume=aligned,
        )


def test_alpha360_empty_input_preserves_matrix_width():
    empty = np.array([], dtype=np.float64)
    matrix = finkit.feature_set("alpha360").evaluate(
        open=empty,
        high=empty,
        low=empty,
        close=empty,
        vwap=empty,
        volume=empty,
    )
    assert matrix.shape == (0, 360)


def test_one_line_lookup_returns_the_library():
    library = finkit.factor_library("alpha158")

    assert library.name == "alpha158"
    assert len(library) == ALPHA158_FACTORS
    assert "MA20" in library
    assert "not-a-factor" not in library
    assert len(library.names()) == ALPHA158_FACTORS
    # Sorted and unique, so `names()` can be zipped against a reference table.
    assert library.names() == sorted(set(library.names()))


def test_worldquant101_exposes_the_computable_subset():
    library = finkit.factor_library("worldquant101")

    assert len(library) == WORLDQUANT101_FACTORS
    # The published corpus numbers 101 alphas but defines 71; the 54 that need a
    # cross-section of instruments are annotated in the Rust module rather than
    # approximated here, so the exposed count is the computable subset.
    #
    # `Alpha101` is computable — it is `(close - open) / ((high - low) + 0.001)`,
    # entirely single-instrument — so the absent example has to be one that
    # genuinely needs the universe. `Alpha99` ranks a correlation across
    # instruments on one date, which is the blocking case.
    assert "Alpha6" in library
    assert "Alpha101" in library
    assert "Alpha99" not in library


def test_unknown_library_names_are_rejected():
    with pytest.raises(ValueError) as error:
        finkit.factor_library("no-such-library")
    assert "alpha158" in str(error.value)


def test_the_expression_is_exposed_for_audit():
    library = finkit.factor_library("alpha158")

    # The expression is the specification: it is what a reader checks against
    # Qlib's published definition. A binding that dropped it would make the
    # library unverifiable from Python.
    assert library.expression("MA20") == "MA(CLOSE, 20)/CLOSE"
    # ...and dependencies are lower-case, matching the `evaluate` keywords.
    assert library.dependencies("MA20") == ["close"]


def test_dependencies_are_passable_back_as_keywords():
    """Whatever `dependencies()` returns must be a valid `evaluate` keyword."""
    library = finkit.factor_library("alpha158")
    market = _market()
    keywords = set(finkit.factor_library_series_names())

    for name in library.names():
        for dependency in library.dependencies(name):
            assert dependency in keywords, (
                f"{name} declares `{dependency}`, which is not an `evaluate` keyword; "
                "a caller following `dependencies()` could not call `evaluate`"
            )


def test_evaluate_matches_a_hand_computed_moving_average():
    library = finkit.factor_library("alpha158")
    market = _market()

    values = library.evaluate("MA20", **market)

    assert isinstance(values, np.ndarray)
    assert values.dtype == np.float64
    assert values.shape == market["close"].shape

    # Alpha158's `MA` factor is `Mean($close, 20) / $close` — a ratio, not the
    # moving average itself. Asserting the ratio rather than the mean is what
    # pins the definition; a binding that returned `Mean(close, 20)` would look
    # like a plausible moving average and be wrong.
    #
    # finkit reports `NaN` until the window is full, so the first finite value is
    # at index 19 and it must equal the mean of the first 20 closes, divided by
    # the close at that bar.
    assert np.isnan(values[:19]).all()
    expected = market["close"][:20].mean() / market["close"][19]
    assert values[19] == pytest.approx(expected, rel=1e-12)


def test_missing_dependency_is_named_not_silently_nan():
    """A missing series must raise, not produce an all-NaN column.

    All-`NaN` is indistinguishable from a factor with no signal, so a caller
    would read a plausible result and never learn their input was wrong.
    """
    library = finkit.factor_library("alpha158")
    market = _market()
    del market["close"]

    with pytest.raises(ValueError) as error:
        library.evaluate("MA20", **market)
    assert "close" in str(error.value)


def test_ragged_series_are_rejected():
    library = finkit.factor_library("alpha158")

    with pytest.raises(ValueError):
        library.evaluate("MA20", close=np.arange(64.0), open=np.arange(32.0))


def test_evaluate_all_returns_every_factor_it_can_compute():
    library = finkit.factor_library("alpha158")
    market = _market()

    all_values = library.evaluate_all(**market)

    assert len(all_values) == ALPHA158_FACTORS
    assert all(isinstance(values, np.ndarray) for values in all_values.values())
    # Every factor here reads only OHLCV+vwap, so the sweep is complete.
    assert all(values.shape == market["close"].shape for values in all_values.values())


def test_evaluate_all_skips_factors_whose_inputs_are_absent():
    """A `close`-only sweep returns the `close`-only factors, not an error."""
    library = finkit.factor_library("alpha158")

    all_values = library.evaluate_all(close=_market()["close"])

    assert 0 < len(all_values) < ALPHA158_FACTORS
    assert "MA20" in all_values
    # The K-bar block reads OPEN/HIGH/LOW and cannot be computed from close alone.
    assert "KMID" not in all_values


def test_the_registry_is_the_union_of_the_libraries_and_the_demo_factors():
    names = finkit.factor_registry_names()

    assert len(names) == SHIPPED_REGISTRY_FACTORS
    assert len(names) == len(set(names)), "registry names must be unique"
    # One from each source: a demo factor, Alpha158, and WorldQuant.
    for name in ["momentum_5", "MA20", "Alpha6"]:
        assert name in names
