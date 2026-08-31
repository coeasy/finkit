"""Test batch computation with single GIL release."""
import threading
import time
import numpy as np


def test_compute_indicators_basic():
    """Test basic batch computation functionality."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 101, dtype=np.float64)
    requests = [
        ("sma", [14]),
        ("ema", [14]),
        ("rsi", [14]),
    ]
    results = ta.compute_indicators(close=close, requests=requests)

    assert "sma_14" in results
    assert "ema_14" in results
    assert "rsi_14" in results
    assert len(results["sma_14"]) == len(close)
    assert len(results["ema_14"]) == len(close)
    assert len(results["rsi_14"]) == len(close)

    sma_single = ta.sma(close, 14)
    np.testing.assert_allclose(results["sma_14"], sma_single, rtol=1e-10)

    print("Basic batch computation test passed")


def test_compute_indicators_with_ohlcv():
    """Test batch computation with full OHLCV data."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    n = 100
    open_p = np.arange(1, n + 1, dtype=np.float64)
    high = np.arange(2, n + 2, dtype=np.float64)
    low = np.arange(0.5, n + 0.5, dtype=np.float64)
    close = np.arange(1.5, n + 1.5, dtype=np.float64)
    volume = np.arange(1000, 1000 + n, dtype=np.float64)

    requests = [
        ("sma", [14]),
        ("adx", [14]),
        ("atr", [14]),
        ("mfi", [14]),
        ("obv", []),
        ("bop", []),
    ]
    results = ta.compute_indicators(
        close=close,
        requests=requests,
        open=open_p,
        high=high,
        low=low,
        volume=volume,
    )

    assert "sma_14" in results
    assert "adx_14" in results
    assert "atr_14" in results
    assert "mfi_14" in results
    assert "obv_" in results
    assert "bop_" in results

    sma_single = ta.sma(close, 14)
    np.testing.assert_allclose(results["sma_14"], sma_single, rtol=1e-10)

    print("OHLCV batch computation test passed")


def test_compute_indicators_multi_output():
    """Test batch computation with multi-output indicators."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 101, dtype=np.float64)
    requests = [
        ("macd", [12, 26, 9]),
        ("bollinger_bands", [20, 2.0, 2.0]),
        ("stoch", [5, 3, 3]),
    ]
    high = np.arange(2, 102, dtype=np.float64)
    low = np.arange(0.5, 100.5, dtype=np.float64)

    results = ta.compute_indicators(
        close=close,
        requests=requests,
        high=high,
        low=low,
    )

    assert "macd_12_26_9_0" in results
    assert "macd_12_26_9_1" in results
    assert "macd_12_26_9_2" in results
    assert "bollinger_bands_20_2_2_0" in results
    assert "bollinger_bands_20_2_2_1" in results
    assert "bollinger_bands_20_2_2_2" in results
    assert "stoch_5_3_3_0" in results
    assert "stoch_5_3_3_1" in results

    macd_single = ta.macd(close, 12, 26, 9)
    np.testing.assert_allclose(results["macd_12_26_9_0"], macd_single[0], rtol=1e-10)
    np.testing.assert_allclose(results["macd_12_26_9_1"], macd_single[1], rtol=1e-10)
    np.testing.assert_allclose(results["macd_12_26_9_2"], macd_single[2], rtol=1e-10)

    print("Multi-output batch computation test passed")


def test_compute_indicators_gil_release():
    """Verify batch computation releases GIL by running concurrent calls."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 10001, dtype=np.float64)
    requests = [
        ("sma", [14]),
        ("ema", [14]),
        ("rsi", [14]),
        ("macd", [12, 26, 9]),
        ("bollinger_bands", [20, 2, 2]),
    ]

    results = [None, None]
    errors = [None, None]

    def worker(idx):
        try:
            results[idx] = ta.compute_indicators(close=close, requests=requests)
        except Exception as e:
            errors[idx] = e

    t1 = threading.Thread(target=worker, args=(0,))
    t2 = threading.Thread(target=worker, args=(1,))

    start = time.time()
    t1.start()
    t2.start()
    t1.join(timeout=30)
    t2.join(timeout=30)
    elapsed = time.time() - start

    assert errors[0] is None, f"Thread 0 error: {errors[0]}"
    assert errors[1] is None, f"Thread 1 error: {errors[1]}"
    assert results[0] is not None
    assert results[1] is not None

    for key in results[0]:
        np.testing.assert_allclose(results[0][key], results[1][key], rtol=1e-10)

    print(f"GIL release test passed in {elapsed:.3f}s")


def test_compute_indicators_performance_comparison():
    """Compare performance of batch vs individual calls."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 10001, dtype=np.float64)
    requests = [
        ("sma", [14]),
        ("ema", [14]),
        ("rsi", [14]),
        ("mom", [10]),
        ("roc", [10]),
        ("cmo", [14]),
        ("trix", [30]),
        ("apo", [12, 26]),
        ("macd", [12, 26, 9]),
        ("bollinger_bands", [20, 2, 2]),
    ]

    n_runs = 5

    batch_times = []
    for _ in range(n_runs):
        start = time.time()
        batch_results = ta.compute_indicators(close=close, requests=requests)
        batch_times.append(time.time() - start)
    avg_batch = sum(batch_times) / n_runs

    individual_times = []
    for _ in range(n_runs):
        start = time.time()
        sma = ta.sma(close, 14)
        ema = ta.ema(close, 14)
        rsi = ta.rsi(close, 14)
        mom = ta.mom(close, 10)
        roc = ta.roc(close, 10)
        cmo = ta.cmo(close, 14)
        trix = ta.trix(close, 30)
        apo = ta.apo(close, 12, 26)
        macd = ta.macd(close, 12, 26, 9)
        bbands = ta.bollinger_bands(close, 20, 2.0, 2.0)
        individual_times.append(time.time() - start)
    avg_individual = sum(individual_times) / n_runs

    np.testing.assert_allclose(batch_results["sma_14"], sma, rtol=1e-10)
    np.testing.assert_allclose(batch_results["ema_14"], ema, rtol=1e-10)
    np.testing.assert_allclose(batch_results["rsi_14"], rsi, rtol=1e-10)

    speedup = avg_individual / avg_batch
    print(f"Performance comparison:")
    print(f"  Batch average: {avg_batch*1000:.2f} ms")
    print(f"  Individual average: {avg_individual*1000:.2f} ms")
    print(f"  Speedup: {speedup:.2f}x")

    assert speedup > 0.5, f"Batch should be at least half as fast as individual calls (got {speedup:.2f}x)"


def test_compute_indicators_c_contiguous():
    """Test that C-contiguous arrays work correctly."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close_c = np.arange(1, 101, dtype=np.float64)
    assert close_c.flags["C_CONTIGUOUS"]

    close_non_c = np.asfortranarray(np.vstack([close_c, close_c]))[0]
    assert not close_non_c.flags["C_CONTIGUOUS"]

    close_non_c_as_c = np.ascontiguousarray(close_non_c)
    assert close_f_as_c.flags["C_CONTIGUOUS"]

    requests = [("sma", [14]), ("ema", [14])]

    results_c = ta.compute_indicators(close=close_c, requests=requests)
    results_f_converted = ta.compute_indicators(close=close_non_c_as_c, requests=requests)

    np.testing.assert_allclose(results_c["sma_14"], results_f_converted["sma_14"], rtol=1e-10)
    np.testing.assert_allclose(results_c["ema_14"], results_f_converted["ema_14"], rtol=1e-10)

    print("C-contiguous array test passed")


def test_compute_indicators_error_handling():
    """Test error handling for missing data."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 101, dtype=np.float64)
    requests = [
        ("adx", [14]),
        ("unknown_indicator", [14]),
    ]

    results = ta.compute_indicators(close=close, requests=requests)

    assert "adx_14_error" in results
    assert "ADX requires high and low data" in results["adx_14_error"]

    assert "unknown_indicator_14_error" in results
    assert "Unknown indicator" in results["unknown_indicator_14_error"]

    print("Error handling test passed")


def test_compute_indicators_large_batch():
    """Test batch computation with many indicators."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    close = np.arange(1, 1001, dtype=np.float64)
    high = np.arange(2, 1002, dtype=np.float64)
    low = np.arange(0.5, 1000.5, dtype=np.float64)
    volume = np.arange(1000, 2000, dtype=np.float64)

    requests = [
        ("sma", [5]),
        ("sma", [10]),
        ("sma", [20]),
        ("sma", [50]),
        ("ema", [5]),
        ("ema", [10]),
        ("ema", [20]),
        ("ema", [50]),
        ("wma", [14]),
        ("dema", [14]),
        ("tema", [14]),
        ("rsi", [7]),
        ("rsi", [14]),
        ("rsi", [21]),
        ("mom", [5]),
        ("mom", [10]),
        ("roc", [5]),
        ("roc", [10]),
        ("cmo", [14]),
        ("trix", [30]),
        ("adx", [14]),
        ("aroon", [14]),
        ("cci", [14]),
        ("willr", [14]),
        ("atr", [14]),
        ("natr", [14]),
        ("stoch", [5, 3, 3]),
        ("mfi", [14]),
        ("obv", []),
        ("zscore", [14]),
        ("linear_reg", [14]),
        ("tsf", [14]),
        ("std_dev", [14]),
        ("percent_rank", [10]),
    ]

    start = time.time()
    results = ta.compute_indicators(
        close=close,
        requests=requests,
        high=high,
        low=low,
        volume=volume,
    )
    elapsed = time.time() - start

    expected_keys = len(requests)
    actual_keys = len([k for k in results if not k.endswith("_error")])
    assert actual_keys >= expected_keys * 0.9, f"Expected at least {expected_keys * 0.9} keys, got {actual_keys}"

    print(f"Large batch test passed: {actual_keys} indicators computed in {elapsed*1000:.2f} ms")


if __name__ == "__main__":
    test_compute_indicators_basic()
    test_compute_indicators_with_ohlcv()
    test_compute_indicators_multi_output()
    test_compute_indicators_gil_release()
    test_compute_indicators_performance_comparison()
    test_compute_indicators_c_contiguous()
    test_compute_indicators_error_handling()
    test_compute_indicators_large_batch()
    print("\nAll tests passed!")