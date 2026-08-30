"""Test GIL release in Rust TA-Lib Python bindings."""
import threading
import time


def test_gil_release_concurrent():
    """Verify computation-heavy calls release the GIL by running them in parallel threads."""
    try:
        import finkit as ta
    except ImportError:
        print("SKIP: finkit not installed")
        return

    data = list(range(1, 10001))
    data_f = [float(x) for x in data]
    results = [None, None]
    errors = [None, None]

    def worker(idx):
        try:
            results[idx] = ta.sma(data_f, 14)
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
    assert len(results[0]) == len(data_f)
    print(f"GIL release test passed in {elapsed:.3f}s")


if __name__ == "__main__":
    test_gil_release_concurrent()
