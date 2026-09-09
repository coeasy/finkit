"""Full current public overlap gate against TA-Lib.

The original gate covered the indicators that were already exposed when PR 28
landed.  This companion gate adds the remaining TA-Lib surface, including all
61 candlestick functions and the math/operator/statistics families now exposed
by the direct compatibility bindings.
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
from pathlib import Path
from typing import Any, Callable

import numpy as np
import talib
import finkit

from benchmark_talib_full_current_gate import (
    data,
    flatten,
    measure,
    parity,
    talib_aroon_order,
)


def extra_cases(o, h, l, c, v):
    periods = np.full(c.shape, 14.0)
    # Trigonometric inverse functions have a restricted domain.  Keep their
    # benchmark input valid instead of turning a numerical case into an API
    # error unrelated to the implementation under test.
    math_input = np.linspace(-0.9, 0.9, c.size, dtype=np.float64)
    cases = [
        ("ADXR", lambda: finkit.adxr(h, l, c, timeperiod=14), lambda: talib.ADXR(h, l, c, timeperiod=14)),
        ("AROONOSC", lambda: finkit.aroonosc(h, l, timeperiod=14), lambda: talib.AROONOSC(h, l, timeperiod=14)),
        ("BBANDS", lambda: finkit.bbands(c, timeperiod=5, nbdevup=2.0, nbdevdn=2.0, matype=0), lambda: talib.BBANDS(c, timeperiod=5, nbdevup=2.0, nbdevdn=2.0, matype=0)),
        ("LINEARREG", lambda: finkit.linearreg(c, timeperiod=14), lambda: talib.LINEARREG(c, timeperiod=14)),
        ("LINEARREG_ANGLE", lambda: finkit.linearreg_angle(c, timeperiod=14), lambda: talib.LINEARREG_ANGLE(c, timeperiod=14)),
        ("LINEARREG_INTERCEPT", lambda: finkit.linearreg_intercept(c, timeperiod=14), lambda: talib.LINEARREG_INTERCEPT(c, timeperiod=14)),
        ("LINEARREG_SLOPE", lambda: finkit.linearreg_slope(c, timeperiod=14), lambda: talib.LINEARREG_SLOPE(c, timeperiod=14)),
        ("MA", lambda: finkit.ma(c, timeperiod=30, matype=0), lambda: talib.MA(c, timeperiod=30, matype=0)),
        ("MACDEXT", lambda: finkit.macdext(c, fastperiod=12, fastmatype=0, slowperiod=26, slowmatype=0, signalperiod=9, signalmatype=0), lambda: talib.MACDEXT(c, fastperiod=12, fastmatype=0, slowperiod=26, slowmatype=0, signalperiod=9, signalmatype=0)),
        ("MACDFIX", lambda: finkit.macdfix(c, signalperiod=9), lambda: talib.MACDFIX(c, signalperiod=9)),
        ("MAVP", lambda: finkit.mavp(c, periods, minperiod=2, maxperiod=30, matype=0), lambda: talib.MAVP(c, periods, minperiod=2, maxperiod=30, matype=0)),
        ("MAX", lambda: finkit.max(c, timeperiod=30), lambda: talib.MAX(c, timeperiod=30)),
        ("MAXINDEX", lambda: finkit.maxindex(c, timeperiod=30), lambda: talib.MAXINDEX(c, timeperiod=30)),
        ("MIN", lambda: finkit.min(c, timeperiod=30), lambda: talib.MIN(c, timeperiod=30)),
        ("MININDEX", lambda: finkit.minindex(c, timeperiod=30), lambda: talib.MININDEX(c, timeperiod=30)),
        ("MINMAX", lambda: finkit.minmax(c, timeperiod=30), lambda: talib.MINMAX(c, timeperiod=30)),
        ("MINMAXINDEX", lambda: finkit.minmaxindex(c, timeperiod=30), lambda: talib.MINMAXINDEX(c, timeperiod=30)),
        ("PPO", lambda: finkit.ppo(c, fastperiod=12, slowperiod=26, matype=0), lambda: talib.PPO(c, fastperiod=12, slowperiod=26, matype=0)),
        ("ROCP", lambda: finkit.rocp(c, timeperiod=10), lambda: talib.ROCP(c, timeperiod=10)),
        ("ROCR", lambda: finkit.rocr(c, timeperiod=10), lambda: talib.ROCR(c, timeperiod=10)),
        ("ROCR100", lambda: finkit.rocr100(c, timeperiod=10), lambda: talib.ROCR100(c, timeperiod=10)),
        ("SAREXT", lambda: finkit.sarext(h, l), lambda: talib.SAREXT(h, l)),
        ("STOCHF", lambda: finkit.stochf(h, l, c, fastk_period=5, fastd_period=3, fastd_matype=0), lambda: talib.STOCHF(h, l, c, fastk_period=5, fastd_period=3, fastd_matype=0)),
        ("STOCHRSI", lambda: finkit.stochrsi(c, timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0), lambda: talib.STOCHRSI(c, timeperiod=14, fastk_period=5, fastd_period=3, fastd_matype=0)),
        ("TRIMA", lambda: finkit.trima(c, timeperiod=30), lambda: talib.TRIMA(c, timeperiod=30)),
        ("ULTOSC", lambda: finkit.ultosc(h, l, c, timeperiod1=7, timeperiod2=14, timeperiod3=28), lambda: talib.ULTOSC(h, l, c, timeperiod1=7, timeperiod2=14, timeperiod3=28)),
    ]

    unary = ["ACOS", "ASIN", "ATAN", "CEIL", "COS", "COSH", "EXP", "FLOOR", "LN", "LOG10", "SIN", "SINH", "SQRT", "TAN", "TANH"]
    for name in unary:
        fn = getattr(finkit, name.lower())
        ta_fn = getattr(talib, name)
        source = math_input if name in {"ACOS", "ASIN"} else c
        cases.append((name, lambda fn=fn, source=source: fn(source), lambda ta_fn=ta_fn, source=source: ta_fn(source)))

    binary = ["ADD", "DIV", "MULT", "SUB"]
    for name in binary:
        fn = getattr(finkit, name.lower())
        ta_fn = getattr(talib, name)
        cases.append((name, lambda fn=fn: fn(c, o), lambda ta_fn=ta_fn: ta_fn(c, o)))

    candlesticks = sorted(name for name in talib.get_functions() if name.startswith("CDL"))
    for name in candlesticks:
        fn = getattr(finkit, name.lower())
        ta_fn = getattr(talib, name)
        cases.append((name, lambda fn=fn: fn(o, h, l, c), lambda ta_fn=ta_fn: ta_fn(o, h, l, c)))
    return cases


def run(args: argparse.Namespace) -> dict[str, Any]:
    env = {
        "python": sys.version,
        "platform": platform.platform(),
        "numpy": np.__version__,
        "finkit": importlib.metadata.version("finkit"),
        "talib_python": importlib.metadata.version("TA-Lib"),
    }
    rows = []
    errors = []
    parity_failures = []
    for n in args.sizes:
        o, h, l, c, v = data(n)
        all_cases = __import__("benchmark_talib_full_current_gate", fromlist=["cases"]).cases(o, h, l, c, v) + extra_cases(o, h, l, c, v)
        seen = set()
        for name, ff, tf in all_cases:
            if name in seen:
                continue
            seen.add(name)
            try:
                f_value = ff()
                t_value = tf()
                ok, mask, abs_err, rel_err = parity(f_value, t_value)
                if not ok:
                    parity_failures.append(f"{name}:n={n}:mask={mask}:abs={abs_err:.3e}:rel={rel_err:.3e}")
                f_us, t_us, loops = measure(ff, tf)
                speedup = t_us / f_us
                rows.append({"name": name, "n": n, "finkit_us": f_us, "talib_us": t_us, "finkit_speedup_x": speedup, "parity": ok, "mask_equal": mask, "max_abs": abs_err, "max_rel": rel_err, "loops": loops})
                print(f"{name:24s} n={n:8d} f={f_us:10.2f}us ta={t_us:10.2f}us finkit={speedup:7.2f}x parity={ok}")
            except Exception as exc:
                errors.append(f"{name}:n={n}:{type(exc).__name__}:{exc}")
                print("ERROR", errors[-1])
    by_name = {}
    for row in rows:
        by_name.setdefault(row["name"], []).append(row["finkit_speedup_x"])
    geomean = math.exp(statistics.mean(math.log(row["finkit_speedup_x"]) for row in rows)) if rows else 0.0
    summary = {"environment": env, "sizes": args.sizes, "case_count": len(by_name), "observation_count": len(rows), "errors": errors, "parity_failures": parity_failures, "all_observations_faster": bool(rows) and all(row["finkit_speedup_x"] > 1.0 for row in rows), "all_cases_faster_at_every_size": bool(by_name) and all(all(value > 1.0 for value in values) for values in by_name.values()), "geomean_finkit_speedup": geomean, "rows": rows}
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps({key: summary[key] for key in ("case_count", "observation_count", "errors", "parity_failures", "all_observations_faster", "all_cases_faster_at_every_size", "geomean_finkit_speedup")}, indent=2, ensure_ascii=False))
    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", nargs="+", type=int, default=[100_000])
    parser.add_argument("--output", default="dist/bench/talib-all-current-gate.json")
    run(parser.parse_args())
