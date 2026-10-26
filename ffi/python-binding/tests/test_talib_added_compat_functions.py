"""Differential contracts for the direct TA-Lib extension wrappers."""

import numpy as np
import pytest
import talib

import finkit


def _market_data(size: int, seed: int):
    rng = np.random.default_rng(seed)
    close = 100.0 + np.cumsum(rng.normal(0.0, 0.8, size))
    open_ = close + rng.normal(0.0, 0.25, size)
    high = np.maximum(open_, close) + rng.uniform(0.0, 0.5, size)
    low = np.minimum(open_, close) - rng.uniform(0.0, 0.5, size)
    volume = rng.uniform(1.0, 1_000_000.0, size)
    return open_, high, low, close, volume


def _assert_talib_equal(actual, expected):
    if isinstance(actual, tuple):
        assert isinstance(expected, tuple)
        assert len(actual) == len(expected)
        for actual_part, expected_part in zip(actual, expected):
            _assert_talib_equal(actual_part, expected_part)
        return
    np.testing.assert_array_equal(np.isnan(actual), np.isnan(expected))
    np.testing.assert_allclose(actual, expected, rtol=1e-8, atol=1e-8, equal_nan=True)


def test_numpy_direct_metadata_skips_redundant_package_conversion():
    for name in (
        "add",
        "div",
        "vortex",
        "zscore",
        "heikin_ashi",
        "ha",
        "mult",
        "qstick",
        "sub",
        "max",
        "min",
        "minmax",
        "nvi",
        "pvi",
        "pvt",
        "marketfi",
        "wad",
    ):
        assert name in finkit._native_numpy_direct
        assert getattr(finkit, name).__wrapped__ is getattr(finkit._native, name)

    open_, high, low, close, _ = _market_data(32, 17)
    outputs = (
        finkit.vortex(high, low, close),
        finkit.zscore(close),
        finkit.heikin_ashi(open_, high, low, close),
        finkit.min(close, timeperiod=10),
        finkit.max(close, timeperiod=10),
        finkit.minmax(close, timeperiod=10),
    )
    for output in outputs:
        parts = output if isinstance(output, tuple) else (output,)
        assert all(isinstance(part, np.ndarray) for part in parts)


def test_all_talib_candlestick_wrappers_are_numpy_direct():
    candlesticks = [name.lower() for name in talib.get_functions() if name.startswith("CDL")]
    assert candlesticks
    for name in candlesticks:
        assert name in finkit._native_numpy_direct
        assert getattr(finkit, name).__wrapped__ is getattr(finkit._native, name)


def test_small_price_transforms_preserve_array_like_fallbacks():
    open_, high, low, close, _ = _market_data(128, 29)
    input_variants = (
        (open_, high, low, close),
        tuple(values.astype(np.float32) for values in (open_, high, low, close)),
        tuple(values[::-1] for values in (open_, high, low, close)),
        tuple(values.tolist() for values in (open_, high, low, close)),
    )
    for open_values, high_values, low_values, close_values in input_variants:
        normalized = [
            np.ascontiguousarray(values, dtype=np.float64)
            for values in (open_values, high_values, low_values, close_values)
        ]
        open_array, high_array, low_array, close_array = normalized
        np.testing.assert_allclose(
            finkit.avgprice(open_values, high_values, low_values, close_values),
            (open_array + high_array + low_array + close_array) / 4.0,
        )
        np.testing.assert_allclose(
            finkit.medprice(high_values, low_values),
            (high_array + low_array) / 2.0,
        )
        np.testing.assert_allclose(
            finkit.typprice(high_values, low_values, close_values),
            (high_array + low_array + close_array) / 3.0,
        )
        np.testing.assert_allclose(
            finkit.wclprice(high_values, low_values, close_values),
            (high_array + low_array + 2.0 * close_array) / 4.0,
        )


def test_added_unary_talib_compat_functions_match_reference():
    for size, seed in ((64, 1), (257, 2), (2048, 3)):
        open_, high, low, close, volume = _market_data(size, seed)
        cases = (
            (finkit.avgdev(close, timeperiod=14), talib.AVGDEV(close, timeperiod=14)),
            (finkit.dpo(close, timeperiod=20), talib.DPO(close, timeperiod=20)),
            (finkit.er(close, timeperiod=10), talib.ER(close, timeperiod=10)),
            (finkit.fosc(close, timeperiod=5), talib.FOSC(close, timeperiod=5)),
            (
                finkit.percentrank(close, timeperiod=30),
                talib.PERCENTRANK(close, timeperiod=30),
            ),
            (finkit.rma(close, timeperiod=30), talib.RMA(close, timeperiod=30)),
            (finkit.vhf(close, timeperiod=28), talib.VHF(close, timeperiod=28)),
            (finkit.hma(close, timeperiod=20), talib.HMA(close, timeperiod=20)),
            (finkit.zlema(close, timeperiod=30), talib.ZLEMA(close, timeperiod=30)),
            (
                finkit.rvi(close, timeperiod=14, stddevperiod=10),
                talib.RVI(close, timeperiod=14, stddevperiod=10),
            ),
            (finkit.adr(high, low, timeperiod=14), talib.ADR(high, low, timeperiod=14)),
            (
                finkit.ao(high, low, fastperiod=5, slowperiod=34),
                talib.AO(high, low, fastperiod=5, slowperiod=34),
            ),
            (finkit.cmou(close, timeperiod=14), talib.CMOU(close, timeperiod=14)),
            (
                finkit.cvi(high, low, timeperiod=10, rocperiod=10),
                talib.CVI(high, low, timeperiod=10, rocperiod=10),
            ),
            (
                finkit.efi(close, volume, timeperiod=13),
                talib.EFI(close, volume, timeperiod=13),
            ),
            (finkit.marketfi(high, low, volume), talib.MARKETFI(high, low, volume)),
            (
                finkit.massi(high, low, fastperiod=9, slowperiod=25),
                talib.MASSI(high, low, fastperiod=9, slowperiod=25),
            ),
            (
                finkit.minus_dm(high, low, timeperiod=14),
                talib.MINUS_DM(high, low, timeperiod=14),
            ),
            (finkit.nvi(close, volume), talib.NVI(close, volume)),
            (
                finkit.plus_dm(high, low, timeperiod=14),
                talib.PLUS_DM(high, low, timeperiod=14),
            ),
            (finkit.pvi(close, volume), talib.PVI(close, volume)),
            (finkit.pvt(close, volume), talib.PVT(close, volume)),
            (
                finkit.qstick(open_, close, timeperiod=10),
                talib.QSTICK(open_, close, timeperiod=10),
            ),
            (finkit.rvol(volume, timeperiod=20), talib.RVOL(volume, timeperiod=20)),
            (
                finkit.tsi(close, firstperiod=25, secondperiod=13),
                talib.TSI(close, firstperiod=25, secondperiod=13),
            ),
            (finkit.vwma(close, volume, timeperiod=30), talib.VWMA(close, volume, timeperiod=30)),
            (finkit.wad(high, low, close), talib.WAD(high, low, close)),
            (
                finkit.ac(high, low, fastperiod=5, slowperiod=34, signalperiod=5),
                talib.AC(high, low, fastperiod=5, slowperiod=34, signalperiod=5),
            ),
            (
                finkit.accbands(high, low, close, timeperiod=20),
                talib.ACCBANDS(high, low, close, timeperiod=20),
            ),
            (
                finkit.cmf(high, low, close, volume, timeperiod=20),
                talib.CMF(high, low, close, volume, timeperiod=20),
            ),
            (
                finkit.coppock(close, wmaperiod=10, roc1period=11, roc2period=14),
                talib.COPPOCK(close, wmaperiod=10, roc1period=11, roc2period=14),
            ),
            (finkit.cumsum(close), talib.CUMSUM(close)),
            (
                finkit.eri(high, low, close, timeperiod=13),
                talib.ERI(high, low, close, timeperiod=13),
            ),
            (
                finkit.fractal(high, low, leftbars=2, rightbars=2),
                talib.FRACTAL(high, low, leftbars=2, rightbars=2),
            ),
            (finkit.ha(open_, high, low, close), talib.HA(open_, high, low, close)),
            (finkit.imi(open_, close, timeperiod=14), talib.IMI(open_, close, timeperiod=14)),
            (
                finkit.kc(high, low, close, timeperiod=20, atrperiod=10, nbdev=2.0),
                talib.KC(high, low, close, timeperiod=20, atrperiod=10, nbdev=2.0),
            ),
            (
                finkit.kdj(
                    high,
                    low,
                    close,
                    fastk_period=9,
                    slowk_period=3,
                    slowk_matype=13,
                    slowd_period=3,
                    slowd_matype=13,
                ),
                talib.KDJ(
                    high,
                    low,
                    close,
                    fastk_period=9,
                    slowk_period=3,
                    slowk_matype=13,
                    slowd_period=3,
                    slowd_matype=13,
                ),
            ),
            (
                finkit.percentile(close, timeperiod=30, percentile=50.0),
                talib.PERCENTILE(close, timeperiod=30, percentile=50.0),
            ),
            (
                finkit.pvo(volume, fastperiod=12, slowperiod=26, matype=1),
                talib.PVO(volume, fastperiod=12, slowperiod=26, matype=1),
            ),
            (
                finkit.smi(
                    high,
                    low,
                    close,
                    timeperiod=13,
                    fastperiod=2,
                    slowperiod=25,
                    signalperiod=9,
                ),
                talib.SMI(
                    high,
                    low,
                    close,
                    timeperiod=13,
                    fastperiod=2,
                    slowperiod=25,
                    signalperiod=9,
                ),
            ),
        )
        for actual, expected in cases:
            _assert_talib_equal(actual, expected)


def test_hma_period_rounding_and_identity_match_talib():
    close = _market_data(512, 91)[3]
    for period in (1, 2, 3, 4, 5, 8, 14, 20, 30, 31, 100):
        _assert_talib_equal(
            finkit.hma(close, timeperiod=period),
            talib.HMA(close, timeperiod=period),
        )


def test_cmf_edge_windows_match_talib_contract():
    high = np.array([12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0] * 8)
    low = high - 2.0
    close = (high + low) / 2.0
    positive_volume = np.full_like(high, 100.0)

    cases = []
    flat_high = np.full_like(high, 10.0)
    cases.append((flat_high, flat_high, flat_high, positive_volume))
    cases.append((high, low, close, np.zeros_like(high)))

    missing_high = high.copy()
    missing_high[13] = np.nan
    cases.append((missing_high, low, close, positive_volume))

    negative_volume = -positive_volume
    cases.append((high, low, close, negative_volume))

    for high_case, low_case, close_case, volume_case in cases:
        _assert_talib_equal(
            finkit.cmf(high_case, low_case, close_case, volume_case, timeperiod=3),
            talib.CMF(high_case, low_case, close_case, volume_case, timeperiod=3),
        )


def test_percentile_rejects_period_below_talib_minimum():
    values = np.arange(8, dtype=np.float64)
    with pytest.raises(Exception):
        talib.PERCENTILE(values, timeperiod=1)
    with pytest.raises(Exception):
        finkit.percentile(values, timeperiod=1)


def test_cmou_rejects_period_below_talib_minimum():
    values = np.arange(8, dtype=np.float64)
    with pytest.raises(Exception):
        talib.CMOU(values, timeperiod=1)
    with pytest.raises(Exception):
        finkit.cmou(values, timeperiod=1)


def test_ac_fused_kernel_matches_talib_across_period_orderings():
    for size, seed in ((64, 11), (257, 12), (2048, 13)):
        _, high, low, _, _ = _market_data(size, seed)
        for fast, slow, signal in ((2, 5, 2), (5, 34, 5), (40, 10, 7)):
            actual = finkit.ac(
                high,
                low,
                fastperiod=fast,
                slowperiod=slow,
                signalperiod=signal,
            )
            expected = talib.AC(
                high,
                low,
                fastperiod=fast,
                slowperiod=slow,
                signalperiod=signal,
            )
            _assert_talib_equal(actual, expected)


def test_rvol_rolling_kernel_matches_talib_across_periods():
    for size, seed in ((128, 21), (257, 22), (2048, 23)):
        volume = _market_data(size, seed)[4]
        for period in (1, 2, 20, 127):
            actual = finkit.rvol(volume, timeperiod=period)
            expected = talib.RVOL(volume, timeperiod=period)
            _assert_talib_equal(actual, expected)
