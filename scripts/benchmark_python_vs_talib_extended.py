#!/usr/bin/env python3
"""Extended installed-package benchmark: Finkit 0.1.4 vs TA-Lib 0.7.1.

Scope:
- broad public Python indicator API comparison across 10K/100K/1M bars;
- compiled formula engine comparison using eval() and eval_zero_copy();
- precision/alignment diagnostics on identical contiguous float64 inputs.

The benchmark intentionally measures what Python users actually pay for, including
binding conversion and public result materialisation.
"""

from __future__ import annotations

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
import finkit
import talib

SIZES = (10_000, 100_000, 1_000_000)
TRIALS = 5
WARMUPS = 2
TARGET_TRIAL_SECONDS = 0.025
MAX_LOOPS = 500


def pkg_version(name: str) -> str:
    return importlib.metadata.version(name)


def make_data(n: int) -> tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
    t = np.arange(n, dtype=np.float64)
    trend = t * 0.01
    cycle = np.sin(t * 0.037) * 2.0 + np.cos(t * 0.113) * 1.5 + np.sin(t * 0.371) * 0.8
    close = 100.0 + trend + cycle
    open_ = close - 0.25 + np.sin(t * 0.017) * 0.15
    high = np.maximum(open_, close) + 0.8 + np.abs(np.sin(t * 0.071)) * 0.5
    low = np.minimum(open_, close) - 0.8 - np.abs(np.cos(t * 0.053)) * 0.5
    volume = 100_000.0 + np.sin(t * 0.021) * 20_000.0 + np.abs(np.cos(t * 0.127)) * 15_000.0
    return tuple(np.ascontiguousarray(x, dtype=np.float64) for x in (open_, high, low, close, volume))  # type: ignore[return-value]


def flatten_outputs(value: Any) -> list[np.ndarray]:
    if isinstance(value, dict):
        if "__result__" in value:
            return [np.asarray(value["__result__"], dtype=np.float64)]
        return [np.asarray(v, dtype=np.float64) for _, v in sorted(value.items())]
    if isinstance(value, (tuple, list)):
        return [np.asarray(v, dtype=np.float64) for v in value]
    return [np.asarray(value, dtype=np.float64)]


def parity(a: Any, b: Any) -> dict[str, Any]:
    aa, bb = flatten_outputs(a), flatten_outputs(b)
    if len(aa) != len(bb):
        return {"same_shape": False, "finite_mask_equal": False, "max_abs": math.inf, "max_rel": math.inf, "compared": 0}
    same_shape = True
    mask_equal = True
    max_abs = 0.0
    max_rel = 0.0
    compared = 0
    for x, y in zip(aa, bb):
        if x.shape != y.shape:
            same_shape = False
            continue
        fx = np.isfinite(x)
        fy = np.isfinite(y)
        if not np.array_equal(fx, fy):
            mask_equal = False
        mask = fx & fy
        if np.any(mask):
            d = np.abs(x[mask] - y[mask])
            max_abs = max(max_abs, float(np.max(d)))
            max_rel = max(max_rel, float(np.max(d / np.maximum(np.abs(y[mask]), 1e-12))))
            compared += int(np.sum(mask))
    return {
        "same_shape": same_shape,
        "finite_mask_equal": mask_equal,
        "max_abs": max_abs,
        "max_rel": max_rel,
        "compared": compared,
    }


def one_call_ns(fn: Callable[[], Any]) -> int:
    t0 = time.perf_counter_ns()
    out = fn()
    dt = time.perf_counter_ns() - t0
    if out is None:
        raise RuntimeError("benchmark returned None")
    return max(1, dt)


def run_loops(fn: Callable[[], Any], loops: int) -> float:
    last = None
    t0 = time.perf_counter_ns()
    for _ in range(loops):
        last = fn()
    elapsed = time.perf_counter_ns() - t0
    if last is None:
        raise RuntimeError("benchmark returned None")
    return elapsed / loops / 1_000.0


def measure(a_fn: Callable[[], Any], b_fn: Callable[[], Any]) -> tuple[float, float, int]:
    for _ in range(WARMUPS):
        a_fn(); b_fn()
    probe = max(one_call_ns(a_fn), one_call_ns(b_fn)) / 1e9
    loops = max(1, min(MAX_LOOPS, int(TARGET_TRIAL_SECONDS / max(probe, 1e-9))))
    aa: list[float] = []
    bb: list[float] = []
    enabled = gc.isenabled()
    gc.disable()
    try:
        for i in range(TRIALS):
            if i % 2 == 0:
                aa.append(run_loops(a_fn, loops)); bb.append(run_loops(b_fn, loops))
            else:
                bb.append(run_loops(b_fn, loops)); aa.append(run_loops(a_fn, loops))
    finally:
        if enabled:
            gc.enable()
    return statistics.median(aa), statistics.median(bb), loops


def indicator_cases(open_: np.ndarray, high: np.ndarray, low: np.ndarray, close: np.ndarray, volume: np.ndarray):
    return [
        ("SMA20", "overlap", lambda: finkit.sma(close, timeperiod=20), lambda: talib.SMA(close, timeperiod=20)),
        ("EMA20", "overlap", lambda: finkit.ema(close, timeperiod=20), lambda: talib.EMA(close, timeperiod=20)),
        ("WMA20", "overlap", lambda: finkit.wma(close, timeperiod=20), lambda: talib.WMA(close, timeperiod=20)),
        ("DEMA20", "overlap", lambda: finkit.dema(close, timeperiod=20), lambda: talib.DEMA(close, timeperiod=20)),
        ("TEMA20", "overlap", lambda: finkit.tema(close, timeperiod=20), lambda: talib.TEMA(close, timeperiod=20)),
        ("KAMA20", "overlap", lambda: finkit.kama(close, timeperiod=20), lambda: talib.KAMA(close, timeperiod=20)),
        ("BBANDS20", "overlap", lambda: finkit.bollinger_bands(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0), lambda: talib.BBANDS(close, timeperiod=20, nbdevup=2.0, nbdevdn=2.0, matype=0)),
        ("SAR", "overlap", lambda: finkit.sar(high, low, acceleration=0.02, maximum=0.2), lambda: talib.SAR(high, low, acceleration=0.02, maximum=0.2)),
        ("MIDPOINT14", "overlap", lambda: finkit.midpoint(close, timeperiod=14), lambda: talib.MIDPOINT(close, timeperiod=14)),
        ("MIDPRICE14", "overlap", lambda: finkit.midprice(high, low, timeperiod=14), lambda: talib.MIDPRICE(high, low, timeperiod=14)),
        ("RSI14", "momentum", lambda: finkit.rsi(close, timeperiod=14), lambda: talib.RSI(close, timeperiod=14)),
        ("MACD", "momentum", lambda: finkit.macd(close, fastperiod=12, slowperiod=26, signalperiod=9), lambda: talib.MACD(close, fastperiod=12, slowperiod=26, signalperiod=9)),
        ("STOCH", "momentum", lambda: finkit.stoch(high, low, close, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0), lambda: talib.STOCH(high, low, close, fastk_period=5, slowk_period=3, slowk_matype=0, slowd_period=3, slowd_matype=0)),
        ("ADX14", "momentum", lambda: finkit.adx(high, low, close, timeperiod=14), lambda: talib.ADX(high, low, close, timeperiod=14)),
        ("CCI14", "momentum", lambda: finkit.cci(high, low, close, timeperiod=14), lambda: talib.CCI(high, low, close, timeperiod=14)),
        ("MOM10", "momentum", lambda: finkit.mom(close, timeperiod=10), lambda: talib.MOM(close, timeperiod=10)),
        ("ROC10", "momentum", lambda: finkit.roc(close, timeperiod=10), lambda: talib.ROC(close, timeperiod=10)),
        ("WILLR14", "momentum", lambda: finkit.willr(high, low, close, timeperiod=14), lambda: talib.WILLR(high, low, close, timeperiod=14)),
        ("CMO14", "momentum", lambda: finkit.cmo(close, timeperiod=14), lambda: talib.CMO(close, timeperiod=14)),
        ("MFI14", "momentum", lambda: finkit.mfi(high, low, close, volume, timeperiod=14), lambda: talib.MFI(high, low, close, volume, timeperiod=14)),
        ("PLUS_DI14", "momentum", lambda: finkit.plus_di(high, low, close, timeperiod=14), lambda: talib.PLUS_DI(high, low, close, timeperiod=14)),
        ("MINUS_DI14", "momentum", lambda: finkit.minus_di(high, low, close, timeperiod=14), lambda: talib.MINUS_DI(high, low, close, timeperiod=14)),
        ("OBV", "volume", lambda: finkit.obv(close, volume), lambda: talib.OBV(close, volume)),
        ("AD", "volume", lambda: finkit.ad(high, low, close, volume), lambda: talib.AD(high, low, close, volume)),
        ("ADOSC", "volume", lambda: finkit.adosc(high, low, close, volume, fastperiod=3, slowperiod=10), lambda: talib.ADOSC(high, low, close, volume, fastperiod=3, slowperiod=10)),
        ("ATR14", "volatility", lambda: finkit.atr(high, low, close, timeperiod=14), lambda: talib.ATR(high, low, close, timeperiod=14)),
        ("NATR14", "volatility", lambda: finkit.natr(high, low, close, timeperiod=14), lambda: talib.NATR(high, low, close, timeperiod=14)),
        ("TRANGE", "volatility", lambda: finkit.trange(high, low, close), lambda: talib.TRANGE(high, low, close)),
        ("STDDEV20", "statistics", lambda: finkit.stddev(close, timeperiod=20, nbdev=1.0), lambda: talib.STDDEV(close, timeperiod=20, nbdev=1.0)),
        ("VAR20", "statistics", lambda: finkit.var(close, timeperiod=20, nbdev=1.0), lambda: talib.VAR(close, timeperiod=20, nbdev=1.0)),
        ("CORREL30", "statistics", lambda: finkit.correl(high, low, timeperiod=30), lambda: talib.CORREL(high, low, timeperiod=30)),
        ("BOP", "price-action", lambda: finkit.bop(open_, high, low, close), lambda: talib.BOP(open_, high, low, close)),
    ]


def cross_ref(a: np.ndarray, b: np.ndarray) -> np.ndarray:
    out = np.zeros(a.size, dtype=np.float64)
    valid = np.isfinite(a) & np.isfinite(b)
    prev = np.zeros(a.size, dtype=bool)
    prev[1:] = valid[:-1] & (a[:-1] <= b[:-1])
    curr = valid & (a > b)
    out[curr & prev] = 1.0
    return out


def ref1(x: np.ndarray) -> np.ndarray:
    out = np.empty_like(x)
    out[0] = np.nan
    out[1:] = x[:-1]
    return out


def formula_cases(open_: np.ndarray, high: np.ndarray, low: np.ndarray, close: np.ndarray, volume: np.ndarray):
    specs: list[tuple[str, str, Callable[[], Any]]] = [
        ("MA20", "MA(CLOSE, 20)", lambda: talib.SMA(close, timeperiod=20)),
        ("EMA20", "EMA(CLOSE, 20)", lambda: talib.EMA(close, timeperiod=20)),
        ("RSI14", "RSI(CLOSE, 14)", lambda: talib.RSI(close, timeperiod=14)),
        ("ATR14", "ATR(HIGH, LOW, CLOSE, 14)", lambda: talib.ATR(high, low, close, timeperiod=14)),
        ("ROC10", "ROC(CLOSE, 10)", lambda: talib.ROC(close, timeperiod=10)),
        ("BOLL_UPPER", "MA(CLOSE,20) + 2 * STD(CLOSE,20)", lambda: talib.SMA(close, timeperiod=20) + 2.0 * talib.STDDEV(close, timeperiod=20, nbdev=1.0)),
        ("MA_CROSS", "CROSS(MA(CLOSE,5), MA(CLOSE,20))", lambda: cross_ref(talib.SMA(close, timeperiod=5), talib.SMA(close, timeperiod=20))),
        ("REF1", "REF(CLOSE, 1)", lambda: ref1(close)),
    ]
    rows = []
    for name, expr, ref_fn in specs:
        try:
            plan_eval = finkit.CompiledFormula(expr)
            plan_zero = finkit.CompiledFormula(expr)
            eval_fn = lambda p=plan_eval: p.eval(open_, high, low, close, volume)
            zero_fn = lambda p=plan_zero: p.eval_zero_copy(open_, high, low, close, volume)
            rows.append((name, expr, "eval", eval_fn, ref_fn))
            rows.append((name, expr, "eval_zero_copy", zero_fn, ref_fn))
        except Exception as exc:
            rows.append((name, expr, "compile_error", lambda e=exc: (_ for _ in ()).throw(e), ref_fn))
    return rows


def main() -> int:
    env = {
        "python": sys.version,
        "platform": platform.platform(),
        "numpy": np.__version__,
        "finkit": pkg_version("finkit"),
        "talib_python": pkg_version("TA-Lib"),
        "talib_core": getattr(talib, "__ta_version__", b"unknown").decode() if isinstance(getattr(talib, "__ta_version__", b"unknown"), bytes) else str(getattr(talib, "__ta_version__", "unknown")),
        "trials": TRIALS,
        "warmups": WARMUPS,
    }
    print("ENV", json.dumps(env, ensure_ascii=False))
    if env["finkit"] != "0.1.4" or env["talib_python"] != "0.7.1":
        raise SystemExit(f"unexpected versions: {env}")

    indicator_rows: list[dict[str, Any]] = []
    formula_rows: list[dict[str, Any]] = []

    for n in SIZES:
        open_, high, low, close, volume = make_data(n)
        print(f"=== indicators n={n} ===")
        for name, category, f_fn, t_fn in indicator_cases(open_, high, low, close, volume):
            row: dict[str, Any] = {"indicator": name, "category": category, "bars": n}
            try:
                fo, to = f_fn(), t_fn()
                row["parity"] = parity(fo, to)
                f_us, t_us, loops = measure(f_fn, t_fn)
                row.update({
                    "finkit_us": f_us,
                    "talib_us": t_us,
                    "speedup_talib_over_finkit": t_us / f_us,
                    "talib_faster_x": f_us / t_us,
                    "finkit_mbars_s": n / f_us,
                    "talib_mbars_s": n / t_us,
                    "loops": loops,
                })
                print(f"IND {name:12s} n={n:7d} f={f_us:11.2f}us ta={t_us:10.2f}us ta_faster={f_us/t_us:7.2f}x mask={row['parity']['finite_mask_equal']} abs={row['parity']['max_abs']:.3e}")
            except Exception as exc:
                row["error"] = f"{type(exc).__name__}: {exc}"
                print(f"IND_ERROR {name} n={n}: {row['error']}")
            indicator_rows.append(row)

        print(f"=== formulas n={n} ===")
        for name, expr, mode, f_fn, ref_fn in formula_cases(open_, high, low, close, volume):
            row = {"formula": name, "expr": expr, "mode": mode, "bars": n}
            try:
                fo, ro = f_fn(), ref_fn()
                row["parity"] = parity(fo, ro)
                f_us, r_us, loops = measure(f_fn, ref_fn)
                row.update({
                    "finkit_us": f_us,
                    "reference_us": r_us,
                    "speedup_reference_over_finkit": r_us / f_us,
                    "reference_faster_x": f_us / r_us,
                    "loops": loops,
                })
                print(f"FORM {name:10s} {mode:14s} n={n:7d} f={f_us:11.2f}us ref={r_us:10.2f}us ref_faster={f_us/r_us:7.2f}x mask={row['parity']['finite_mask_equal']} abs={row['parity']['max_abs']:.3e}")
            except Exception as exc:
                row["error"] = f"{type(exc).__name__}: {exc}"
                print(f"FORM_ERROR {name} {mode} n={n}: {row['error']}")
            formula_rows.append(row)

    ok_ind = [r for r in indicator_rows if "speedup_talib_over_finkit" in r]
    ok_formula = [r for r in formula_rows if "speedup_reference_over_finkit" in r]
    gm_ind = math.exp(statistics.mean(math.log(r["speedup_talib_over_finkit"]) for r in ok_ind))
    gm_formula = math.exp(statistics.mean(math.log(r["speedup_reference_over_finkit"]) for r in ok_formula))
    ind_wins = sum(1 for r in ok_ind if r["speedup_talib_over_finkit"] > 1.0)
    formula_wins = sum(1 for r in ok_formula if r["speedup_reference_over_finkit"] > 1.0)

    summary = {
        "indicator_cases": len(ok_ind),
        "finkit_indicator_wins": ind_wins,
        "indicator_geomean_speedup_talib_over_finkit": gm_ind,
        "indicator_geomean_talib_faster_x": 1.0 / gm_ind,
        "formula_cases": len(ok_formula),
        "finkit_formula_wins": formula_wins,
        "formula_geomean_speedup_reference_over_finkit": gm_formula,
        "formula_geomean_reference_faster_x": 1.0 / gm_formula,
    }
    print("SUMMARY", json.dumps(summary, ensure_ascii=False))

    out_dir = Path("dist/bench")
    out_dir.mkdir(parents=True, exist_ok=True)
    data = {"environment": env, "summary": summary, "indicators": indicator_rows, "formulas": formula_rows}
    (out_dir / "python-vs-talib-extended.json").write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    lines = [
        "# Finkit 0.1.4 vs TA-Lib 0.7.1 — Extended installed-package benchmark",
        "",
        f"- Indicator observations: **{len(ok_ind)}**, Finkit wins: **{ind_wins}**",
        f"- Indicator geometric mean (TA-Lib/Finkit): **{gm_ind:.4f}x** (TA-Lib about **{1/gm_ind:.1f}x faster** at the public Python layer)",
        f"- Formula observations: **{len(ok_formula)}**, Finkit wins: **{formula_wins}**",
        f"- Formula geometric mean (reference/Finkit): **{gm_formula:.4f}x** (reference about **{1/gm_formula:.1f}x faster**)",
        "",
        "## Indicators",
        "",
        "| Indicator | Category | Bars | Finkit us | TA-Lib us | TA-Lib faster | Mask equal | Max abs |",
        "|---|---|---:|---:|---:|---:|---|---:|",
    ]
    for r in indicator_rows:
        if "error" in r:
            lines.append(f"| {r['indicator']} | {r['category']} | {r['bars']:,} | ERROR | - | - | - | {r['error']} |")
        else:
            p = r["parity"]
            lines.append(f"| {r['indicator']} | {r['category']} | {r['bars']:,} | {r['finkit_us']:.2f} | {r['talib_us']:.2f} | {r['talib_faster_x']:.2f}x | {p['finite_mask_equal']} | {p['max_abs']:.2e} |")
    lines += [
        "",
        "## Compiled formulas",
        "",
        "Reference is TA-Lib public Python calls plus NumPy only where a formula has no single-call TA-Lib equivalent.",
        "",
        "| Formula | Mode | Bars | Finkit us | Reference us | Reference faster | Mask equal | Max abs |",
        "|---|---|---:|---:|---:|---:|---|---:|",
    ]
    for r in formula_rows:
        if "error" in r:
            lines.append(f"| {r['formula']} | {r['mode']} | {r['bars']:,} | ERROR | - | - | - | {r['error']} |")
        else:
            p = r["parity"]
            lines.append(f"| {r['formula']} | {r['mode']} | {r['bars']:,} | {r['finkit_us']:.2f} | {r['reference_us']:.2f} | {r['reference_faster_x']:.2f}x | {p['finite_mask_equal']} | {p['max_abs']:.2e} |")
    (out_dir / "python-vs-talib-extended.md").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines[:20]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
