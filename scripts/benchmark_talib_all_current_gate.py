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
import subprocess
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
from benchmark_gate_contract import evaluate_performance_rows


def talib_donchian_outputs(value: Any) -> tuple[Any, Any, Any]:
    """Drop Finkit's extra width output; the first three match TA-Lib order."""
    return value[0], value[1], value[2]


def extra_cases(o, h, l, c, v):
    periods = np.full(c.shape, 14.0)
    # Trigonometric inverse functions have a restricted domain.  Keep their
    # benchmark input valid instead of turning a numerical case into an API
    # error unrelated to the implementation under test.
    math_input = np.linspace(-0.9, 0.9, c.size, dtype=np.float64)
    cases = [
        ("SUM", lambda: finkit.sum(c, timeperiod=30), lambda: talib.SUM(c, timeperiod=30)),
        ("VORTEX", lambda: finkit.vortex(h, l, c, timeperiod=14), lambda: talib.VORTEX(h, l, c, timeperiod=14)),
        ("VWAP", lambda: finkit.vwap(h, l, c, v), lambda: talib.VWAP(h, l, c, v)),
        ("DONCHIAN", lambda: talib_donchian_outputs(finkit.donchian(h, l, period=20)), lambda: talib.DONCHIAN(h, l, timeperiod=20)),
        ("SUPERTREND", lambda: finkit.supertrend_talib(h, l, c, timeperiod=10, multiplier=3.0), lambda: talib.SUPERTREND(h, l, c, timeperiod=10, multiplier=3.0)),
        ("AVGDEV", lambda: finkit.avgdev(c, timeperiod=14), lambda: talib.AVGDEV(c, timeperiod=14)),
        ("DPO", lambda: finkit.dpo(c, timeperiod=20), lambda: talib.DPO(c, timeperiod=20)),
        ("ER", lambda: finkit.er(c, timeperiod=10), lambda: talib.ER(c, timeperiod=10)),
        ("FOSC", lambda: finkit.fosc(c, timeperiod=5), lambda: talib.FOSC(c, timeperiod=5)),
        ("PERCENTRANK", lambda: finkit.percentrank(c, timeperiod=100), lambda: talib.PERCENTRANK(c, timeperiod=100)),
        ("RMA", lambda: finkit.rma(c, timeperiod=30), lambda: talib.RMA(c, timeperiod=30)),
        ("VHF", lambda: finkit.vhf(c, timeperiod=28), lambda: talib.VHF(c, timeperiod=28)),
        ("HMA", lambda: finkit.hma(c, timeperiod=20), lambda: talib.HMA(c, timeperiod=20)),
        ("ZLEMA", lambda: finkit.zlema(c, timeperiod=30), lambda: talib.ZLEMA(c, timeperiod=30)),
        ("RVI", lambda: finkit.rvi(c, timeperiod=14, stddevperiod=10), lambda: talib.RVI(c, timeperiod=14, stddevperiod=10)),
        ("ADR", lambda: finkit.adr(h, l, timeperiod=14), lambda: talib.ADR(h, l, timeperiod=14)),
        ("AO", lambda: finkit.ao(h, l, fastperiod=5, slowperiod=34), lambda: talib.AO(h, l, fastperiod=5, slowperiod=34)),
        ("CMOU", lambda: finkit.cmou(c, timeperiod=14), lambda: talib.CMOU(c, timeperiod=14)),
        ("CVI", lambda: finkit.cvi(h, l, timeperiod=10, rocperiod=10), lambda: talib.CVI(h, l, timeperiod=10, rocperiod=10)),
        ("EFI", lambda: finkit.efi(c, v, timeperiod=13), lambda: talib.EFI(c, v, timeperiod=13)),
        ("MARKETFI", lambda: finkit.marketfi(h, l, v), lambda: talib.MARKETFI(h, l, v)),
        ("MASSI", lambda: finkit.massi(h, l, fastperiod=9, slowperiod=25), lambda: talib.MASSI(h, l, fastperiod=9, slowperiod=25)),
        ("MINUS_DM", lambda: finkit.minus_dm(h, l, timeperiod=14), lambda: talib.MINUS_DM(h, l, timeperiod=14)),
        ("NVI", lambda: finkit.nvi(c, v), lambda: talib.NVI(c, v)),
        ("PLUS_DM", lambda: finkit.plus_dm(h, l, timeperiod=14), lambda: talib.PLUS_DM(h, l, timeperiod=14)),
        ("PVI", lambda: finkit.pvi(c, v), lambda: talib.PVI(c, v)),
        ("PVT", lambda: finkit.pvt(c, v), lambda: talib.PVT(c, v)),
        ("QSTICK", lambda: finkit.qstick(o, c, timeperiod=10), lambda: talib.QSTICK(o, c, timeperiod=10)),
        ("RVOL", lambda: finkit.rvol(v, timeperiod=20), lambda: talib.RVOL(v, timeperiod=20)),
        ("TSI", lambda: finkit.tsi(c, firstperiod=25, secondperiod=13), lambda: talib.TSI(c, firstperiod=25, secondperiod=13)),
        ("VWMA", lambda: finkit.vwma(c, v, timeperiod=30), lambda: talib.VWMA(c, v, timeperiod=30)),
        ("WAD", lambda: finkit.wad(h, l, c), lambda: talib.WAD(h, l, c)),
        ("AC", lambda: finkit.ac(h, l, fastperiod=5, slowperiod=34, signalperiod=5), lambda: talib.AC(h, l, fastperiod=5, slowperiod=34, signalperiod=5)),
        ("ACCBANDS", lambda: finkit.accbands(h, l, c, timeperiod=20), lambda: talib.ACCBANDS(h, l, c, timeperiod=20)),
        ("CMF", lambda: finkit.cmf(h, l, c, v, timeperiod=20), lambda: talib.CMF(h, l, c, v, timeperiod=20)),
        ("COPPOCK", lambda: finkit.coppock(c, wmaperiod=10, roc1period=11, roc2period=14), lambda: talib.COPPOCK(c, wmaperiod=10, roc1period=11, roc2period=14)),
        ("CUMSUM", lambda: finkit.cumsum(c), lambda: talib.CUMSUM(c)),
        ("ERI", lambda: finkit.eri(h, l, c, timeperiod=13), lambda: talib.ERI(h, l, c, timeperiod=13)),
        ("FRACTAL", lambda: finkit.fractal(h, l, leftbars=2, rightbars=2), lambda: talib.FRACTAL(h, l, leftbars=2, rightbars=2)),
        ("HA", lambda: finkit.ha(o, h, l, c), lambda: talib.HA(o, h, l, c)),
        ("IMI", lambda: finkit.imi(o, c, timeperiod=14), lambda: talib.IMI(o, c, timeperiod=14)),
        ("KC", lambda: finkit.kc(h, l, c, timeperiod=20, atrperiod=10, nbdev=2.0), lambda: talib.KC(h, l, c, timeperiod=20, atrperiod=10, nbdev=2.0)),
        ("KDJ", lambda: finkit.kdj(h, l, c, fastk_period=9, slowk_period=3, slowk_matype=13, slowd_period=3, slowd_matype=13), lambda: talib.KDJ(h, l, c, fastk_period=9, slowk_period=3, slowk_matype=13, slowd_period=3, slowd_matype=13)),
        ("PERCENTILE", lambda: finkit.percentile(c, timeperiod=30, percentile=50.0), lambda: talib.PERCENTILE(c, timeperiod=30, percentile=50.0)),
        ("PVO", lambda: finkit.pvo(v, fastperiod=12, slowperiod=26, matype=1), lambda: talib.PVO(v, fastperiod=12, slowperiod=26, matype=1)),
        ("SMI", lambda: finkit.smi(h, l, c, timeperiod=13, fastperiod=2, slowperiod=25, signalperiod=9), lambda: talib.SMI(h, l, c, timeperiod=13, fastperiod=2, slowperiod=25, signalperiod=9)),
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
    repo_root = Path(__file__).resolve().parents[1]
    try:
        git_sha = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        git_sha = None
    try:
        rustc_version = subprocess.run(
            ["rustc", "--version"],
            cwd=repo_root,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        rustc_version = None
    env = {
        "python": sys.version,
        "platform": platform.platform(),
        "cpu": platform.processor(),
        "numpy": np.__version__,
        "finkit": importlib.metadata.version("finkit"),
        "finkit_module": str(Path(finkit.__file__).resolve()),
        "finkit_source_git_sha": git_sha,
        "rustc": rustc_version,
        "talib_python": importlib.metadata.version("TA-Lib"),
        "talib_core": getattr(talib, "__ta_version__", b"").decode("ascii", errors="replace"),
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
    upstream_names = sorted(name.upper() for name in talib.get_functions())
    performance = evaluate_performance_rows(
        rows, upstream_names, minimum_speedup=1.0, expected_sizes=args.sizes
    )
    performance["observation_count"] = len(rows)
    performance["expected_observation_count"] = len(upstream_names) * len(args.sizes)
    summary = {
        "environment": env,
        "sizes": args.sizes,
        "case_count": len(by_name),
        "observation_count": len(rows),
        "errors": errors,
        "parity_failures": parity_failures,
        "performance_gate": performance,
        "all_observations_meet_speed_floor": performance["speed_pass"] and performance["coverage_pass"],
        "geomean_finkit_speedup": geomean,
        "rows": rows,
    }
    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
    print(json.dumps({key: summary[key] for key in ("case_count", "observation_count", "errors", "parity_failures", "performance_gate", "all_observations_meet_speed_floor", "geomean_finkit_speedup")}, indent=2, ensure_ascii=False))
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", nargs="+", type=int, default=[1_000, 100_000, 1_000_000])
    parser.add_argument("--output", default="dist/bench/talib-all-current-gate.json")
    args = parser.parse_args()
    summary = run(args)
    return (
        0
        if not summary["errors"]
        and not summary["parity_failures"]
        and summary["all_observations_meet_speed_floor"]
        else 2
    )


if __name__ == "__main__":
    raise SystemExit(main())
