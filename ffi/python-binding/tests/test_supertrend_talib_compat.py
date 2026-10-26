import numpy as np

import finkit


def test_supertrend_returns_numpy_arrays_without_list_roundtrip():
    close = np.ascontiguousarray(
        80.0 + np.arange(257, dtype=np.float64) * 0.03 + np.sin(np.arange(257) * 0.17)
    )
    high = np.ascontiguousarray(close + 0.8)
    low = np.ascontiguousarray(close - 0.6)

    native = finkit.supertrend(high, low, close, period=14, multiplier=3.0)
    compat = finkit.supertrend_talib(
        high, low, close, timeperiod=14, multiplier=3.0
    )

    assert len(native) == 4
    assert len(compat) == 2
    assert all(isinstance(output, np.ndarray) for output in native + compat)
    assert all(output.dtype == np.float64 for output in compat)
    np.testing.assert_array_equal(compat[0], native[1])
    np.testing.assert_array_equal(compat[1], native[0].astype(np.float64))
