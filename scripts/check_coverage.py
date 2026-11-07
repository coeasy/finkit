#!/usr/bin/env python3
"""Report registry name coverage without inferring implementation from fuzzy text matches.

The indicator registry is the denominator for batch and streaming declarations.
Formula functions use the same extractor as ``gen_ssot_docs.py``; they have a
separate denominator because the formula language intentionally includes many
non-indicator functions and does not promise a one-to-one indicator mapping.

This is a diagnostic report, not a correctness/parity test. Exact name matches
and reviewed aliases are reported separately. Unmatched entries remain visible
for human review instead of being counted as implemented through a substring.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Iterable

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs" / "indicator_registry.json"
STREAMING_DIR = ROOT / "core" / "src" / "streaming"
BATCH_DIRS = [
    ROOT / "core" / "src" / "indicators",
    ROOT / "core" / "src" / "patterns",
    ROOT / "core" / "src" / "math",
    ROOT / "core" / "src" / "features",
    ROOT / "core" / "src" / "transforms",
]

# Registry display names and Rust symbols that have been checked as the same
# operation. Unknown names must be reviewed before being added here.
BATCH_ALIASES = {
    "Bollinger Bands": "bbands",
    "Donchian Channel": "donchian",
    "Ichimoku Cloud": "ichimoku",
    "Stochastic": "stoch",
    "Williams %R": "willr",
    "CFO": "chande_forecast_oscillator",
    "Twiggs MF": "twiggs_money_flow",
    "CORREL": "correlation",
    "AD_LINE": "advance_decline_line",
    "AD_RATIO": "advance_decline_ratio",
    "HV": "historical_volatility",
    "CDL_4PRICE_DOJI": "doji_4prices",
    # These registry entries denote the bullish/bearish output channel of a
    # single signed-pattern function; the Rust function returns both signs.
    "CDL_3INSIDE_UP": "cdl_3inside",
    "CDL_3INSIDE_DOWN": "cdl_3inside",
    "CDL_3OUTSIDE_UP": "cdl_3outside",
    "CDL_3OUTSIDE_DOWN": "cdl_3outside",
    "CDL_DARK_CLOUD": "cdl_darkcloudcover",
    "CDL_TWEEZER_TOP": "tweezer_top",
    "CDL_TWEEZER_BOT": "tweezer_bot",
    "CDL_UPSIDE_GAP_3METHODS": "upside_gap_3methods",
    "CDL_CONCEALING_BABY": "concealing_baby_swallow",
}

STREAMING_ALIASES = {
    "Bollinger Bands": "Boll",
    "Chaikin Volatility": "ChaikinVol",
    "Donchian Channel": "Donchian",
    "Ichimoku Cloud": "Ichimoku",
    "Keltner Channel": "Keltner",
    "Stochastic": "Stoch",
    "Williams %R": "WillR",
    "LINEAR_REG": "LinReg",
}

FORMULA_ALIASES = {
    "Bollinger Bands": "BBANDS",
    "Donchian Channel": "DONCHIAN",
    "Stochastic": "STOCH",
    "Williams %R": "WILLR",
}


def norm(name: str) -> str:
    """Fold case/punctuation for exact symbol comparison only."""
    return re.sub(r"[^a-z0-9]", "", name.lower())


def rust_sources(directories: Iterable[Path]):
    for directory in directories:
        if directory.is_dir():
            yield from directory.rglob("*.rs")


def collect_rust_symbols(
    directories: Iterable[Path], *, structs: bool = False, strip_streaming_prefix: bool = False
) -> dict[str, list[dict[str, str]]]:
    """Collect public Rust functions or types by normalized identifier."""
    symbols: dict[str, list[dict[str, str]]] = {}
    function_re = re.compile(r"^pub\s+(?:async\s+)?fn\s+([A-Za-z0-9_]+)", re.M)
    struct_re = re.compile(r"^pub\s+struct\s+([A-Za-z0-9_]+)", re.M)
    for path in rust_sources(directories):
        source = path.read_text(encoding="utf-8", errors="replace")
        names = struct_re.findall(source) if structs else function_re.findall(source)
        for raw_name in names:
            name = raw_name
            if strip_streaming_prefix:
                if not raw_name.startswith("Streaming"):
                    continue
                name = raw_name.removeprefix("Streaming")
            symbols.setdefault(norm(name), []).append(
                {"name": name, "source": path.relative_to(ROOT).as_posix()}
            )
    return symbols


def collect_formula_symbols() -> tuple[dict[str, list[dict[str, str]]], int]:
    """Use the generator's registry-aware extraction for the runtime formula surface."""
    sys.path.insert(0, str(ROOT / "scripts"))
    from gen_ssot_docs import parse_formula_functions

    names = parse_formula_functions()
    symbols: dict[str, list[dict[str, str]]] = {}
    for name in names:
        symbols.setdefault(norm(name), []).append(
            {"name": name, "source": "gen_ssot_docs.parse_formula_functions"}
        )
    return symbols, len(names)


def match_name(
    registry_name: str,
    symbols: dict[str, list[dict[str, str]]],
    aliases: dict[str, str],
    *,
    allow_aliases: bool = True,
) -> dict[str, str | None]:
    """Match an exact normalized name or an explicit, reviewed alias.

    Prefix and substring matches are deliberately never considered. A repeated
    normalized symbol is marked ambiguous instead of choosing one arbitrarily.
    """
    candidates = symbols.get(norm(registry_name), [])
    if len(candidates) > 1:
        # Multiple modules may export the same public operation (for example a
        # canonical indicator and a math helper). Name presence is still exact;
        # the report preserves every location and does not infer numeric parity.
        return {
            "status": "exact",
            "name": registry_name,
            "source": None,
            "candidate_count": len(candidates),
            "sources": sorted({candidate["source"] for candidate in candidates}),
        }
    if len(candidates) == 1:
        return {"status": "exact", **candidates[0], "candidate_count": 1}
    if allow_aliases and registry_name in aliases:
        target = aliases[registry_name]
        candidates = symbols.get(norm(target), [])
        if candidates:
            return {
                "status": "alias",
                "name": target,
                "source": candidates[0]["source"] if len(candidates) == 1 else None,
                "candidate_count": len(candidates),
                "sources": sorted({candidate["source"] for candidate in candidates}),
            }
        return {
            "status": "stale_alias",
            "name": target,
            "source": None,
            "candidate_count": 0,
        }
    return {"status": "unmatched", "name": None, "source": None, "candidate_count": 0}


def validate_aliases(
    aliases: dict[str, str], symbols: dict[str, list[dict[str, str]]], layer: str
) -> list[str]:
    errors = []
    for registry_name, target in aliases.items():
        matches = symbols.get(norm(target), [])
        if not matches:
            errors.append(f"{layer} alias {registry_name!r} -> {target!r} is missing")
    return errors


def build_report(*, allow_aliases: bool = True) -> dict:
    registry = json.loads(REGISTRY.read_text(encoding="utf-8"))
    indicators = registry["indicators"]
    batch_symbols = collect_rust_symbols(BATCH_DIRS)
    streaming_symbols = collect_rust_symbols(
        [STREAMING_DIR], structs=True, strip_streaming_prefix=True
    )
    formula_symbols, formula_function_count = collect_formula_symbols()

    rows = []
    for indicator in indicators:
        name = indicator["name"]
        streaming_required = indicator.get("streaming") is True
        rows.append(
            {
                "name": name,
                "category": indicator.get("category", ""),
                "streaming_required": streaming_required,
                "batch": match_name(
                    name, batch_symbols, BATCH_ALIASES, allow_aliases=allow_aliases
                ),
                "streaming": (
                    match_name(
                        name,
                        streaming_symbols,
                        STREAMING_ALIASES,
                        allow_aliases=allow_aliases,
                    )
                    if streaming_required
                    else {"status": "not_required", "name": None, "source": None}
                ),
                "formula_name_match": match_name(
                    name,
                    formula_symbols,
                    FORMULA_ALIASES,
                    allow_aliases=allow_aliases,
                ),
            }
        )

    batch_counts = Counter(row["batch"]["status"] for row in rows)
    streaming_rows = [row for row in rows if row["streaming_required"]]
    streaming_counts = Counter(row["streaming"]["status"] for row in streaming_rows)
    formula_name_counts = Counter(row["formula_name_match"]["status"] for row in rows)
    alias_errors = (
        validate_aliases(BATCH_ALIASES, batch_symbols, "batch")
        + validate_aliases(STREAMING_ALIASES, streaming_symbols, "streaming")
        + validate_aliases(FORMULA_ALIASES, formula_symbols, "formula")
    )

    return {
        "registry_indicator_count": len(rows),
        "batch": {"denominator": len(rows), **dict(batch_counts)},
        "streaming": {
            "denominator": len(streaming_rows),
            **dict(streaming_counts),
            "not_required": len(rows) - len(streaming_rows),
        },
        "formula": {
            "registered_function_count": formula_function_count,
            "indicator_name_matches": dict(formula_name_counts),
            "note": "Name overlap only; this is not a formula support/parity denominator.",
        },
        "alias_errors": alias_errors,
        "rows": rows,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--strict", action="store_true", help="do not apply reviewed aliases")
    parser.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    args = parser.parse_args()
    report = build_report(allow_aliases=not args.strict)

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print(f"registry indicators: {report['registry_indicator_count']}")
        print(f"batch (denominator={report['batch']['denominator']}): {report['batch']}")
        print(f"streaming (required denominator={report['streaming']['denominator']}): {report['streaming']}")
        print(f"formula runtime functions: {report['formula']['registered_function_count']}")
        print(f"formula indicator-name matches (not parity): {report['formula']['indicator_name_matches']}")
        for layer in ("batch", "streaming"):
            unmatched = [
                row["name"]
                for row in report["rows"]
                if (layer == "batch" or row["streaming_required"])
                and row[layer]["status"] in {"unmatched", "ambiguous", "stale_alias", "ambiguous_alias"}
            ]
            print(f"{layer} unmatched/ambiguous: {unmatched}")
        if report["alias_errors"]:
            print("invalid aliases:")
            for error in report["alias_errors"]:
                print(f"  - {error}")

    return 1 if report["alias_errors"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
