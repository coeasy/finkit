#!/usr/bin/env python3
"""Current public-API coverage gate against TA-Lib.

Unlike the historical full-coverage benchmark, this file follows the current
Finkit Python signatures and records every exact public overlap with TA-Lib.
It is deliberately conservative: a missing function, signature mismatch, or
parity failure is a gate failure rather than a silently skipped observation.
"""

from __future__ import annotations

import argparse
import gc
import importlib.metadata
import json
import math
import platform
import statistics
import sys
import time
from pathlib import Path
from typing import Any, Callable

import numpy as np
import talib
import finkit


def data(n: int):
    x = np.arange(n, dtype=np.float64)
    close = np.ascontiguousarray(100.0 + 0.006 * x + np.sin(x * 0.017) * 2.1)
    open_ = np.ascontiguousarray(close - 0.15 + np.sin(x * 0.031) * 0.05)
    high = np.ascontiguousarray(np.maximum(open_, close) + 0.7 + np.abs(np.sin(x * 0.023)) * 0.3)
    low = np.ascontiguousarray(np.minimum(open_, close) - 0.8 - np.abs(np.cos(x * 0.019)) * 0.2)
    volume = np.ascontiguousarray(1_000_000.0 + (x % 997.0) * 170.0 + np.abs(np.sin(x * 0.011)) * 50_000.0)
    return open_, high, low, close, volume


def flatten(value: Any) -> list[np.ndarray]:
    if isinstance(value, tuple):
        result: list[np.ndarray] = []
        for item in value:
            result.extend(flatten(item))
        return result
    return [np.asarray(value)]


def talib_aroon_order(value: Any) -> tuple[Any, Any]:
    """TA-Lib returns (down, up); Finkit's native API returns (up, down)."""
    return value[1], value[0]


def parity(left: Any, right: Any) -> tuple[bool, bool, float, float]:
    aa = flatten(left)
    bb = flatten(right)
    if len(aa) != len(bb):
        return False, False, math.inf, math.inf
    mask_equal = True
    numerical = True
    max_abs = 0.0
    max_rel = 0.0
    for x, y in zip(aa, bb):
        if x.shape != y.shape:
            return False, False, math.inf, math.inf
        mx = np.isfinite(x)
        my = np.isfinite(y)
        mask_equal &= bool(np.array_equal(mx, my))
        mask = mx & my
        if np.any(mask):
            diff = np.abs(x[mask] - y[mask])
            max_abs = max(max_abs, float(np.max(diff)))
            denom = np.maximum(np.maximum(np.abs(x[mask]), np.abs(y[mask])), 1.0)
            max_rel = max(max_rel, float(np.max(diff / denom)))
            numerical &= bool(np.allclose(x[mask], y[mask], rtol=1e-8, atol=1e-8))
    return mask_equal and numerical, mask_equal, max_abs, max_rel


def loops_time(fn: Callable[[], Any], loops: int) -> float:
    last = None
    started = time.perf_counter_ns()
    for _ in range(loops):
        last = fn()
    elapsed = time.perf_counter_ns() - started
    if last is None:
        raise RuntimeError("benchmark returned None")
    return elapsed / loops / 1_000.0


def measure(left: Callable[[], Any], right: Callable[[], Any]) -> tuple[float, float, int]:
    for _ in range(2):
        left()
        right()
    probe = max(1, max(loops_time(left, 1), loops_time(right, 1))) / 1e6
    loops = max(1, min(200, int(0.025 / probe)))
    left_samples: list[float] = []
    right_samples: list[float] = []
    was_enabled = gc.isenabled()
    gc.disable()
    try:
        for i in range(5):
            if i % 2:
                right_samples.append(loops_time(right, loops))
                left_samples.append(loops_time(left, loops))
            else:
                left_samples.append(loops_time(left, loops))
                right_samples.append(loops_time(right, loops))
    finally:
        if was_enabled:
            gc.enable()
    return statistics.median(left_samples), statistics.median(right_samples), loops


def cases(o, h, l, c, v):
    # Keep names aligned with talib.get_functions().  The lambdas use explicit
    # keywords wherever the two packages have historically differed.
    return [
        ("AD", lambda: finkit.ad(h, l, c, v), lambda: talib.AD(h, l, c, v)),
        ("ADOSC", lambda: finkit.adosc(h, l, c, v, fastperiod=3, slowperiod=10), lambda: talib.ADOSC(h, l, c, v, fastperiod=3, slowperiod=10)),
        ("ADX", lambda: finkit.adx(h, l, c, timeperiod=14), lambda: talib.ADX(h, l, c, timeperiod=14)),
        ("APO", lambda: finkit.apo(c, fastperiod=12, slowperiod=26), lambda: talib.APO(c, fastperiod=12, slowperiod=26)),
        ("AROON", lambda: talib_aroon_order(finkit.aroon(h, l, timeperiod=14)), lambda: talib.AROON(h, l, timeperiod=14)),
        ("ATR", lambda: finkit.atr(h, l, c, timeperiod=14), lambda: talib.ATR(h, l, c, timeperiod=14)),
        ("AVGPRICE", lambda: finkit.avgprice(o, h, l, c), lambda: talib.AVGPRICE(o, h, l, c)),
        ("BETA", lambda: finkit.beta(c, o, timeperiod=5), lambda: talib.BETA(c, o, timeperiod=5)),
        ("BOP", lambda: finkit.bop(o, h, l, c), lambda: talib.BOP(o, h, l, c)),
        ("CCI", lambda: finkit.cci(h, l, c, timeperiod=14), lambda: talib.CCI(h, l, c, timeperiod=14)),
        ("CMO", lambda: finkit.cmo(c, timeperiod=14), lambda: talib.CMO(c, timeperiod=14)),
        ("CORREL", lambda: finkit.correl(h, l, timeperiod=30), lambda: talib.CORREL(h, l, timeperiod=30)),
        ("DEMA", lambda: finkit.dema(c, timeperiod=14), lambda: talib.DEMA(c, timeperiod=14)),
        ("DX", lambda: finkit.dx(h, l, c, timeperiod=14), lambda: talib.DX(h, l, c, timeperiod=14)),
        ("EMA", lambda: finkit.ema(c, timeperiod=14), lambda: talib.EMA(c, timeperiod=14)),
        ("HT_DCPERIOD", lambda: finkit.ht_dcperiod(c), lambda: talib.HT_DCPERIOD(c)),
        ("HT_DCPHASE", lambda: finkit.ht_dcphase(c), lambda: talib.HT_DCPHASE(c)),
        ("HT_PHASOR", lambda: finkit.ht_phasor(c), lambda: talib.HT_PHASOR(c)),
        ("HT_SINE", lambda: finkit.ht_sine(c), lambda: talib.HT_SINE(c)),
        ("HT_TRENDLINE", lambda: finkit.ht_trendline(c), lambda: talib.HT_TRENDLINE(c)),
        ("HT_TRENDMODE", lambda: finkit.ht_trendmode(c), lambda: talib.HT_TRENDMODE(c)),
        ("KAMA", lambda: finkit.kama(c, timeperiod=10), lambda: talib.KAMA(c, timeperiod=10)),
        ("MACD", lambda: finkit.macd(c, fastperiod=12, slowperiod=26, signalperiod=9), lambda: talib.MACD(c, fastperiod=12, slowperiod=26, signalperiod=9)),
        ("MAMA", lambda: finkit.mama(c, fastlimit=0.5, slowlimit=0.05), lambda: talib.MAMA(c, fastlimit=0.5, slowlimit=0.05)),
        ("MEDPRICE", lambda: finkit.medprice(h, l), lambda: talib.MEDPRICE(h, l)),
        ("MFI", lambda: finkit.mfi(h, l, c, v, timeperiod=14), lambda: talib.MFI(h, l, c, v, timeperiod=14)),
        ("MIDPOINT", lambda: finkit.midpoint(c, timeperiod=14), lambda: talib.MIDPOINT(c, timeperiod=14)),
        ("MIDPRICE", lambda: finkit.midprice(h, l, timeperiod=14), lambda: talib.MIDPRICE(h, l, timeperiod=14)),
        ("MINUS_DI", lambda: finkit.minus_di(h, l, c, timeperiod=14), lambda: talib.MINUS_DI(h, l, c, timeperiod=14)),
        ("MOM", lambda: finkit.mom(c, timeperiod=10), lambda: talib.MOM(c, timeperiod=10)),
        ("NATR", lambda: finkit.natr(h, l, c, timeperiod=14), lambda: talib.NATR(h, l, c, timeperiod=14)),
        ("OBV", lambda: finkit.obv(c, v), lambda: talib.OBV(c, v)),
        ("PLUS_DI", lambda: finkit.plus_di(h, l, c, timeperiod=14), lambda: talib.PLUS_DI(h, l, c, timeperiod=14)),
        ("ROC", lambda: finkit.roc(c, timeperiod=10), lambda: talib.ROC(c, timeperiod=10)),
        ("RSI", lambda: finkit.rsi(c, timeperiod=14), lambda: talib.RSI(c, timeperiod=14)),
        ("SAR", lambda: finkit.sar(h, l, acceleration=0.02, maximum=0.2), lambda: talib.SAR(h, l, acceleration=0.02, maximum=0.2)),
        ("SMA", lambda: finkit.sma(c, timeperiod=14), lambda: talib.SMA(c, timeperiod=14)),
        ("STDDEV", lambda: finkit.stddev(c, timeperiod=20, nbdev=1.0), lambda: talib.STDDEV(c, timeperiod=20, nbdev=1.0)),
        ("STOCH", lambda: finkit.stoch(h, l, c, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0), lambda: talib.STOCH(h, l, c, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0)),
        ("T3", lambda: finkit.t3(c, timeperiod=5, vfactor=0.7), lambda: talib.T3(c, timeperiod=5, vfactor=0.7)),
        ("TEMA", lambda: finkit.tema(c, timeperiod=14), lambda: talib.TEMA(c, timeperiod=14)),
        ("TRANGE", lambda: finkit.trange(h, l, c), lambda: talib.TRANGE(h, l, c)),
        ("TRIX", lambda: finkit.trix(c, timeperiod=14), lambda: talib.TRIX(c, timeperiod=14)),
        ("TSF", lambda: finkit.tsf(c, timeperiod=14), lambda: talib.TSF(c, timeperiod=14)),
        ("TYPPRICE", lambda: finkit.typprice(h, l, c), lambda: talib.TYPPRICE(h, l, c)),
        ("VAR", lambda: finkit.var(c, timeperiod=5, nbdev=1.0), lambda: talib.VAR(c, timeperiod=5, nbdev=1.0)),
        ("WCLPRICE", lambda: finkit.wclprice(h, l, c), lambda: talib.WCLPRICE(h, l, c)),
        ("WILLR", lambda: finkit.willr(h, l, c, timeperiod=14), lambda: talib.WILLR(h, l, c, timeperiod=14)),
        ("WMA", lambda: finkit.wma(c, timeperiod=14), lambda: talib.WMA(c, timeperiod=14)),
    ]


def run(args: argparse.Namespace) -> dict[str, Any]:
    env = {
        "python": sys.version,
        "platform": platform.platform(),
        "numpy": np.__version__,
        "finkit": importlib.metadata.version("finkit"),
        "talib_python": importlib.metadata.version("TA-Lib"),
    }
    rows: list[dict[str, Any]] = []
    errors: list[str] = []
    parity_failures: list[str] = []
    for n in args.sizes:
        o, h, l, c, v = data(n)
        for name, ff, tf in cases(o, h, l, c, v):
            try:
                f_value = ff()
                t_value = tf()
                ok, mask, abs_err, rel_err = parity(f_value, t_value)
                if not ok:
                    parity_failures.append(f"{name}:n={n}:mask={mask}:abs={abs_err:.3e}:rel={rel_err:.3e}")
                f_us, t_us, loops = measure(ff, tf)
                ratio = f_us / t_us
                row = {
                    "name": name,
                    "n": n,
                    "finkit_us": f_us,
                    "talib_us": t_us,
                    "finkit_speedup_x": 1.0 / ratio,
                    "talib_faster_x": ratio,
                    "parity": ok,
                    "mask_equal": mask,
                    "max_abs": abs_err,
                    "max_rel": rel_err,
                    "loops": loops,
                }
                rows.append(row)
                print(f"{name:14s} n={n:8d} f={f_us:10.2f}us ta={t_us:10.2f}us finkit={1.0 / ratio:7.2f}x parity={ok}")
            except Exception as exc:
                msg = f"{name}:n={n}:{type(exc).__name__}:{exc}"
                errors.append(msg)
                print("ERROR", msg)

    by_name: dict[str, list[float]] = {}
    for row in rows:
        by_name.setdefault(row["name"], []).append(row["finkit_speedup_x"])
    indicator_geomean = math.exp(statistics.mean(math.log(x) for x in (row["finkit_speedup_x"] for row in rows))) if rows else 0.0
    summary = {
        "environment": env,
        "sizes": args.sizes,
        "case_count": len(cases(*data(args.sizes[0]))) if args.sizes else 0,
        "observation_count": len(rows),
        "errors": errors,
        "parity_failures": parity_failures,
        "all_observations_faster": bool(rows) and all(row["finkit_speedup_x"] > 1.0 for row in rows),
        "all_cases_faster_at_every_size": bool(by_name) and all(all(value > 1.0 for value in values) for values in by_name.values()),
        "geomean_finkit_speedup": indicator_geomean,
        "rows": rows,
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps({key: summary[key] for key in ("case_count", "observation_count", "errors", "parity_failures", "all_observations_faster", "all_cases_faster_at_every_size", "geomean_finkit_speedup")}, indent=2, ensure_ascii=False))
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", nargs="+", type=int, default=[100_000])
    parser.add_argument("--output", default="dist/bench/talib-full-current-gate.json")
    args = parser.parse_args()
    summary = run(args)
    return 0 if not summary["errors"] and not summary["parity_failures"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
