#!/usr/bin/env python3
"""Generate the four-terminal formula dialect coverage contract.

The point of this contract is to turn "finkit covers 通达信/同花顺/大智慧/Pine"
from a slogan into a machine-checkable table: one row per (terminal, function)
with a five-state status, and a coverage percentage derived from those rows.

## Where the reference lists come from

Every reference name is traceable to a named source, and each terminal records
that source in its ``reference`` field. Nothing here is guessed: a name only
enters a reference list if it appears in

* ``repo:talib_catalog``   -- ``ffi/ffi-common/src/talib_catalog.rs`` (the
  classic TA core shared by all four terminals' function tables), or
* ``repo:functions_legacy`` -- a name the repository itself registers, or
* ``repo:grammar``         -- ``docs/formula/grammar.md`` §平台方言差异, or
* ``vendor:pine_reference`` -- the official TradingView Pine v6 reference
  (``ta.*`` / ``math.*``), mapped to canonical names through
  ``core/src/formula/pine/builtin_table.rs``.

## The five states split into two groups

Registration alone would report ~100% coverage for every terminal, which is
exactly the kind of number this project has been burned by before (see
``.workbuddy-ai/memory/TRAPS.md`` §"被作废的数字会继续传播"). The states
therefore answer two different questions:

* **routed** -- ``exact``/``near``/``approximate``. The runtime registers the
  name, so a formula calling it executes. ``approximate`` additionally flags
  future-data/repaint semantics that need explicit review (``BACKSET`` is the
  only such name we actually route).
* **not routed** -- ``host_required``/``unsupported``. The runtime does *not*
  register the name, so a formula calling it cannot run. ``host_required``
  records the *reason*: the vendor function only means something once the host
  injects market/session data (``DYNAINFO``, ``FINANCE``, ``WINNER``, ``COST``,
  ``CAPITAL``, the 大智慧 block/money-flow family, ``SECURITY``, and the
  "since listing" / open-interest families).

``host_required`` is therefore **not** a claim that we route the name. This
mirrors ``CompatibilityStatus`` in ``core/src/formula/compat.rs``, which
assigns ``HostRequired`` *inside* its ``unknown_functions`` branch and only
reaches ``Approximate`` for a name the runtime already knows.

The invariant ``routed <=> registered`` is enforced by
``core/tests/formula_dialect_coverage.rs``, so a row cannot silently claim
coverage the engine does not have.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TALIB_CATALOG = ROOT / "ffi" / "ffi-common" / "src" / "talib_catalog.rs"
COVERAGE_MATRIX = ROOT / "tests" / "contracts" / "talib_coverage_matrix_v1.json"
FORMULA_DOC = ROOT / "docs" / "generated" / "formula-functions.md"
OUT = ROOT / "tests" / "contracts" / "formula_dialect_coverage_v1.json"

SCHEMA_VERSION = 1
COLLECTED = "2026-09-23"

STATUSES = ("exact", "near", "approximate", "host_required", "unsupported")

# Registered, but only meaningful when the host injects the underlying data.
HOST_REQUIRED = {
    "DYNAINFO",
    "FINANCE",
    "WINNER",
    "LWINNER",
    "COST",
    "CAPITAL",
    "SECURITY",
    "REQUEST.SECURITY",
    "BLOCKDATA",
    "BLOCKINDEX",
    "BLOCKAVG",
    "MONEYFLOW",
    "NETINFLOW",
    "BIGORDER",
    "SMALLORDER",
    "MAININFLOW",
    "MAININFLOWPCT",
    "SUPERBIGORDER",
    "DKCOL",
    "OPENINTEREST",
    "HHVALL",
    "LLVALL",
}

# Future-data / repaint semantics. Only meaningful for names the runtime
# actually registers (``BACKSET``); ``REFX``/``FUTURE`` are unimplemented and
# are reported as ``unsupported`` instead of being credited as coverage.
APPROXIMATE = {"REFX", "BACKSET", "FUTURE"}

# --------------------------------------------------------------------------
# Provenance is *derived*, never hand-written.
# --------------------------------------------------------------------------
# An earlier revision let each terminal's reference dict carry its own
# ``repo:...`` tag by hand, and five of those tags turned out to be false
# (``IFF``/``REFX``/``SECURITY``/``EXPMEMA``/``IFNULL`` were attributed to
# ``functions_legacy.rs`` without appearing in it). A coverage contract whose
# whole promise is "every name is traceable" cannot have hand-written
# traceability, so the tag is now computed by searching these files.
#
# Only **code** counts. ``docs/formula/grammar.md`` was a source in the first
# revision and had to be removed: prose can be written from the same unreliable
# knowledge the contract is supposed to check, so mentioning a name in a doc
# promoted it to `verified`. That is a self-fulfilling loop -- documenting the
# `EXPMEMA` gap silently moved TDX coverage by 0.9 points. A registration site,
# a dispatch table or a mapping table cannot be talked into existence.
REPO_SOURCES = (
    ("repo:functions_legacy", ROOT / "core" / "src" / "formula" / "functions_legacy.rs"),
    ("repo:functions_talib_081", ROOT / "core" / "src" / "formula" / "functions_talib_081.rs"),
    ("repo:compat", ROOT / "core" / "src" / "formula" / "compat.rs"),
    ("repo:analysis", ROOT / "core" / "src" / "formula" / "analysis.rs"),
    ("repo:pine_builtin_table", ROOT / "core" / "src" / "formula" / "pine" / "builtin_table.rs"),
    ("repo:registry", ROOT / "core" / "src" / "registry.rs"),
    ("repo:talib_catalog", ROOT / "ffi" / "ffi-common" / "src" / "talib_catalog.rs"),
)

# The classic TA core is substantiated by the coverage matrix JSON itself.
CLASSIC_SOURCE = "repo:talib_coverage_matrix"
CLASSIC_PATH = ROOT / "tests" / "contracts" / "talib_coverage_matrix_v1.json"

# Pine reference names are Pine spellings, so the token that substantiates them
# is the ``ta.*`` name in the in-repo mapping table rather than the canonical
# name it resolves to.
PINE_SOURCE = ("repo:pine_builtin_table", ROOT / "core" / "src" / "formula" / "pine" / "builtin_table.rs")

# Vendor function lists. These are *not* files in this repository, so they can
# never make a row `verified` -- they only explain where an `attributed` name
# came from. Declaring them here keeps the tag vocabulary closed, so a typo like
# ``vendor:dzhh`` fails the gate instead of silently creating a new source.
VENDOR_SOURCES = {
    "vendor:tdx": "通达信 documented function list (quote / parameter families)",
    "vendor:ths": "同花顺 documented function list",
    "vendor:dzh": "大智慧 documented function list (block / money-flow / depth families)",
    "vendor:pine_reference": "TradingView Pine v6 reference (ta.* / math.*)",
}

# --------------------------------------------------------------------------
# Terminal-specific reference additions, each with its in-repo provenance.
# --------------------------------------------------------------------------

TDX_SPECIFIC = {
    # vendor-documented parameter families (docs/formula/grammar.md §通达信特有)
    "DYNAINFO": "vendor:tdx",
    "FINANCE": "vendor:tdx",
    "WINNER": "vendor:tdx",
    "COST": "vendor:tdx",
    "CAPITAL": "vendor:tdx",
    "DKCOL": "vendor:tdx",
    # TDX time / bar family (core/src/formula/functions_legacy.rs §TIME / BAR FUNCTIONS (TDX))
    "BARSCOUNT": "repo:functions_legacy",
    "BARSLAST": "repo:functions_legacy",
    "BARSLASTCOUNT": "repo:functions_legacy",
    "BARSSINCE": "repo:functions_legacy",
    "CURRBARSCOUNT": "repo:functions_legacy",
    "HHVBARS": "repo:functions_legacy",
    "LLVBARS": "repo:functions_legacy",
    "SUMBARS": "repo:functions_legacy",
    "COUNT": "repo:functions_legacy",
    "FILTER": "repo:functions_legacy",
    "LAST": "repo:functions_legacy",
    "BACKSET": "repo:functions_legacy",
    "REFX": "repo:functions_legacy",
    "VALUEWHEN": "repo:functions_legacy",
    # TDX math / statistics extensions
    "BETWEEN": "repo:functions_legacy",
    "IFF": "repo:functions_legacy",
    "IFNULL": "repo:functions_legacy",
    "INTPART": "repo:functions_legacy",
    "RANGE": "repo:functions_legacy",
    "SIGN": "repo:functions_legacy",
    # TDX aliases registered by the repo
    "PDI": "repo:functions_legacy",
    "MDI": "repo:functions_legacy",
    "MTM": "repo:functions_legacy",
    "EXPMEMA": "repo:functions_legacy",
    # cross-period surface declared by the repo
    "PERIODTYPE": "repo:grammar",
    "REFDATE": "repo:grammar",
}

THS_SPECIFIC = {
    # docs/formula/grammar.md §同花顺（THS）历史引用别名
    "CLOSE1": "repo:grammar",
    "OPEN1": "repo:grammar",
    "HIGH1": "repo:grammar",
    "LOW1": "repo:grammar",
    "VOL1": "repo:grammar",
    # THS chip distribution + condition-selection surface
    "WINNER": "vendor:ths",
    "LWINNER": "vendor:ths",
    "COST": "vendor:ths",
    "CAPITAL": "vendor:ths",
    "DYNAINFO": "vendor:ths",
    "FINANCE": "vendor:ths",
    "DKCOL": "vendor:ths",
    # THS-registered helpers
    "VALUEWHEN": "repo:functions_legacy",
    "BARSLAST": "repo:functions_legacy",
    "HHVBARS": "repo:functions_legacy",
    "LLVBARS": "repo:functions_legacy",
    "SUMBARS": "repo:functions_legacy",
    "COUNT": "repo:functions_legacy",
    "FILTER": "repo:functions_legacy",
    "PERIODTYPE": "repo:grammar",
    "REFDATE": "repo:grammar",
    "SECURITY": "repo:grammar",
}

DZH_SPECIFIC = {
    # docs/formula/grammar.md §大智慧（DZH）特有语法
    "BLOCKINDEX": "vendor:dzh",
    "BLOCKAVG": "vendor:dzh",
    "BLOCKDATA": "vendor:dzh",
    "DYNAINFO": "vendor:dzh",
    # DZH block / money-flow family (core/src/formula/functions_legacy.rs
    # §DZH BLOCK FUNCTIONS, §DZH MONEY FLOW FUNCTIONS)
    "MONEYFLOW": "repo:functions_legacy",
    "NETINFLOW": "repo:functions_legacy",
    "BIGORDER": "repo:functions_legacy",
    "SMALLORDER": "repo:functions_legacy",
    "MAININFLOW": "repo:functions_legacy",
    "MAININFLOWPCT": "repo:functions_legacy",
    "SUPERBIGORDER": "repo:functions_legacy",
    # DZH-specific helpers the repo registers
    "DKCOL": "repo:functions_legacy",
    "PERIODTYPE": "repo:grammar",
    "REFDATE": "repo:grammar",
    "VALUEWHEN": "repo:functions_legacy",
    "BARSLAST": "repo:functions_legacy",
    "COUNT": "repo:functions_legacy",
    "FILTER": "repo:functions_legacy",
    # DZH depth-of-book family (declared in the convergence plan §M2-2)
    "DDX": "vendor:dzh",
    "DDY": "vendor:dzh",
    "DDZ": "vendor:dzh",
    "SV": "vendor:dzh",
    "HHVALL": "vendor:dzh",
    "LLVALL": "vendor:dzh",
    "OPENINTEREST": "vendor:dzh",
}

# Official TradingView Pine v6 `ta.*` / `math.*` names, mapped to the canonical
# formula name through core/src/formula/pine/builtin_table.rs.
PINE_SPECIFIC = {
    "ta.alma": "SMA",
    "ta.atr": "ATR",
    "ta.bb": "BBANDS",
    "ta.bbw": "BOLLWIDTH",
    "ta.percentile_linear_interpolation": "PERCENTILE",
    "ta.percentile_nearest_rank": "PERCENTRANK",
    "ta.percentrank": "PERCENTRANK",
    "ta.change": "MOM",
    "ta.cmo": "CMO",
    "ta.cog": "COG",
    "ta.correlation": "CORREL",
    "ta.cci": "CCI",
    "ta.dev": "AVGDEV",
    "ta.dmi": "ADX",
    "ta.donchian": "DONCHIAN_MID",
    "ta.ema": "EMA",
    "ta.highest": "HHV",
    "ta.highestbars": "HHVBARS",
    "ta.hma": "HMA",
    "ta.kc": "KC",
    "ta.kcw": "BOLLWIDTH",
    "ta.linreg": "LINEARREG",
    "ta.lowest": "LLV",
    "ta.lowestbars": "LLVBARS",
    "ta.macd": "MACD",
    "ta.max": "MAX",
    "ta.median": "MEDIAN",
    "ta.mfi": "MFI",
    "ta.min": "MIN",
    "ta.mode": "MODE",
    "ta.mom": "MOM",
    "ta.pivothigh": "PIVOTHIGH",
    "ta.pivotlow": "PIVOTLOW",
    "ta.range": "TRANGE",
    "ta.rising": "RISING",
    "ta.rma": "RMA",
    "ta.roc": "ROC",
    "ta.rsi": "RSI",
    "ta.sar": "SAR",
    "ta.sma": "SMA",
    "ta.stdev": "STDDEV",
    "ta.stoch": "STOCH",
    "ta.supertrend": "SUPERTREND",
    "ta.swma": "SWMA",
    "ta.tr": "TRANGE",
    "ta.tsi": "TSI",
    "ta.valuewhen": "VALUEWHEN",
    "ta.variance": "VAR",
    "ta.vwap": "VWAP",
    "ta.vwma": "VWMA",
    "ta.wma": "WMA",
    "ta.wpr": "WILLR",
    "ta.crossover": "CROSSOVER",
    "ta.crossunder": "CROSSDOWN",
    "ta.barssince": "BARSSINCE",
    "ta.cum": "SUM",
    "ta.obv": "OBV",
    "ta.pvt": "PVT",
    "ta.wad": "WAD",
    "ta.ad": "AD",
    "ta.accdist": "AD",
    "ta.nvi": "NVI",
    "ta.pvi": "PVI",
    "ta.vol": "STDDEV",
    "ta.ao": "AO",
    "ta.ac": "AC",
    "math.abs": "ABS",
    "math.acos": "ACOS",
    "math.asin": "ASIN",
    "math.atan": "ATAN",
    "math.avg": "AVG",
    "math.ceil": "CEILING",
    "math.cos": "COS",
    "math.exp": "EXP",
    "math.floor": "FLOOR",
    "math.log": "LN",
    "math.log10": "LOG10",
    "math.max": "MAX",
    "math.min": "MIN",
    "math.pow": "POW",
    "math.round": "ROUND",
    "math.sign": "SIGN",
    "math.sin": "SIN",
    "math.sqrt": "SQRT",
    "math.sum": "SUM",
    "math.tan": "TAN",
}

TERMINALS = (
    (
        "tongdaxin",
        "tdx",
        "通达信",
        "repo:talib_coverage_matrix (numeric_reference 201) + repo:grammar §通达信特有 + "
        "core/src/formula/functions_legacy.rs §TIME / BAR FUNCTIONS (TDX)",
        TDX_SPECIFIC,
    ),
    (
        "tonghuashun",
        "ths",
        "同花顺",
        "repo:talib_coverage_matrix (numeric_reference 201) + repo:grammar §同花顺历史引用别名 + "
        "core/src/formula/functions_legacy.rs THS aliases",
        THS_SPECIFIC,
    ),
    (
        "dazhihui",
        "dzh",
        "大智慧",
        "repo:talib_coverage_matrix (numeric_reference 201) + repo:grammar §大智慧特有语法 + "
        "core/src/formula/functions_legacy.rs §DZH BLOCK / MONEY FLOW FUNCTIONS",
        DZH_SPECIFIC,
    ),
    (
        "tradingview_pine",
        "pine",
        "TradingView Pine",
        "TradingView Pine v6 reference ta.*/math.* mapped through "
        "core/src/formula/pine/builtin_table.rs + repo:talib_coverage_matrix (numeric_reference 201)",
        PINE_SPECIFIC,
    ),
)


def read_talib_catalog() -> list[str]:
    text = TALIB_CATALOG.read_text(encoding="utf-8")
    match = re.search(
        r"pub const TALIB_PROFILE_CATALOG_NAMES: &\[&str\] = &\[(.*?)\];",
        text,
        re.S,
    )
    if match is None:
        raise SystemExit("could not locate TALIB_PROFILE_CATALOG_NAMES")
    return re.findall(r'"([A-Z0-9_]+)"', match.group(1))


def read_classic_ta_core() -> list[str]:
    """The classic TA function core every one of the four terminals ships.

    Sourced from the checked-in TA-Lib coverage matrix rather than the
    profile-only catalog: the catalog lists only names that have *no* Core
    registry equivalent, so it under-counts the shared surface by design.
    """
    payload = json.loads(COVERAGE_MATRIX.read_text(encoding="utf-8"))
    names = payload["surfaces"]["numeric_reference"]["indicators"]
    if len(names) != payload["surfaces"]["numeric_reference"]["expected_count"]:
        raise SystemExit("TA-Lib coverage matrix is internally inconsistent")
    return names


def read_formula_surface() -> set[str]:
    text = FORMULA_DOC.read_text(encoding="utf-8")
    return set(re.findall(r"^\| `([A-Z0-9_]+)` \|$", text, re.M))


_FILE_TEXT_CACHE: dict[Path, str | None] = {}


def _file_text(path: Path) -> str | None:
    if path not in _FILE_TEXT_CACHE:
        _FILE_TEXT_CACHE[path] = path.read_text(encoding="utf-8") if path.is_file() else None
    return _FILE_TEXT_CACHE[path]


def _substantiated_by(name: str, path: Path) -> bool:
    text = _file_text(path)
    if text is None:
        return False
    return re.search(r"\b" + re.escape(name) + r"\b", text) is not None


def resolve_source(
    name: str,
    classic: set[str],
    attribution: str | None,
    spellings: tuple[str, ...] = (),
) -> tuple[str | None, str | None, str | None]:
    """Return ``(source, provenance, via)`` for one reference name.

    ``provenance`` is ``verified`` when the name is found in a named file (the
    gate re-checks this), and ``attributed`` when the only support is a vendor
    function list that cannot be read from this repository. Names with neither
    return ``(None, None, None)`` and are dropped from the reference list --
    they are reported separately rather than quietly inflating the denominator.

    ``via`` records the token that substantiated the row when it differs from
    the canonical name (a Pine ``ta.*`` spelling, say), so the provenance stays
    auditable instead of being a bare tag.
    """
    if name in classic:
        return CLASSIC_SOURCE, "verified", None
    for tag, path in REPO_SOURCES:
        if _substantiated_by(name, path):
            return tag, "verified", None
    # A Pine reference name is substantiated by its Pine spelling appearing in
    # the in-repo mapping table; the canonical name may never appear there.
    if spellings:
        pine_tag, pine_path = PINE_SOURCE
        for spelling in spellings:
            if _substantiated_by(spelling, pine_path):
                return pine_tag, "verified", spelling
    # A hand-written ``repo:`` tag is never trusted as a fallback: it must be
    # substantiated above or it does not count.
    if attribution is not None and attribution.startswith("vendor:"):
        # Keep the vendor spelling on the row so an attributed name stays
        # auditable even though this repository cannot substantiate it.
        return attribution, "attributed", (spellings[0] if spellings else None)
    return None, None, None


def classify(name: str, runtime: set[str]) -> str:
    """Status for one ``(terminal, function)`` row.

    Order matters, and the original version got it wrong in both directions:

    * Checking ``HOST_REQUIRED`` first credited ``host_required`` to names the
      runtime *does* register (``DYNAINFO`` and friends) -- understating
      coverage, but for the wrong reason.
    * Checking ``APPROXIMATE`` before registration credited ``REFX``/``FUTURE``
      as *routed* coverage even though nothing registers them -- overstating it.

    The two facts are independent, so they are recorded independently: the
    status says whether the name is *useful out of the box*, and the separate
    ``registered`` field says whether the engine registers it at all.
    """
    # A host-dependent name is not out-of-the-box coverage whether or not the
    # engine happens to register a stub for it (the stub returns all-NaN).
    if name in HOST_REQUIRED:
        return "host_required"
    if name not in runtime:
        return "unsupported"
    # Registered and produces values from series data alone.
    return "approximate" if name in APPROXIMATE else "near"


def build() -> dict:
    runtime = read_formula_surface()
    if not runtime:
        raise SystemExit("formula surface is empty; regenerate docs first")
    classic = read_classic_ta_core()

    terminals = {}
    unverified: dict[str, list[str]] = {}
    for key, short, label, reference, specific in TERMINALS:
        if key == "tradingview_pine":
            # Pine reference names are Pine spellings; the contract records the
            # canonical name the mapping resolves to, so all four terminals share
            # one namespace and can be compared directly.
            reference_names = {canonical for canonical in specific.values()}
            reference_names.update(classic)
            attribution_of = {}
            spellings_of: dict[str, list[str]] = {}
            for pine_name, canonical in specific.items():
                attribution_of.setdefault(canonical, "vendor:pine_reference")
                spellings_of.setdefault(canonical, []).append(pine_name)
        else:
            reference_names = set(classic)
            reference_names.update(specific)
            attribution_of = dict(specific)
            spellings_of = {}

        functions = {}
        dropped = []
        for name in sorted(reference_names):
            source, provenance, via = resolve_source(
                name,
                classic,
                attribution_of.get(name),
                tuple(spellings_of.get(name, ())),
            )
            if source is None:
                dropped.append(name)
                continue
            functions[name] = {
                "status": classify(name, runtime),
                # Machine fact, not a judgement call: is the name in the runtime
                # function table? The gate re-derives this from the live engine,
                # so a row cannot claim a registration that does not exist.
                "registered": name in runtime,
                "source": source,
                "provenance": provenance,
                "via": via,
            }
        if dropped:
            unverified[key] = dropped

        counts = {status: 0 for status in STATUSES}
        verified = 0
        for entry in functions.values():
            counts[entry["status"]] += 1
            if entry["provenance"] == "verified":
                verified += 1
        total = len(functions)
        registered = sum(1 for entry in functions.values() if entry["registered"])
        # "Out of the box" = the engine registers it AND it yields values from
        # series data alone. `host_required` names need the host to inject
        # market/session data, so they are deliberately excluded.
        #
        # Two denominators, because they answer different questions:
        #   `coverage_pct`          -- share of the terminal's documented surface
        #                              that runs out of the box. This is the
        #                              headline a user comparing platforms wants.
        #   `verified_coverage_pct` -- the same share restricted to rows whose
        #                              reference name could be substantiated in
        #                              this repository. A large gap between the
        #                              two means the *reference list* is weakly
        #                              sourced, not that coverage is better.
        out_of_the_box = sum(
            1
            for entry in functions.values()
            if entry["status"] in ("exact", "near", "approximate")
        )
        out_of_the_box_verified = sum(
            1
            for entry in functions.values()
            if entry["provenance"] == "verified"
            and entry["status"] in ("exact", "near", "approximate")
        )
        terminals[key] = {
            "id": short,
            "label": label,
            "reference": reference,
            "collected": COLLECTED,
            "reference_size": total,
            "verified_size": verified,
            "attributed_size": total - verified,
            "functions": functions,
            "coverage": counts,
            "coverage_pct": round(100.0 * out_of_the_box / total, 1) if total else 0.0,
            "verified_coverage_pct": (
                round(100.0 * out_of_the_box_verified / verified, 1) if verified else 0.0
            ),
            "registered": registered,
            "host_dependent_registered": sorted(
                name
                for name, entry in functions.items()
                if entry["status"] == "host_required" and entry["registered"]
            ),
        }

    return {
        "schema_version": SCHEMA_VERSION,
        "generated_by": "scripts/gen_dialect_coverage.py",
        "collected": COLLECTED,
        "statuses": list(STATUSES),
        "runtime_formula_surface": len(runtime),
        "provenance_sources": {
            **{tag: str(path.relative_to(ROOT)).replace("\\", "/") for tag, path in REPO_SOURCES},
            CLASSIC_SOURCE: str(CLASSIC_PATH.relative_to(ROOT)).replace("\\", "/"),
        },
        "vendor_sources": dict(VENDOR_SOURCES),
        "notes": (
            "Each row carries independent facts. `status` says whether the name "
            "is useful out of the box: `exact`/`near` route through the "
            "canonical AlphaTA runtime (a documented common-subset mapping, not "
            "a claim of terminal-identical numerics), `approximate` routes but "
            "flags future-data semantics, `host_required` only means something "
            "once the host injects market/session data, and `unsupported` is "
            "not implemented. `registered` says whether the engine's function "
            "table contains the name at all. `provenance` says whether the "
            "reference name was *verified* in a named in-repo file or merely "
            "*attributed* to a vendor function list; `coverage_pct` is computed "
            "over verified rows only, so it excludes `host_required` even when "
            "a NaN-returning stub is registered (see "
            "`host_dependent_registered`). Names with no support in this "
            "repository are listed in `unverified_candidates` instead of being "
            "counted."
        ),
        "terminals": terminals,
        "unverified_candidates": unverified,
    }


def render(payload: dict) -> str:
    return json.dumps(payload, ensure_ascii=False, indent=2, sort_keys=False) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--generate", action="store_true")
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--summary", action="store_true")
    args = parser.parse_args()

    payload = build()
    rendered = render(payload)

    if args.summary:
        for key, data in payload["terminals"].items():
            print(
                f"{data['id']:6s} {data['reference_size']:4d} refs "
                f"({data['verified_size']:4d} verified, {data['attributed_size']:3d} attributed)"
                f"  out-of-the-box {data['coverage_pct']:5.1f}%"
                f" (verified-only {data['verified_coverage_pct']:5.1f}%)"
                f"  registered {data['registered']:4d}  {data['coverage']}"
            )
            if data["host_dependent_registered"]:
                print(
                    "       registered-but-host-dependent: "
                    + ", ".join(data["host_dependent_registered"])
                )
        if payload["unverified_candidates"]:
            print("\nDropped (no support in this repository):")
            for key, names in payload["unverified_candidates"].items():
                print(f"  {key}: {', '.join(names)}")

    if args.generate:
        OUT.write_text(rendered, encoding="utf-8", newline="\n")
        print(f"Wrote {OUT.relative_to(ROOT)}")
        return 0

    if args.check:
        if not OUT.is_file():
            print(f"FAILED: {OUT.relative_to(ROOT)} is missing", file=sys.stderr)
            return 1
        if OUT.read_text(encoding="utf-8") != rendered:
            print(
                f"FAILED: {OUT.relative_to(ROOT)} is out of date.\n"
                "Run: python scripts/gen_dialect_coverage.py --generate",
                file=sys.stderr,
            )
            return 1
        print("OK: dialect coverage contract matches its generator")
        return 0

    if not args.summary:
        parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
