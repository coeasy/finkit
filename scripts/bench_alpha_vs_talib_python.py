#!/usr/bin/env python3
"""Full TA-Lib v0.6.4 Python API comparison for finkit.

The benchmark scope is the 161 functions exported by TA-Lib Python v0.6.4
(`_ta_lib.pyi`).  It deliberately records unsupported functions instead of
silently dropping them, so a green process means only that the report was
generated; use ``--strict`` to make incomplete coverage fail the command.

Example::

    python scripts/bench_alpha_vs_talib_python.py --scale 100K --repeat 7 \
        --output dist/bench/talib-v064 --strict
"""

from __future__ import annotations

import argparse
import json
import math
import statistics
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable

import numpy as np


# This is the public Python surface in TA-Lib v0.6.4.  NVI/PVI are present in
# the C source tree as unfinished templates, but are intentionally not in the
# Python package and therefore are outside this executable comparison scope.
TALIB_V064_FUNCTIONS = tuple(
    """
    BBANDS DEMA EMA HT_TRENDLINE KAMA MA MAMA MAVP MIDPOINT MIDPRICE SAR SAREXT
    SMA T3 TEMA TRIMA WMA
    ADX ADXR APO AROON AROONOSC BOP CCI CMO DX MACD MACDEXT MACDFIX MFI
    MINUS_DI MINUS_DM MOM PLUS_DI PLUS_DM PPO ROC ROCP ROCR ROCR100 RSI
    STOCH STOCHF STOCHRSI TRIX ULTOSC WILLR
    AD ADOSC OBV
    ATR NATR TRANGE
    AVGPRICE MEDPRICE TYPPRICE WCLPRICE
    HT_DCPERIOD HT_DCPHASE HT_PHASOR HT_SINE HT_TRENDMODE
    CDL2CROWS CDL3BLACKCROWS CDL3INSIDE CDL3LINESTRIKE CDL3OUTSIDE
    CDL3STARSINSOUTH CDL3WHITESOLDIERS CDLABANDONEDBABY CDLADVANCEBLOCK
    CDLBELTHOLD CDLBREAKAWAY CDLCLOSINGMARUBOZU CDLCONCEALBABYSWALL
    CDLCOUNTERATTACK CDLDARKCLOUDCOVER CDLDOJI CDLDOJISTAR CDLDRAGONFLYDOJI
    CDLENGULFING CDLEVENINGDOJISTAR CDLEVENINGSTAR CDLGAPSIDESIDEWHITE
    CDLGRAVESTONEDOJI CDLHAMMER CDLHANGINGMAN CDLHARAMI CDLHARAMICROSS
    CDLHIGHWAVE CDLHIKKAKE CDLHIKKAKEMOD CDLHOMINGPIGEON CDLIDENTICAL3CROWS
    CDLINNECK CDLINVERTEDHAMMER CDLKICKING CDLKICKINGBYLENGTH
    CDLLADDERBOTTOM CDLLONGLEGGEDDOJI CDLLONGLINE CDLMARUBOZU CDLMATCHINGLOW
    CDLMATHOLD CDLMORNINGDOJISTAR CDLMORNINGSTAR CDLONNECK CDLPIERCING
    CDLRICKSHAWMAN CDLRISEFALL3METHODS CDLSEPARATINGLINES CDLSHOOTINGSTAR
    CDLSHORTLINE CDLSPINNINGTOP CDLSTALLEDPATTERN CDLSTICKSANDWICH CDLTAKURI
    CDLTASUKIGAP CDLTHRUSTING CDLTRISTAR CDLUNIQUE3RIVER CDLUPsideGAP2CROWS
    CDLXSIDEGAP3METHODS
    BETA CORREL LINEARREG LINEARREG_ANGLE LINEARREG_INTERCEPT LINEARREG_SLOPE
    STDDEV TSF VAR
    ACOS ASIN ATAN CEIL COS COSH EXP FLOOR LN LOG10 SIN SINH SQRT TAN TANH
    ADD DIV MAX MAXINDEX MIN MININDEX MINMAX MINMAXINDEX MULT SUB SUM
    ACCBANDS AVGDEV IMI
    """.replace("CDLUPsideGAP2CROWS", "CDLUPSIDEGAP2CROWS").split()
)

# Keep the spelling check close to the source list.  A typo here must never
# silently reduce the comparison matrix.
if len(TALIB_V064_FUNCTIONS) != 161 or len(set(TALIB_V064_FUNCTIONS)) != 161:
    raise RuntimeError("the TA-Lib v0.6.4 function list must contain 161 unique names")

PATTERN_NAMES = frozenset(name for name in TALIB_V064_FUNCTIONS if name.startswith("CDL"))
MATH_TRANSFORMS = frozenset(
    "ACOS ASIN ATAN CEIL COS COSH EXP FLOOR LN LOG10 SIN SINH SQRT TAN TANH".split()
)
MATH_OPERATORS = frozenset(
    "ADD DIV MAX MAXINDEX MIN MININDEX MINMAX MINMAXINDEX MULT SUB SUM".split()
)


# Existing direct finkit names.  The lower-case TA-Lib name remains the
# fallback for names whose public spelling already matches finkit's API.
FINKIT_ALIASES = {
    "BBANDS": "bollinger_bands",
    "CORREL": "correlation",
    "STDDEV": "std_dev",
    "LINEARREG": "linear_reg",
    "HT_DCPERIOD": "ht_dcperiod",
    "HT_DCPHASE": "ht_dcphase",
    "HT_PHASOR": "ht_phasor",
    "HT_SINE": "ht_sine",
    "HT_TRENDMODE": "ht_trendmode",
    "HT_TRENDLINE": "ht_trendline",
    "CDLDOJI": "cdl_doji",
    "CDLDRAGONFLYDOJI": "cdl_dragonfly_doji",
    "CDLGRAVESTONEDOJI": "cdl_gravestone_doji",
    "CDLLONGLEGGEDDOJI": "cdl_long_legged_doji",
    "CDLHAMMER": "cdl_hammer",
    "CDLINVERTEDHAMMER": "cdl_inverted_hammer",
    "CDLHANGINGMAN": "cdl_hanging_man",
    "CDLSHOOTINGSTAR": "cdl_shooting_star",
    "CDLENGULFING": "cdl_engulfing",
    "CDLHARAMI": "cdl_harami",
    "CDLHARAMICROSS": "cdl_harami_cross",
    "CDLMORNINGSTAR": "cdl_morning_star",
    "CDLEVENINGSTAR": "cdl_evening_star",
    "CDLMORNINGDOJISTAR": "cdl_morning_doji_star",
    "CDLEVENINGDOJISTAR": "cdl_evening_doji_star",
    "CDLMARUBOZU": "cdl_marubozu",
    "CDL3WHITESOLDIERS": "cdl_three_white_soldiers",
    "CDL3BLACKCROWS": "cdl_three_black_crows",
    "CDLPIERCING": "cdl_piercing",
    "CDLDARKCLOUDCOVER": "cdl_dark_cloud_cover",
    "CDLBELTHOLD": "cdl_belt_hold",
    "CDLSPINNINGTOP": "cdl_spinning_top",
    "CDLHIGHWAVE": "cdl_high_wave",
    "CDLRICKSHAWMAN": "cdl_rickshaw_man",
    "CDLSHORTLINE": "cdl_short_line",
    "CDLLONGLINE": "cdl_long_line",
    "CDLKICKING": "cdl_kicking",
}


def parse_scale(value: str) -> int:
    """Parse 10K/100K/1M or a positive integer."""
    text = value.strip().upper().replace(",", "")
    multiplier = 1
    if text.endswith("K"):
        multiplier, text = 1_000, text[:-1]
    elif text.endswith("M"):
        multiplier, text = 1_000_000, text[:-1]
    size = int(text) * multiplier
    if size < 64:
        raise argparse.ArgumentTypeError("scale must be at least 64 bars")
    return size


def make_data(n: int, seed: int) -> dict[str, np.ndarray]:
    """Create deterministic, internally consistent OHLCV arrays."""
    rng = np.random.default_rng(seed)
    close = 100.0 * np.exp(np.cumsum(rng.normal(0.0, 0.006, n)))
    open_price = close * (1.0 + rng.normal(0.0, 0.002, n))
    spread = np.abs(rng.normal(0.006, 0.002, n)) * close
    high = np.maximum(open_price, close) + spread
    low = np.minimum(open_price, close) - spread
    volume = rng.integers(100_000, 10_000_000, n).astype(np.float64)
    periods = rng.integers(2, 31, n).astype(np.float64)
    math_data = np.clip(close / np.nanmax(close), 0.05, 0.95)
    alternate = close * (1.0 + rng.normal(0.0, 0.01, n))
    return {
        "open": open_price.astype(np.float64),
        "high": high.astype(np.float64),
        "low": low.astype(np.float64),
        "close": close.astype(np.float64),
        "volume": volume,
        "periods": periods,
        "math": math_data.astype(np.float64),
        "alternate": alternate.astype(np.float64),
    }


def spec_for(name: str) -> dict[str, Any]:
    """Return one deterministic TA-Lib call specification."""
    if name in PATTERN_NAMES:
        return {"inputs": ("open", "high", "low", "close"), "params": (), "returns": 1, "category": "Pattern"}
    if name in MATH_TRANSFORMS:
        return {"inputs": ("math",), "params": (), "returns": 1, "category": "Math Transform"}
    if name in MATH_OPERATORS:
        if name in {"ADD", "DIV", "MULT", "SUB"}:
            return {"inputs": ("close", "alternate"), "params": (), "returns": 1, "category": "Math Operator"}
        returns = 2 if name in {"MINMAX", "MINMAXINDEX"} else 1
        return {"inputs": ("close",), "params": (30,), "returns": returns, "category": "Math Operator"}
    if name == "MAVP":
        return {"inputs": ("close", "periods"), "params": (2, 30, 0), "returns": 1, "category": "Overlap"}
    if name == "BBANDS":
        return {"inputs": ("close",), "params": (20, 2.0, 2.0, 0), "alpha_params": (20, 2.0, 2.0), "returns": 3, "category": "Overlap"}
    if name == "ACCBANDS":
        return {"inputs": ("high", "low", "close"), "params": (20,), "returns": 3, "category": "Overlap"}
    if name in {"MA", "SMA", "EMA", "WMA", "DEMA", "TEMA", "TRIMA", "KAMA", "T3", "MIDPOINT", "MIDPRICE"}:
        inputs = ("high", "low") if name == "MIDPRICE" else ("close",)
        params = (20, 0) if name == "MA" else ((20, 0.7) if name == "T3" else (20,))
        alpha_params = (20,) if name == "MA" else params
        return {"inputs": inputs, "params": params, "alpha_params": alpha_params, "returns": 1, "category": "Overlap"}
    if name == "MAMA":
        return {"inputs": ("close",), "params": (0.5, 0.05), "returns": 2, "category": "Overlap"}
    if name in {"SAR"}:
        return {"inputs": ("high", "low"), "params": (0.02, 0.2), "returns": 1, "category": "Overlap"}
    if name == "SAREXT":
        return {"inputs": ("high", "low"), "params": (), "returns": 1, "category": "Overlap"}
    if name in {"MACD", "MACDFIX"}:
        params = (12, 26, 9) if name == "MACD" else (9,)
        return {"inputs": ("close",), "params": params, "alpha_params": params if name == "MACD" else (), "returns": 3, "category": "Momentum"}
    if name == "MACDEXT":
        return {"inputs": ("close",), "params": (12, 0, 26, 0, 9, 0), "returns": 3, "category": "Momentum"}
    if name == "STOCH":
        return {"inputs": ("high", "low", "close"), "params": (5, 3, 0, 3, 0), "alpha_params": (5, 3, 3), "returns": 2, "category": "Momentum"}
    if name == "STOCHF":
        return {"inputs": ("high", "low", "close"), "params": (5, 3, 0), "alpha_params": (5, 3), "returns": 2, "category": "Momentum"}
    if name == "STOCHRSI":
        return {"inputs": ("close",), "params": (14, 5, 3, 0), "returns": 2, "category": "Momentum"}
    if name == "AROON":
        return {"inputs": ("high", "low"), "params": (14,), "returns": 2, "category": "Momentum"}
    if name == "ULTOSC":
        return {"inputs": ("high", "low", "close"), "params": (7, 14, 28), "returns": 1, "category": "Momentum"}
    if name == "ADOSC":
        return {"inputs": ("high", "low", "close", "volume"), "params": (3, 10), "returns": 1, "category": "Volume"}
    if name in {"ADX", "ADXR", "ATR", "NATR", "CCI", "CMO", "DX", "MFI", "MINUS_DI", "MINUS_DM", "PLUS_DI", "PLUS_DM", "RSI", "TRIX", "WILLR", "AVGDEV", "IMI"}:
        if name in {"ADX", "ADXR", "ATR", "NATR", "CCI", "DX", "MINUS_DI", "MINUS_DM", "PLUS_DI", "PLUS_DM", "WILLR"}:
            inputs = ("high", "low", "close")
        elif name == "MFI":
            inputs = ("high", "low", "close", "volume")
        elif name == "IMI":
            inputs = ("open", "close")
        else:
            inputs = ("close",)
        alpha_params = () if name in {"MINUS_DM", "PLUS_DM"} else (14,)
        return {"inputs": inputs, "params": (14,), "alpha_params": alpha_params, "returns": 1, "category": "Momentum" if name != "AVGDEV" else "Statistic"}
    if name in {"AD", "OBV"}:
        inputs = ("high", "low", "close", "volume") if name == "AD" else ("close", "volume")
        return {"inputs": inputs, "params": (), "returns": 1, "category": "Volume"}
    if name in {"HT_DCPERIOD", "HT_DCPHASE", "HT_TRENDLINE", "HT_TRENDMODE", "HT_PHASOR", "HT_SINE"}:
        return {"inputs": ("close",), "params": (), "returns": 2 if name in {"HT_PHASOR", "HT_SINE"} else 1, "category": "Cycle"}
    if name in {"AVGPRICE"}:
        return {"inputs": ("open", "high", "low", "close"), "params": (), "returns": 1, "category": "Price Transform"}
    if name in {"MEDPRICE"}:
        return {"inputs": ("high", "low"), "params": (), "returns": 1, "category": "Price Transform"}
    if name in {"TYPPRICE", "WCLPRICE"}:
        return {"inputs": ("high", "low", "close"), "params": (), "returns": 1, "category": "Price Transform"}
    if name in {"BETA", "CORREL"}:
        return {"inputs": ("close", "alternate"), "params": (30,), "returns": 1, "category": "Statistic"}
    if name in {"LINEARREG", "LINEARREG_ANGLE", "LINEARREG_INTERCEPT", "LINEARREG_SLOPE", "STDDEV", "TSF", "VAR"}:
        return {"inputs": ("close",), "params": (30,), "returns": 1, "category": "Statistic"}
    if name in {"APO", "PPO"}:
        return {"inputs": ("close",), "params": (12, 26, 0), "alpha_params": (12, 26), "returns": 1, "category": "Momentum"}
    if name in {"ROC", "ROCP", "ROCR", "ROCR100", "MOM"}:
        return {"inputs": ("close",), "params": (14,), "returns": 1, "category": "Momentum"}
    return {"inputs": ("close",), "params": (), "returns": 1, "category": "Other"}


def resolve_direct(finkit: Any, name: str) -> Callable[..., Any] | None:
    candidate = FINKIT_ALIASES.get(name, name.lower())
    fn = getattr(finkit, candidate, None)
    return fn if callable(fn) else None


def invoke_batch(finkit: Any, data: dict[str, np.ndarray], name: str, spec: dict[str, Any]) -> Any:
    batch = getattr(finkit, "compute_indicators", None)
    if not callable(batch):
        raise LookupError("finkit.compute_indicators is not available")
    request = [(name.lower(), list(spec["params"]))]
    primary = data["math"] if name in MATH_TRANSFORMS else data["close"]
    if name == "MAVP":
        secondary = data["periods"]
    elif name in {"BETA", "CORREL", "ADD", "DIV", "MULT", "SUB"}:
        secondary = data["alternate"]
    else:
        secondary = None
    result = batch(
        close=primary,
        open=data["open"],
        high=data["high"],
        low=data["low"],
        volume=data["volume"],
        secondary=secondary,
        requests=request,
    )
    prefix = f"{name.lower()}_" + "_".join(str(value) for value in spec["params"])
    if not spec["params"]:
        prefix = f"{name.lower()}_"
    error_key = f"{prefix}_error"
    if error_key in result:
        raise RuntimeError(str(result[error_key]))
    if spec["returns"] == 1:
        key = prefix
        if key not in result:
            raise LookupError(f"batch result {key!r} is missing")
        return result[key]
    values = []
    for index in range(spec["returns"]):
        key = f"{prefix}{index}"
        if key not in result:
            raise LookupError(f"batch result {key!r} is missing")
        values.append(result[key])
    return tuple(values)


def invoke_direct(fn: Callable[..., Any], data: dict[str, np.ndarray], spec: dict[str, Any]) -> Any:
    args = [data[key] for key in spec["inputs"]]
    return fn(*args, *spec.get("alpha_params", spec["params"]))


def normalise_output(value: Any) -> list[np.ndarray]:
    if isinstance(value, (tuple, list)):
        return [np.asarray(item, dtype=np.float64).reshape(-1) for item in value]
    return [np.asarray(value, dtype=np.float64).reshape(-1)]


def compare_outputs(alpha: Any, talib: Any) -> dict[str, Any]:
    left = normalise_output(alpha)
    right = normalise_output(talib)
    if len(left) != len(right):
        return {"shape_mismatch": True, "output_count": [len(left), len(right)], "max_abs_diff": math.inf, "max_rel_diff": math.inf, "finite_match_ratio": 0.0, "precision_ok": False}
    max_abs = 0.0
    max_rel = 0.0
    finite_total = 0
    finite_equal = 0
    shape_mismatch = False
    precision_ok = True
    for a, b in zip(left, right):
        if a.shape != b.shape:
            shape_mismatch = True
            precision_ok = False
            continue
        a_finite = np.isfinite(a)
        b_finite = np.isfinite(b)
        both = a_finite & b_finite
        finite_total += int(np.count_nonzero(both))
        finite_equal += int(np.count_nonzero(a_finite == b_finite))
        if np.any(both):
            delta = np.abs(a[both] - b[both])
            scale = np.maximum(np.abs(b[both]), 1e-12)
            max_abs = max(max_abs, float(np.max(delta)))
            max_rel = max(max_rel, float(np.max(delta / scale)))
        precision_ok = precision_ok and bool(np.allclose(a, b, rtol=1e-6, atol=1e-8, equal_nan=True))
    return {"shape_mismatch": shape_mismatch, "output_count": len(left), "max_abs_diff": max_abs, "max_rel_diff": max_rel, "finite_match_ratio": finite_equal / max(a.size if left else 1, 1), "precision_ok": precision_ok}


def median_time(fn: Callable[[], Any], repeat: int, warmup: int) -> tuple[float, Any]:
    for _ in range(warmup):
        fn()
    samples = []
    last = None
    for _ in range(repeat):
        started = time.perf_counter_ns()
        last = fn()
        samples.append((time.perf_counter_ns() - started) / 1_000_000.0)
    return statistics.median(samples), last


def run(args: argparse.Namespace) -> int:
    try:
        import finkit
        import talib
    except ImportError as exc:
        print(f"[bench] missing dependency: {exc}", file=sys.stderr)
        return 2

    data = make_data(args.scale, args.seed)
    results: list[dict[str, Any]] = []
    for index, name in enumerate(TALIB_V064_FUNCTIONS, 1):
        spec = spec_for(name)
        talib_fn = getattr(talib, name, None)
        row: dict[str, Any] = {"index": index, "name": name, **spec, "status": "unavailable"}
        if talib_fn is None:
            row["reason"] = "not exported by installed TA-Lib Python package"
            results.append(row)
            continue
        direct = resolve_direct(finkit, name)
        if direct is not None:
            alpha_call = lambda fn=direct, s=spec: invoke_direct(fn, data, s)
            row["adapter"] = "direct"
        elif callable(getattr(finkit, "compute_indicators", None)):
            alpha_call = lambda n=name, s=spec: invoke_batch(finkit, data, n, s)
            row["adapter"] = "compute_indicators"
        else:
            row["reason"] = "no direct finkit function and no batch API"
            results.append(row)
            continue
        talib_call = lambda fn=talib_fn, s=spec: fn(*(data[key] for key in s["inputs"]), *s["params"])
        try:
            talib_ms, talib_value = median_time(talib_call, args.repeat, args.warmup)
            alpha_ms, alpha_value = median_time(alpha_call, args.repeat, args.warmup)
            comparison = compare_outputs(alpha_value, talib_value)
            row.update(comparison)
            row.update({"talib_ms": talib_ms, "finkit_ms": alpha_ms, "speedup": talib_ms / alpha_ms if alpha_ms else math.inf})
            row["status"] = "pass" if comparison["precision_ok"] else "precision-fail"
        except Exception as exc:  # one broken signature must not hide the other 160 rows
            row.update({"status": "error", "reason": f"{type(exc).__name__}: {exc}"})
        results.append(row)
        marker = "OK" if row["status"] == "pass" else row["status"].upper()
        speed = f"{row.get('speedup', 0.0):.2f}x" if "speedup" in row else "-"
        diff = f"{row.get('max_abs_diff', math.inf):.2e}" if "max_abs_diff" in row else "-"
        print(f"[{index:3d}/{len(TALIB_V064_FUNCTIONS)}] {marker:14s} {name:24s} speedup={speed:>8s} diff={diff}")

    compared = [row for row in results if "speedup" in row]
    passed = [row for row in compared if row["status"] == "pass"]
    speedups = [row["speedup"] for row in compared if math.isfinite(row["speedup"]) and row["speedup"] > 0]
    summary = {
        "scope": "TA-Lib Python v0.6.4",
        "talib_version": getattr(talib, "__version__", "unknown"),
        "finkit_version": getattr(finkit, "__version__", "unknown"),
        "timestamp": datetime.now(timezone.utc).isoformat(),
        "scale": args.scale,
        "seed": args.seed,
        "repeat": args.repeat,
        "warmup": args.warmup,
        "total_functions": len(results),
        "compared": len(compared),
        "unavailable": sum(row["status"] == "unavailable" for row in results),
        "errors": sum(row["status"] == "error" for row in results),
        "precision_pass": len(passed),
        "precision_fail": sum(row["status"] == "precision-fail" for row in results),
        "median_speedup": float(statistics.median(speedups)) if speedups else None,
        "geomean_speedup": float(math.exp(statistics.mean(math.log(x) for x in speedups))) if speedups else None,
        "faster_than_talib": sum(x >= 1.0 for x in speedups),
        "slower_than_talib": sum(x < 1.0 for x in speedups),
    }
    payload = {"summary": summary, "results": results, "functions": list(TALIB_V064_FUNCTIONS)}
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=True)
    (output / "python_comparison.json").write_text(json.dumps(payload, indent=2, ensure_ascii=False), encoding="utf-8")
    write_report(output / "python_comparison.md", summary, results)
    write_summary(output / "python_comparison_summary.md", summary, results)
    print(f"\n[bench] compared={summary['compared']}/{summary['total_functions']} precision={summary['precision_pass']}/{summary['compared']} median_speedup={summary['median_speedup']}")
    print(f"[bench] JSON: {output / 'python_comparison.json'}")
    return 1 if args.strict and (summary["unavailable"] or summary["errors"] or summary["precision_fail"]) else 0


def write_report(path: Path, summary: dict[str, Any], rows: list[dict[str, Any]]) -> None:
    lines = ["# finkit vs TA-Lib v0.6.4 — Full Python API comparison", "", f"- Scope: `{summary['scope']}` ({summary['total_functions']} functions)", f"- TA-Lib: `{summary['talib_version']}`; finkit: `{summary['finkit_version']}`", f"- Compared: **{summary['compared']}/{summary['total_functions']}**; precision pass: **{summary['precision_pass']}**", f"- Median speedup: **{summary['median_speedup']}x**; geometric mean: **{summary['geomean_speedup']}x**", "", "`speedup > 1.0x` means finkit completed faster. NaN positions are compared as equal; the first warm-up calls are excluded.", "", "## Per-function result", "", "| # | Function | Category | Adapter | Status | finkit ms | TA-Lib ms | Speedup | Max abs diff |", "|---:|---|---|---|---|---:|---:|---:|---:|"]
    for row in rows:
        finkit_ms = row.get("finkit_ms", "-")
        talib_ms = row.get("talib_ms", "-")
        speedup = row.get("speedup", "-")
        diff = row.get("max_abs_diff", row.get("reason", "-"))
        finkit_text = f"{finkit_ms:.4f}" if isinstance(finkit_ms, (float, int)) else str(finkit_ms)
        talib_text = f"{talib_ms:.4f}" if isinstance(talib_ms, (float, int)) else str(talib_ms)
        speedup_text = f"{speedup:.2f}" if isinstance(speedup, (float, int)) else str(speedup)
        lines.append(f"| {row['index']} | `{row['name']}` | {row['category']} | {row.get('adapter', '-')} | {row['status']} | {finkit_text} | {talib_text} | {speedup_text} | {diff} |")
    missing = [row for row in rows if row["status"] in {"unavailable", "error", "precision-fail"}]
    if missing:
        lines.extend(["", "## Action list", "", "The following rows need implementation, binding, signature, or numerical-parity work:", ""])
        lines.extend(f"- `{row['name']}` — **{row['status']}**: {row.get('reason', 'precision mismatch')}" for row in missing)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_summary(path: Path, summary: dict[str, Any], rows: list[dict[str, Any]]) -> None:
    lines = ["# Full TA-Lib comparison summary", "", "| Metric | Value |", "|---|---:|"]
    lines.extend(f"| {key} | {value} |" for key, value in summary.items())
    lines.extend(["", "## Slowest finkit rows", "", "| Function | Speedup | Max abs diff |", "|---|---:|---:|"])
    ranked = sorted((row for row in rows if "speedup" in row), key=lambda row: row["speedup"])[:20]
    lines.extend(f"| `{row['name']}` | {row['speedup']:.2f}x | {row['max_abs_diff']:.2e} |" for row in ranked)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scale", type=parse_scale, default=10_000, help="bars: 10K, 100K, 1M (default: 10K)")
    parser.add_argument("--repeat", type=int, default=7, help="timed samples per function")
    parser.add_argument("--warmup", type=int, default=2, help="warm-up calls per function")
    parser.add_argument("--seed", type=int, default=42, help="random seed")
    parser.add_argument("--output", default="dist/bench/talib-v064", help="output directory")
    parser.add_argument("--strict", action="store_true", help="return non-zero for missing/error/precision-fail rows")
    args = parser.parse_args()
    if args.repeat < 1 or args.warmup < 0:
        parser.error("repeat must be positive and warmup must not be negative")
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
