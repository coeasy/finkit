"""Python binding coverage for configurable streaming MACDEXT."""

from __future__ import annotations

import math

import pytest


def test_streaming_macd_ext_default_and_batch():
    import finkit

    indicator = finkit.StreamingMACDEXT(3, 5, 3)
    values = [50.0 + math.sin(index * 0.2) for index in range(80)]
    outputs = indicator.update_batch(values)

    assert len(outputs) == len(values)
    assert indicator.count() == len(values)
    assert indicator.is_ready()
    assert math.isfinite(outputs[-1].macd)
    assert math.isfinite(outputs[-1].signal)
    assert math.isfinite(outputs[-1].histogram)


@pytest.mark.parametrize("fast_ma,slow_ma,signal_ma", [("wma", "ema", "dema"), ("hma", "t3", "trima"), ("vidya", "alma", "tema")])
def test_streaming_macd_ext_ma_variants(fast_ma: str, slow_ma: str, signal_ma: str):
    import finkit

    indicator = finkit.StreamingMACDEXT(
        10, 20, 5, fast_ma=fast_ma, slow_ma=slow_ma, signal_ma=signal_ma
    )
    for index in range(120):
        indicator.update(100.0 + math.cos(index * 0.13))

    assert indicator.is_ready()


def test_streaming_macd_ext_rejects_non_scalar_ma():
    import finkit

    with pytest.raises(ValueError, match="unsupported MACDEXT MA type"):
        finkit.StreamingMACDEXT(fast_ma="mama")
