#!/usr/bin/env python3
"""Installed-wheel performance/parity gate against TA-Lib 0.7.1.

This benchmark intentionally measures the public Python package boundary.  It
catches regressions that native Rust Criterion runs cannot see (PyO3 container
materialisation, argument copies, and package wrappers).
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
        out: list[np.ndarray] = []
        for item in value:
            out.extend(flatten(item))
        return out
    return [np.asarray(value)]


def parity(a: Any, b: Any) -> tuple[bool, bool, float, float]:
    aa = flatten(a)
    bb = flatten(b)
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
        if not np.array_equal(mx, my):
            mask_equal = False
        mask = mx & my
        if np.any(mask):
            diff = np.abs(x[mask] - y[mask])
            max_abs = max(max_abs, float(np.max(diff)))
            denom = np.maximum(np.maximum(np.abs(x[mask]), np.abs(y[mask])), 1.0)
            max_rel = max(max_rel, float(np.max(diff / denom)))
            if not np.allclose(x[mask], y[mask], rtol=1e-8, atol=1e-8):
                numerical = False
    return mask_equal and numerical, mask_equal, max_abs, max_rel


def one_ns(fn: Callable[[], Any]) -> int:
    t0 = time.perf_counter_ns()
    value = fn()
    dt = time.perf_counter_ns() - t0
    if value is None:
        raise RuntimeError("benchmark returned None")
    return max(1, dt)


def loops_time(fn: Callable[[], Any], loops: int) -> float:
    last = None
    t0 = time.perf_counter_ns()
    for _ in range(loops):
        last = fn()
    elapsed = time.perf_counter_ns() - t0
    if last is None:
        raise RuntimeError("benchmark returned None")
    return elapsed / loops / 1_000.0


def measure(a: Callable[[], Any], b: Callable[[], Any]) -> tuple[float, float, int]:
    for _ in range(2):
        a(); b()
    probe = max(one_ns(a), one_ns(b)) / 1e9
    loops = max(1, min(200, int(0.025 / max(probe, 1e-9))))
    av: list[float] = []
    bv: list[float] = []
    enabled = gc.isenabled()
    gc.disable()
    try:
        for i in range(5):
            if i % 2:
                bv.append(loops_time(b, loops)); av.append(loops_time(a, loops))
            else:
                av.append(loops_time(a, loops)); bv.append(loops_time(b, loops))
    finally:
        if enabled:
            gc.enable()
    return statistics.median(av), statistics.median(bv), loops


def indicator_cases(o, h, l, c, v):
    return [
        ("SMA20", lambda: finkit.sma(c, timeperiod=20), lambda: talib.SMA(c, timeperiod=20)),
        ("EMA20", lambda: finkit.ema(c, timeperiod=20), lambda: talib.EMA(c, timeperiod=20)),
        ("WMA20", lambda: finkit.wma(c, timeperiod=20), lambda: talib.WMA(c, timeperiod=20)),
        ("DEMA20", lambda: finkit.dema(c, timeperiod=20), lambda: talib.DEMA(c, timeperiod=20)),
        ("TEMA20", lambda: finkit.tema(c, timeperiod=20), lambda: talib.TEMA(c, timeperiod=20)),
        ("KAMA20", lambda: finkit.kama(c, timeperiod=20), lambda: talib.KAMA(c, timeperiod=20)),
        ("BBANDS20", lambda: finkit.bollinger_bands(c, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0), lambda: talib.BBANDS(c, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0)),
        ("SAR", lambda: finkit.sar(h, l, acceleration=0.02, maximum=0.2), lambda: talib.SAR(h, l, acceleration=0.02, maximum=0.2)),
        ("MIDPOINT14", lambda: finkit.midpoint(c, timeperiod=14), lambda: talib.MIDPOINT(c, timeperiod=14)),
        ("MIDPRICE14", lambda: finkit.midprice(h, l, timeperiod=14), lambda: talib.MIDPRICE(h, l, timeperiod=14)),
        ("RSI14", lambda: finkit.rsi(c, timeperiod=14), lambda: talib.RSI(c, timeperiod=14)),
        ("MACD", lambda: finkit.macd(c, fastperiod=12, slowperiod=26, signalperiod=9), lambda: talib.MACD(c, fastperiod=12, slowperiod=26, signalperiod=9)),
        ("STOCH", lambda: finkit.stoch(h, l, c, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0), lambda: talib.STOCH(h, l, c, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0)),
        ("ADX14", lambda: finkit.adx(h, l, c, timeperiod=14), lambda: talib.ADX(h, l, c, timeperiod=14)),
        ("CCI14", lambda: finkit.cci(h, l, c, timeperiod=14), lambda: talib.CCI(h, l, c, timeperiod=14)),
        ("MOM10", lambda: finkit.mom(c, timeperiod=10), lambda: talib.MOM(c, timeperiod=10)),
        ("ROC10", lambda: finkit.roc(c, timeperiod=10), lambda: talib.ROC(c, timeperiod=10)),
        ("WILLR14", lambda: finkit.willr(h, l, c, timeperiod=14), lambda: talib.WILLR(h, l, c, timeperiod=14)),
        ("CMO14", lambda: finkit.cmo(c, timeperiod=14), lambda: talib.CMO(c, timeperiod=14)),
        ("MFI14", lambda: finkit.mfi(h, l, c, v, timeperiod=14), lambda: talib.MFI(h, l, c, v, timeperiod=14)),
        ("PLUS_DI14", lambda: finkit.plus_di(h, l, c, timeperiod=14), lambda: talib.PLUS_DI(h, l, c, timeperiod=14)),
        ("MINUS_DI14", lambda: finkit.minus_di(h, l, c, timeperiod=14), lambda: talib.MINUS_DI(h, l, c, timeperiod=14)),
        ("OBV", lambda: finkit.obv(c, v), lambda: talib.OBV(c, v)),
        ("AD", lambda: finkit.ad(h, l, c, v), lambda: talib.AD(h, l, c, v)),
        ("ADOSC", lambda: finkit.adosc(h, l, c, v, fastperiod=3, slowperiod=10), lambda: talib.ADOSC(h, l, c, v, fastperiod=3, slowperiod=10)),
        ("ATR14", lambda: finkit.atr(h, l, c, timeperiod=14), lambda: talib.ATR(h, l, c, timeperiod=14)),
        ("NATR14", lambda: finkit.natr(h, l, c, timeperiod=14), lambda: talib.NATR(h, l, c, timeperiod=14)),
        ("TRANGE", lambda: finkit.trange(h, l, c), lambda: talib.TRANGE(h, l, c)),
        ("STDDEV20", lambda: finkit.stddev(c, timeperiod=20, nbdev=1.0), lambda: talib.STDDEV(c, timeperiod=20, nbdev=1.0)),
        ("VAR20", lambda: finkit.var(c, timeperiod=20, nbdev=1.0), lambda: talib.VAR(c, timeperiod=20, nbdev=1.0)),
        ("CORREL30", lambda: finkit.correl(h, l, timeperiod=30), lambda: talib.CORREL(h, l, timeperiod=30)),
        ("BOP", lambda: finkit.bop(o, h, l, c), lambda: talib.BOP(o, h, l, c)),
    ]


def formula_cases(o, h, l, c, v):
    specs = [
        ("MA20", "MA(CLOSE,20)", lambda: talib.SMA(c, timeperiod=20)),
        ("EMA20", "EMA(CLOSE,20)", lambda: talib.EMA(c, timeperiod=20)),
        ("RSI14", "RSI(CLOSE,14)", lambda: talib.RSI(c, timeperiod=14)),
        ("ATR14", "ATR(HIGH,LOW,CLOSE,14)", lambda: talib.ATR(h, l, c, timeperiod=14)),
        ("ROC10", "ROC(CLOSE,10)", lambda: talib.ROC(c, timeperiod=10)),
        ("STD20", "STD(CLOSE,20)", lambda: talib.STDDEV(c, timeperiod=20, nbdev=1.0)),
        ("BOLL", "BOLL(CLOSE,20,2)", lambda: talib.BBANDS(c, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0)[0]),
    ]
    out = []
    for name, source, ref in specs:
        plan = finkit.CompiledFormula(source)
        out.append((name, lambda p=plan: p.eval_zero_copy(o, h, l, c, v)["__result__"], ref))
    return out


def run(args: argparse.Namespace) -> dict[str, Any]:
    env = {
        "python": sys.version,
        "platform": platform.platform(),
        "numpy": np.__version__,
        "finkit": importlib.metadata.version("finkit"),
        "talib_python": importlib.metadata.version("TA-Lib"),
        "talib_core": getattr(talib, "__ta_version__", b"unknown").decode(errors="replace") if isinstance(getattr(talib, "__ta_version__", b""), bytes) else str(getattr(talib, "__ta_version__", "unknown")),
    }
    print("ENV", json.dumps(env, ensure_ascii=False))
    rows: list[dict[str, Any]] = []
    errors: list[str] = []
    parity_failures: list[str] = []

    for n in args.sizes:
        o, h, l, c, v = data(n)
        for name, ff, tf in indicator_cases(o, h, l, c, v):
            try:
                fa = ff(); ta = tf()
                ok, mask, abs_err, rel_err = parity(fa, ta)
                if not ok:
                    parity_failures.append(f"indicator:{name}:n={n}:mask={mask}:abs={abs_err:.3e}:rel={rel_err:.3e}")
                f_us, t_us, loops = measure(ff, tf)
                ratio = f_us / t_us
                row = {"kind":"indicator","name":name,"n":n,"finkit_us":f_us,"talib_us":t_us,"talib_faster_x":ratio,"parity":ok,"mask_equal":mask,"max_abs":abs_err,"max_rel":rel_err,"loops":loops}
                rows.append(row)
                print(f"IND {name:12s} n={n:8d} f={f_us:10.2f}us ta={t_us:9.2f}us ta_faster={ratio:7.2f}x parity={ok}")
            except Exception as exc:
                msg = f"indicator:{name}:n={n}:{type(exc).__name__}:{exc}"
                errors.append(msg); print("ERROR", msg)

        for name, ff, tf in formula_cases(o, h, l, c, v):
            try:
                fa = ff(); ta = tf()
                ok, mask, abs_err, rel_err = parity(fa, ta)
                if not ok:
                    parity_failures.append(f"formula:{name}:n={n}:mask={mask}:abs={abs_err:.3e}:rel={rel_err:.3e}")
                f_us, t_us, loops = measure(ff, tf)
                ratio = f_us / t_us
                rows.append({"kind":"formula","name":name,"n":n,"finkit_us":f_us,"talib_us":t_us,"talib_faster_x":ratio,"parity":ok,"mask_equal":mask,"max_abs":abs_err,"max_rel":rel_err,"loops":loops})
                print(f"FORM {name:11s} n={n:8d} f={f_us:10.2f}us ref={t_us:9.2f}us ref_faster={ratio:7.2f}x parity={ok}")
            except Exception as exc:
                msg = f"formula:{name}:n={n}:{type(exc).__name__}:{exc}"
                errors.append(msg); print("ERROR", msg)

    ind_ratios = [r["talib_faster_x"] for r in rows if r["kind"] == "indicator"]
    form_ratios = [r["talib_faster_x"] for r in rows if r["kind"] == "formula"]
    ind_geo = math.exp(statistics.mean(math.log(x) for x in ind_ratios)) if ind_ratios else math.inf
    form_geo = math.exp(statistics.mean(math.log(x) for x in form_ratios)) if form_ratios else math.inf
    summary = {
        "environment": env,
        "indicator_observations": len(ind_ratios),
        "formula_observations": len(form_ratios),
        "indicator_geomean_talib_faster_x": ind_geo,
        "formula_geomean_reference_faster_x": form_geo,
        "max_indicator_talib_faster_x": max(ind_ratios, default=math.inf),
        "errors": errors,
        "parity_failures": parity_failures,
        "rows": rows,
    }
    print("SUMMARY", json.dumps({k:v for k,v in summary.items() if k != "rows"}, ensure_ascii=False))

    out = Path("dist/bench")
    out.mkdir(parents=True, exist_ok=True)
    (out / "talib-release-gate.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
    md = [
        "# Installed-wheel TA-Lib release gate",
        "",
        f"- indicator observations: **{len(ind_ratios)}**",
        f"- indicator geometric mean TA-Lib/Finkit latency ratio: **{ind_geo:.3f}x**",
        f"- formula observations: **{len(form_ratios)}**",
        f"- formula geometric mean reference/Finkit latency ratio: **{form_geo:.3f}x**",
        f"- API errors: **{len(errors)}**",
        f"- parity failures: **{len(parity_failures)}**",
    ]
    (out / "talib-release-gate.md").write_text("\n".join(md) + "\n", encoding="utf-8")

    if args.gate:
        failures = []
        if errors:
            failures.append(f"{len(errors)} API errors")
        if parity_failures:
            failures.append(f"{len(parity_failures)} parity failures")
        if ind_geo > args.max_indicator_geomean:
            failures.append(f"indicator geomean {ind_geo:.2f}x > {args.max_indicator_geomean:.2f}x")
        if form_geo > args.max_formula_geomean:
            failures.append(f"formula geomean {form_geo:.2f}x > {args.max_formula_geomean:.2f}x")
        if failures:
            raise SystemExit("release gate failed: " + "; ".join(failures))
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", type=int, nargs="+", default=[10_000, 100_000, 1_000_000])
    parser.add_argument("--gate", action="store_true")
    parser.add_argument("--max-indicator-geomean", type=float, default=5.0)
    parser.add_argument("--max-formula-geomean", type=float, default=3.0)
    args = parser.parse_args()
    run(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
