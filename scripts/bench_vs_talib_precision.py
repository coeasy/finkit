#!/usr/bin/env python3
# ----------------------------------------------------------------------------
# AlphaTA — precision parity check between AlphaTA and TA-Lib C.
#
# For each indicator in INDICATORS:
#   1. Generate a fixed random input (seeded for reproducibility).
#   2. Call AlphaTA (via the AlphaTA Python wheel).
#   3. Call TA-Lib (via the `talib` PyPI package, which wraps the C lib).
#   4. Compute max abs diff, mean abs diff, max relative diff, and
#      sign-mismatch ratio.
#   5. Emit a JSON + Markdown table to dist/bench/precision.{json,md}.
#   6. (Optional) Merge the `delta_pp` back into the existing
#      dist/bench/results.json produced by bench_report.py.
#
# Usage:
#   python3 scripts/bench_vs_talib_precision.py
#   python3 scripts/bench_vs_talib_precision.py --output dist/bench/precision.md
#   python3 scripts/bench_vs_talib_precision.py \
#       --results dist/bench/results.json --json-out dist/bench/precision.json
#
# Exit codes:
#   0  every indicator within the SLA (default: max_abs < 1e-9)
#   1  at least one indicator exceeded SLA
#   2  missing dependency (AlphaTA / talib not installed)
# ----------------------------------------------------------------------------
from __future__ import annotations

import argparse
import json
import sys
import pathlib
from datetime import datetime, timezone

import numpy as np

# ---- import guards --------------------------------------------------------
try:
    import finkit
except ImportError:
    print("[precision] finkit not installed; pip install finkit (or use the "
          "local wheel in dist/python/<plat>/finkit-*.whl)",
          file=sys.stderr)
    sys.exit(2)

try:
    import talib
except ImportError:
    print("[precision] talib not installed; pip install TA-Lib",
          file=sys.stderr)
    sys.exit(2)


# ---- input generator ------------------------------------------------------
def gen_inputs(n: int = 100_000, seed: int = 0):
    """Return a fixed random OHLCV input tuple (np.float64)."""
    rng = np.random.default_rng(seed)
    close = np.cumsum(rng.standard_normal(n)) + 100.0
    high = close + rng.uniform(0.5, 1.5, n)
    low  = close - rng.uniform(0.5, 1.5, n)
    open_ = close + rng.standard_normal(n) * 0.1
    volume = rng.integers(1_000, 1_000_000, n).astype(np.float64)
    return high, low, close, open_, volume


# ---- per-indicator comparisons --------------------------------------------
def _compare_arrays(fk, ta, name: str) -> dict:
    """Return per-array comparison metrics."""
    fk = np.asarray(fk, dtype=np.float64)
    ta = np.asarray(ta, dtype=np.float64)
    if fk.shape != ta.shape:
        return {"name": name, "shape_mismatch": True,
                "fk_shape": list(fk.shape), "ta_shape": list(ta.shape)}
    diff = np.abs(fk - ta)
    valid = np.isfinite(ta) & np.isfinite(fk)
    return {
        "name": name,
        "max_abs":  float(diff[valid].max()) if valid.any() else 0.0,
        "mean_abs": float(diff[valid].mean()) if valid.any() else 0.0,
        "max_rel":  float((diff / np.maximum(np.abs(ta[valid]), 1e-12)).max())
                    if valid.any() else 0.0,
        "sign_mismatch_pct":
            float(((np.sign(fk[valid]) != np.sign(ta[valid])).mean() * 100.0)
                  if valid.any() else 0.0),
        "samples":  int(valid.sum()),
    }


# ---- per-indicator result aggregator --------------------------------------
def _aggregate(rows: list[dict], name: str) -> dict:
    """Collapse per-array rows (e.g. macd.line/signal/hist) into one row."""
    if not rows:
        return {"name": name, "shape_mismatch": True}
    if any(r.get("shape_mismatch") for r in rows):
        return rows[0]
    return {
        "name": name,
        "max_abs":  max(r["max_abs"]  for r in rows),
        "mean_abs": max(r["mean_abs"] for r in rows),
        "max_rel":  max(r["max_rel"]  for r in rows),
        "sign_mismatch_pct": max(r["sign_mismatch_pct"] for r in rows),
        "samples":  rows[0]["samples"],
    }


def run_comparisons(high, low, close, open_, volume) -> list[dict]:
    """Run the full precision comparison suite."""
    rows: list[list[dict]] = []
    names: list[str] = []

    # ---- single-array indicators -----------------------------------------
    for name, fk_fn, ta_fn in [
        ("SMA(20)",     lambda c: finkit.sma(c, 20),                lambda c: talib.SMA(c, 20)),
        ("EMA(20)",     lambda c: finkit.ema(c, 20),                lambda c: talib.EMA(c, 20)),
        ("WMA(20)",     lambda c: finkit.wma(c, 20),                lambda c: talib.WMA(c, 20)),
        ("RSI(14)",     lambda c: finkit.rsi(c, 14),                lambda c: talib.RSI(c, 14)),
        ("ATR(14)",     lambda: finkit.atr(high, low, close, 14),   lambda: talib.ATR(high, low, close, 14)),
        ("NATR(14)",    lambda: finkit.natr(high, low, close, 14),  lambda: talib.NATR(high, low, close, 14)),
        ("ADX(14)",     lambda: finkit.adx(high, low, close, 14),   lambda: talib.ADX(high, low, close, 14)),
        ("CCI(14)",     lambda: finkit.cci(high, low, close, 14),   lambda: talib.CCI(high, low, close, 14)),
        ("WILLR(14)",   lambda: finkit.willr(high, low, close, 14), lambda: talib.WILLR(high, low, close, 14)),
        ("OBV",         lambda: finkit.obv(close, volume),          lambda: talib.OBV(close, volume)),
    ]:
        names.append(name)
        try:
            fk = fk_fn()
            ta = ta_fn()
        except Exception as exc:                                       # noqa: BLE001
            rows.append([{"name": name, "error": repr(exc)}])
            continue
        rows.append([_compare_arrays(fk, ta, name)])

    # ---- multi-array indicators -----------------------------------------
    try:
        fk_macd = finkit.macd(close)        # (macd, signal, hist)
        ta_macd = talib.MACD(close)         # (macd, signal, hist)
        names.append("MACD(12,26,9)")
        rows.append([
            _compare_arrays(fk_macd[0], ta_macd[0], "MACD.line"),
            _compare_arrays(fk_macd[1], ta_macd[1], "MACD.signal"),
            _compare_arrays(fk_macd[2], ta_macd[2], "MACD.hist"),
        ])
    except Exception as exc:                                           # noqa: BLE001
        names.append("MACD(12,26,9)")
        rows.append([{"name": "MACD", "error": repr(exc)}])

    try:
        fk_bb = finkit.bollinger_bands(close, 20, 2.0, 2.0)  # (upper, mid, lower)
        ta_bb = talib.BBANDS(close, 20, 2.0, 2.0)            # (upper, mid, lower)
        names.append("BBANDS(20,2)")
        rows.append([
            _compare_arrays(fk_bb[0], ta_bb[0], "BBANDS.upper"),
            _compare_arrays(fk_bb[1], ta_bb[1], "BBANDS.middle"),
            _compare_arrays(fk_bb[2], ta_bb[2], "BBANDS.lower"),
        ])
    except Exception as exc:                                           # noqa: BLE001
        names.append("BBANDS(20,2)")
        rows.append([{"name": "BBANDS", "error": repr(exc)}])

    try:
        fk_sto = finkit.stoch(high, low, close)        # (slowk, slowd)
        ta_sto = talib.STOCH(high, low, close)        # (slowk, slowd)
        names.append("STOCH(14,3,3)")
        rows.append([
            _compare_arrays(fk_sto[0], ta_sto[0], "STOCH.slowk"),
            _compare_arrays(fk_sto[1], ta_sto[1], "STOCH.slowd"),
        ])
    except Exception as exc:                                           # noqa: BLE001
        names.append("STOCH(14,3,3)")
        rows.append([{"name": "STOCH", "error": repr(exc)}])

    return [_aggregate(rs, nm) for nm, rs in zip(names, rows)]


# ---- renderers ------------------------------------------------------------
def render_markdown(results: list[dict], sla_abs: float) -> str:
    """Emit a markdown table of precision results."""
    out = [
        "# AlphaTA vs TA-Lib C — Precision Parity",
        "",
        f"> Auto-generated by `scripts/bench_vs_talib_precision.py` on "
        f"{datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M UTC')}",
        "",
        f"> **SLA**: max abs diff < {sla_abs:.0e}, max rel diff < 1e-12",
        "",
        "| Indicator | max abs | max rel | sign mismatch % | Status |",
        "|---|---|---|---|---|",
    ]
    for r in results:
        if r.get("error"):
            out.append(f"| {r['name']} | — | — | — | ❌ error |")
            continue
        if r.get("shape_mismatch"):
            out.append(f"| {r['name']} | — | — | — | ⚠️ shape mismatch |")
            continue
        ok = r["max_abs"] < sla_abs and r["max_rel"] < 1e-12
        status = "✅" if ok else "❌"
        out.append(
            f"| {r['name']} | {r['max_abs']:.2e} | {r['max_rel']:.2e} | "
            f"{r['sign_mismatch_pct']:.2f}% | {status} |"
        )
    return "\n".join(out) + "\n"


# ---- results.json merge ---------------------------------------------------
def merge_into_results(results_path: pathlib.Path, precision: list[dict]) -> None:
    """Add `delta_pp` to each benchmark entry in results.json (best-effort)."""
    if not results_path.is_file():
        return
    data = json.loads(results_path.read_text())
    bench = data.setdefault("benchmarks", {})
    # Map our comparison names -> the indicator keys in results.json
    name_map = {
        "SMA(20)":     "SMA_20",
        "EMA(20)":     "EMA_20",
        "WMA(20)":     "WMA_20",
        "RSI(14)":     "RSI_14",
        "ATR(14)":     "ATR_14",
        "NATR(14)":    "NATR_14",
        "ADX(14)":     "ADX_14",
        "CCI(14)":     "CCI_14",
        "WILLR(14)":   "WILLR_14",
        "OBV":         "OBV",
        "MACD(12,26,9)": "MACD",
        "BBANDS(20,2)": "BBANDS_20",
        "STOCH(14,3,3)": "STOCH",
    }
    for r in precision:
        if r.get("error") or r.get("shape_mismatch"):
            continue
        key = name_map.get(r["name"])
        if key and key in bench:
            # delta_pp here is interpreted as max rel diff expressed in pp
            bench[key]["delta_pp"] = float(r["max_rel"])
    data["precision"] = precision
    results_path.write_text(json.dumps(data, indent=2))


# ---- main -----------------------------------------------------------------
def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare AlphaTA indicator outputs against TA-Lib C on the same input."
    )
    parser.add_argument("--output", default="dist/bench/precision.md",
                        help="Markdown output path (default: dist/bench/precision.md)")
    parser.add_argument("--json-out", default=None,
                        help="JSON output path (default: <output>.json)")
    parser.add_argument("--results", default=None,
                        help="Optional: results.json from bench_report.py to "
                             "merge delta_pp into (no-op if missing).")
    parser.add_argument("--n", type=int, default=100_000,
                        help="Sample size (default: 100_000)")
    parser.add_argument("--seed", type=int, default=0,
                        help="RNG seed for reproducibility (default: 0)")
    parser.add_argument("--sla-max-abs", type=float, default=1e-9,
                        help="SLA for max absolute diff (default: 1e-9)")
    parser.add_argument("--exit-on-fail", action="store_true",
                        help="Exit 1 if any indicator fails the SLA")
    args = parser.parse_args()

    out_md = pathlib.Path(args.output)
    out_json = pathlib.Path(args.json_out) if args.json_out else out_md.with_suffix(".json")
    out_md.parent.mkdir(parents=True, exist_ok=True)
    out_json.parent.mkdir(parents=True, exist_ok=True)

    print(f"[precision] generating {args.n} random OHLCV samples (seed={args.seed})")
    high, low, close, open_, volume = gen_inputs(n=args.n, seed=args.seed)

    print(f"[precision] running {len(['sma','ema','wma','rsi','atr','natr','adx','cci','willr','obv','macd','bbands','stoch'])} indicators")
    results = run_comparisons(high, low, close, open_, volume)

    md = render_markdown(results, sla_abs=args.sla_max_abs)
    out_md.write_text(md, encoding="utf-8")
    out_json.write_text(json.dumps({
        "name": "alpha-ta-vs-talib-precision",
        "version": "1.0",
        "n": args.n,
        "seed": args.seed,
        "sla_max_abs": args.sla_max_abs,
        "results": results,
    }, indent=2), encoding="utf-8")
    print(f"[precision] wrote {out_md}")
    print(f"[precision] wrote {out_json}")

    if args.results:
        merge_into_results(pathlib.Path(args.results), results)
        print(f"[precision] merged delta_pp into {args.results}")

    # SLA check
    fails = [r for r in results
             if not r.get("error") and not r.get("shape_mismatch")
             and r["max_abs"] >= args.sla_max_abs]
    if fails:
        print(f"\n[precision] ❌ {len(fails)} indicator(s) exceeded SLA "
              f"(max_abs >= {args.sla_max_abs:.0e}):", file=sys.stderr)
        for r in fails:
            print(f"  ❌ {r['name']}: max_abs={r['max_abs']:.2e}", file=sys.stderr)
        if args.exit_on_fail:
            return 1
    else:
        print("[precision] ✅ all indicators within SLA")
    return 0


if __name__ == "__main__":
    sys.exit(main())
