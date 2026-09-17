#!/usr/bin/env python3
"""Fair installed-package benchmark: Finkit v0.1.4 vs TA-Lib 0.7.1.

Measures end-user Python API latency on identical contiguous float64 NumPy inputs.
Both libraries allocate/return their normal public outputs.  Results are medians of
7 calibrated trials after warm-up, with alternating execution order to reduce bias.
"""

from __future__ import annotations

import gc
import importlib.metadata
import json
import math
import os
import platform
import statistics
import sys
import time
from pathlib import Path
from typing import Any, Callable

import numpy as np
import finkit
import talib

SIZES = (1_000, 10_000, 100_000, 1_000_000)
TRIALS = 7
WARMUPS = 3
TARGET_TRIAL_SECONDS = 0.035
MAX_LOOPS = 2_000


def pkg_version(name: str) -> str:
    try:
        return importlib.metadata.version(name)
    except importlib.metadata.PackageNotFoundError:
        return "unknown"


def make_data(n: int) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    t = np.arange(n, dtype=np.float64)
    close = 100.0 + t * 0.01 + np.sin(t * 0.37) * 2.0 + np.cos(t * 1.13) * 1.5
    open_ = close - 0.3
    high = close + 1.0 + np.abs(np.sin(t * 0.7)) * 0.5
    low = close - 1.0 - np.abs(np.cos(t * 0.5)) * 0.5
    volume = 10_000.0 + np.sin(t * 0.1) * 3_000.0 + np.abs(np.cos(t * 2.3)) * 2_000.0
    return tuple(np.ascontiguousarray(x, dtype=np.float64) for x in (open_, high, low, close, volume))  # type: ignore[return-value]


def outputs(value: Any) -> list[np.ndarray]:
    if isinstance(value, (tuple, list)):
        return [np.asarray(v, dtype=np.float64) for v in value]
    return [np.asarray(value, dtype=np.float64)]


def parity(a: Any, b: Any) -> dict[str, float | int | bool]:
    aa, bb = outputs(a), outputs(b)
    if len(aa) != len(bb):
        return {"ok": False, "max_abs": math.inf, "max_rel": math.inf, "compared": 0}
    max_abs = 0.0
    max_rel = 0.0
    compared = 0
    ok = True
    for x, y in zip(aa, bb):
        if x.shape != y.shape:
            ok = False
            continue
        mask = np.isfinite(x) & np.isfinite(y)
        if np.any(mask):
            dx = np.abs(x[mask] - y[mask])
            max_abs = max(max_abs, float(np.max(dx)))
            denom = np.maximum(np.abs(y[mask]), 1e-12)
            max_rel = max(max_rel, float(np.max(dx / denom)))
            compared += int(np.sum(mask))
        # Warm-up NaN locations should also align.
        if not np.array_equal(np.isfinite(x), np.isfinite(y)):
            ok = False
    return {"ok": ok, "max_abs": max_abs, "max_rel": max_rel, "compared": compared}


def one_call_ns(fn: Callable[[], Any]) -> int:
    t0 = time.perf_counter_ns()
    fn()
    return max(1, time.perf_counter_ns() - t0)


def run_loops(fn: Callable[[], Any], loops: int) -> float:
    last = None
    t0 = time.perf_counter_ns()
    for _ in range(loops):
        last = fn()
    elapsed = time.perf_counter_ns() - t0
    # Keep a strong reference through the timing boundary.
    if last is None:
        raise RuntimeError("benchmark function returned no result")
    return elapsed / loops / 1_000.0  # us/call


def measure(f_fn: Callable[[], Any], t_fn: Callable[[], Any]) -> tuple[float, float, int]:
    for _ in range(WARMUPS):
        f_fn()
        t_fn()
    probe = max(one_call_ns(f_fn), one_call_ns(t_fn)) / 1e9
    loops = max(1, min(MAX_LOOPS, int(TARGET_TRIAL_SECONDS / max(probe, 1e-9))))
    f_times: list[float] = []
    t_times: list[float] = []
    was_enabled = gc.isenabled()
    gc.disable()
    try:
        for i in range(TRIALS):
            if i % 2 == 0:
                f_times.append(run_loops(f_fn, loops))
                t_times.append(run_loops(t_fn, loops))
            else:
                t_times.append(run_loops(t_fn, loops))
                f_times.append(run_loops(f_fn, loops))
    finally:
        if was_enabled:
            gc.enable()
    return statistics.median(f_times), statistics.median(t_times), loops


def main() -> int:
    f_version = pkg_version("finkit")
    t_version = pkg_version("TA-Lib")
    ta_core = getattr(talib, "__ta_version__", "unknown")
    if isinstance(ta_core, bytes):
        ta_core = ta_core.decode("utf-8", "replace")

    print("=== environment ===")
    print("python:", sys.version.replace("\n", " "))
    print("platform:", platform.platform())
    print("machine:", platform.machine())
    print("cpu_count:", os.cpu_count())
    print("numpy:", np.__version__)
    print("finkit:", f_version)
    print("TA-Lib Python:", t_version)
    print("TA-Lib core:", ta_core)

    if f_version != "0.1.4":
        raise SystemExit(f"expected installed Finkit 0.1.4, got {f_version}")
    if t_version != "0.7.1":
        raise SystemExit(f"expected installed TA-Lib 0.7.1, got {t_version}")

    rows: list[dict[str, Any]] = []
    for n in SIZES:
        open_, high, low, close, volume = make_data(n)
        cases: list[tuple[str, Callable[[], Any], Callable[[], Any]]] = [
            ("SMA20", lambda c=close: finkit.sma(c, timeperiod=20), lambda c=close: talib.SMA(c, timeperiod=20)),
            ("EMA20", lambda c=close: finkit.ema(c, timeperiod=20), lambda c=close: talib.EMA(c, timeperiod=20)),
            ("RSI14", lambda c=close: finkit.rsi(c, timeperiod=14), lambda c=close: talib.RSI(c, timeperiod=14)),
            ("MACD", lambda c=close: finkit.macd(c, 12, 26, 9), lambda c=close: talib.MACD(c, fastperiod=12, slowperiod=26, signalperiod=9)),
            ("ATR14", lambda h=high, l=low, c=close: finkit.atr(h, l, c, timeperiod=14), lambda h=high, l=low, c=close: talib.ATR(h, l, c, timeperiod=14)),
            ("OBV", lambda c=close, v=volume: finkit.obv(c, v), lambda c=close, v=volume: talib.OBV(c, v)),
        ]
        for name, f_fn, t_fn in cases:
            f_out = f_fn()
            t_out = t_fn()
            p = parity(f_out, t_out)
            f_us, t_us, loops = measure(f_fn, t_fn)
            speedup = t_us / f_us
            row = {
                "indicator": name,
                "bars": n,
                "finkit_us": f_us,
                "talib_us": t_us,
                "speedup": speedup,
                "finkit_mbars_s": n / f_us,
                "talib_mbars_s": n / t_us,
                "loops_per_trial": loops,
                "parity": p,
            }
            rows.append(row)
            print(
                f"RESULT {name:5s} n={n:7d} finkit={f_us:10.2f}us "
                f"talib={t_us:10.2f}us speedup={speedup:6.3f}x "
                f"parity={p['ok']} max_abs={p['max_abs']:.3e} max_rel={p['max_rel']:.3e} loops={loops}"
            )

    speedups = [r["speedup"] for r in rows]
    geo = math.exp(sum(math.log(x) for x in speedups) / len(speedups))
    wins = sum(1 for x in speedups if x > 1.0)
    losses = len(speedups) - wins
    print("=== summary ===")
    print(f"cases={len(rows)} finkit_faster={wins} talib_faster_or_equal={losses} geometric_mean_speedup={geo:.4f}x")

    by_size: dict[int, float] = {}
    for n in SIZES:
        xs = [r["speedup"] for r in rows if r["bars"] == n]
        by_size[n] = math.exp(sum(math.log(x) for x in xs) / len(xs))
        print(f"size={n} geometric_mean_speedup={by_size[n]:.4f}x")

    out = {
        "environment": {
            "python": sys.version,
            "platform": platform.platform(),
            "machine": platform.machine(),
            "cpu_count": os.cpu_count(),
            "numpy": np.__version__,
            "finkit": f_version,
            "talib_python": t_version,
            "talib_core": str(ta_core),
            "trials": TRIALS,
            "warmups": WARMUPS,
        },
        "summary": {
            "cases": len(rows),
            "finkit_faster": wins,
            "talib_faster_or_equal": losses,
            "geometric_mean_speedup": geo,
            "geometric_mean_by_size": by_size,
        },
        "rows": rows,
    }
    out_dir = Path("dist/bench")
    out_dir.mkdir(parents=True, exist_ok=True)
    json_path = out_dir / "python-vs-talib-0.7.1.json"
    md_path = out_dir / "python-vs-talib-0.7.1.md"
    json_path.write_text(json.dumps(out, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    lines = [
        "# Finkit 0.1.4 vs TA-Lib 0.7.1 — installed Python packages",
        "",
        f"- Finkit faster in **{wins}/{len(rows)}** cases",
        f"- Geometric-mean speedup (TA-Lib/Finkit): **{geo:.3f}x**",
        "",
        "| Indicator | Bars | Finkit us | TA-Lib us | Speedup | Finkit Mbar/s | TA-Lib Mbar/s | Parity |",
        "|---|---:|---:|---:|---:|---:|---:|---|",
    ]
    for r in rows:
        p = r["parity"]
        lines.append(
            f"| {r['indicator']} | {r['bars']:,} | {r['finkit_us']:.2f} | {r['talib_us']:.2f} | "
            f"{r['speedup']:.3f}x | {r['finkit_mbars_s']:.2f} | {r['talib_mbars_s']:.2f} | "
            f"{'OK' if p['ok'] else 'DIFF'} (abs {p['max_abs']:.2e}) |"
        )
    md_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(md_path.read_text(encoding="utf-8"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
