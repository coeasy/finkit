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

# Official TradingView Pine v6 `ta.*` / `math.*` names we survey.
#
# This is the *reference spelling list* only. What each spelling resolves to is
# deliberately NOT written here: it is read from the engine by
# `pine_engine_mapping()`, because a hand-maintained mirror of the engine's
# mapping drifts. An earlier version of this file carried such a mirror (86
# entries); it disagreed with the engine on 7 canonical names and credited 43
# spellings the engine cannot resolve at all as `near` (out-of-the-box).
#
# Spellings the engine cannot map stay in the contract as `unsupported` rows
# keyed by the spelling itself, so the gap stays visible instead of being
# dropped. Under-claiming is safe here; over-claiming is the defect being
# repaired.
PINE_SPELLINGS = (
    "math.abs",
    "math.acos",
    "math.asin",
    "math.atan",
    "math.avg",
    "math.ceil",
    "math.cos",
    "math.exp",
    "math.floor",
    "math.log",
    "math.log10",
    "math.max",
    "math.min",
    "math.pow",
    "math.round",
    "math.sign",
    "math.sin",
    "math.sqrt",
    "math.sum",
    "math.tan",
    "ta.ac",
    "ta.accdist",
    "ta.ad",
    "ta.alma",
    "ta.ao",
    "ta.atr",
    "ta.barssince",
    "ta.bb",
    "ta.bbw",
    "ta.cci",
    "ta.change",
    "ta.cmo",
    "ta.cog",
    "ta.correlation",
    "ta.crossover",
    "ta.crossunder",
    "ta.cum",
    "ta.dev",
    "ta.dmi",
    "ta.donchian",
    "ta.ema",
    "ta.highest",
    "ta.highestbars",
    "ta.hma",
    "ta.kc",
    "ta.kcw",
    "ta.linreg",
    "ta.lowest",
    "ta.lowestbars",
    "ta.macd",
    "ta.max",
    "ta.median",
    "ta.mfi",
    "ta.min",
    "ta.mode",
    "ta.mom",
    "ta.nvi",
    "ta.obv",
    "ta.percentile_linear_interpolation",
    "ta.percentile_nearest_rank",
    "ta.percentrank",
    "ta.pivothigh",
    "ta.pivotlow",
    "ta.pvi",
    "ta.pvt",
    "ta.range",
    "ta.rising",
    "ta.rma",
    "ta.roc",
    "ta.rsi",
    "ta.sar",
    "ta.sma",
    "ta.stdev",
    "ta.stoch",
    "ta.supertrend",
    "ta.swma",
    "ta.tr",
    "ta.tsi",
    "ta.valuewhen",
    "ta.variance",
    "ta.vol",
    "ta.vwap",
    "ta.vwma",
    "ta.wad",
    "ta.wma",
    "ta.wpr",
)

# `core/src/formula/pine/ast_mapper.rs` resolves a handful of `ta.*` names
# before consulting the builtin table, because they need argument normalisation
# (Pine's compact signatures vs. AlphaTA's OHLCV-expanded ones). That is Rust
# code rather than a data table, so the list is written out here -- and *proved*
# by the behavioural gate `pine_engine_mapping_matches_the_engine`, which lowers
# each spelling through `map_pine_to_alphata` and compares the emitted name.
PINE_AST_OVERRIDES = {
    "ta.tr": "TRANGE",
    "ta.atr": "ATR",
    "ta.natr": "NATR",
    "ta.cci": "CCI",
    "ta.wpr": "WILLR",
    "ta.williamspercentr": "WILLR",
    "ta.vwap": "VWAP",
    "ta.obv": "OBV",
    "ta.sar": "SAR",
    "ta.stoch": "STOCHF",
    "ta.change": "MOM",
    "ta.sma": "MA",
    "ta.vwma": "VWMA",
}


def read_pine_builtin_table() -> dict[str, str]:
    """Parse `PineBuiltinTable`'s default mappings out of the Rust source.

    The table is a plain data table (`namespace` / `pine_name` /
    `alpha_ta_name`), so reading it is safe. This is the authoritative source
    for every Pine spelling the engine resolves through the generic path; the
    `ast_mapper` special cases layered on top live in [`PINE_AST_OVERRIDES`].
    """
    text = _file_text(PINE_SOURCE[1])
    if text is None:
        raise SystemExit("cannot read {}".format(PINE_SOURCE[1].relative_to(ROOT)))
    body = text.split("fn default_mappings()", 1)
    if len(body) != 2:
        raise SystemExit("builtin_table.rs no longer defines `default_mappings()`")
    namespace_re = re.compile(r'namespace:\s*(None|Some\("([a-z]+)")')
    pine_name_re = re.compile(r'pine_name:\s*"([^"]+)"')
    alpha_re = re.compile(r'alpha_ta_name:\s*"([^"]+)"')

    mapping: dict[str, str] = {}
    for chunk in body[1].split("BuiltinMapping {")[1:]:
        namespace = namespace_re.search(chunk)
        pine_name = pine_name_re.search(chunk)
        alpha = alpha_re.search(chunk)
        if not (namespace and pine_name and alpha):
            continue
        if namespace.group(1) == "None":
            key = pine_name.group(1)
        else:
            key = "{}.{}".format(namespace.group(2), pine_name.group(1))
        mapping[key] = alpha.group(1)
    if len(mapping) < 30:
        raise SystemExit(
            "parsed only {} Pine mappings; the parser has drifted from "
            "builtin_table.rs".format(len(mapping))
        )
    return mapping


def pine_fallback_name(spelling: str) -> str:
    """The name `ast_mapper.rs` emits for a spelling the table does not resolve.

    `ast_mapper.rs` falls through to `format!("{}_{}", ns.to_uppercase(),
    name.to_uppercase())` (or the bare uppercase name when there is no
    namespace). Most of these are dead names the runtime never registers -- but
    not all: `math.avg` -> `MATH_AVG` really does exist. Recording the fallback
    rather than assuming it is dead is what keeps `math.avg` from being
    mislabelled.
    """
    if "." in spelling:
        namespace, name = spelling.split(".", 1)
        return "{}_{}".format(namespace.upper(), name.upper())
    return spelling.upper()


def pine_engine_mapping() -> dict[str, str]:
    """Spelling -> the canonical name `map_pine_to_alphata` actually emits.

    Complete by construction: a spelling the builtin table does not resolve
    still resolves to *something*, because `ast_mapper.rs` falls through to
    [`pine_fallback_name`]. That keeps one rule for every row -- the row key is
    the name the engine emits -- so `registered` and `status` stay consistent
    with the row key instead of needing a special case.

    `ast_mapper.rs` checks its `ta.*` special cases *before* consulting the
    table, so the overrides win.
    """
    mapping = read_pine_builtin_table()
    mapping.update(PINE_AST_OVERRIDES)
    for spelling in PINE_SPELLINGS:
        mapping.setdefault(spelling, pine_fallback_name(spelling))
    return mapping

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
        "TradingView Pine v6 reference ta.*/math.*, resolved through the engine's own "
        "mapping (core/src/formula/pine/builtin_table.rs + the ast_mapper ta.* special "
        "cases) + repo:talib_coverage_matrix (numeric_reference 201)",
        PINE_SPELLINGS,
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


# --------------------------------------------------------------------------
# Corpus evidence
# --------------------------------------------------------------------------

FORMULA_CORPUS = ROOT / "tests" / "formula_corpus"
PINE_CORPUS = ROOT / "tests" / "pine_corpus"

# Corpus `platform` tag -> contract terminal key. Mirrors `terminal_for_platform`
# in core/tests/formula_dialect_coverage.rs. An unmapped tag is an error rather
# than a silent drop, so a new platform cannot quietly lose its evidence.
PLATFORM_TERMINALS = {
    "tdx": "tongdaxin",
    "cross": "tongdaxin",
    "ths": "tonghuashun",
    "dzh": "dazhihui",
    "pine": "tradingview_pine",
    "tradingview": "tradingview_pine",
}

# `NAME(` at an identifier boundary. Same shape as `called_functions` in the
# gate, so the generator and the gate cannot disagree about what "calls" means.
CALL_RE = re.compile(r"([A-Za-z_][A-Za-z0-9_]*)\s*\(")


def _called_names(source: str) -> set[str]:
    return {match.group(1).upper() for match in CALL_RE.finditer(source)}


def read_corpus_evidence() -> tuple[dict[str, set[str]], dict[str, int]]:
    """Which reference rows each checked-in corpus actually exercises.

    Derived from the corpus files themselves -- that is the whole point. The
    contract must not be able to claim test coverage it does not have, and the
    gate in ``core/tests/formula_dialect_coverage.rs`` recomputes this from the
    same files rather than trusting the numbers written here.

    Domestic corpus names *are* canonical formula names, so a call maps to a row
    by identity. A Pine ``ta.*`` call reaches a row only through the engine's
    mapping, so it is resolved with [`pine_engine_mapping`] -- the same mapping
    the behavioural gate proves against `map_pine_to_alphata`.
    """
    exercised: dict[str, set[str]] = {key: set() for key, *_ in TERMINALS}
    cases: dict[str, int] = {key: 0 for key, *_ in TERMINALS}

    for path in sorted(FORMULA_CORPUS.glob("*.json")):
        case = json.loads(path.read_text(encoding="utf-8"))
        tag = str(case.get("platform", "")).lower()
        key = PLATFORM_TERMINALS.get(tag)
        if key is None:
            raise SystemExit(
                f"{path.relative_to(ROOT)}: platform {tag!r} maps to no contract terminal"
            )
        cases[key] += 1
        exercised[key] |= _called_names(str(case.get("source_formula", "")))

    mapping = pine_engine_mapping()
    for path in sorted(PINE_CORPUS.glob("*.pine")):
        source = path.read_text(encoding="utf-8")
        cases["tradingview_pine"] += 1
        for match in re.finditer(r"\b((?:ta|math)\.[a-z_][a-z0-9_]*)\s*\(", source):
            canonical = mapping.get(match.group(1))
            if canonical is not None:
                exercised["tradingview_pine"].add(canonical)

    return exercised, cases


def build() -> dict:
    runtime = read_formula_surface()
    if not runtime:
        raise SystemExit("formula surface is empty; regenerate docs first")
    classic = read_classic_ta_core()
    corpus_exercised, corpus_cases = read_corpus_evidence()

    terminals = {}
    unverified: dict[str, list[str]] = {}
    pine_mapping: dict[str, str] = {}
    for key, short, label, reference, specific in TERMINALS:
        if key == "tradingview_pine":
            # Pine reference names are Pine spellings; the contract records the
            # canonical name the *engine* emits for them, so all four terminals
            # share one namespace and can be compared directly.
            #
            # `pine_engine_mapping()` is total, so there is no "unresolvable"
            # case to special-case here: a spelling the table does not know
            # still lands on the `TA_<NAME>` / `MATH_<NAME>` fallback, which is
            # usually -- but not always -- an unregistered name. Dropping those
            # rows instead would report ~100% coverage, the exact failure mode
            # this contract exists to prevent.
            pine_mapping = pine_engine_mapping()
            reference_names = set(pine_mapping.values())
            reference_names.update(classic)
            attribution_of: dict[str, str] = {}
            spellings_of: dict[str, list[str]] = {}
            for pine_name, canonical in pine_mapping.items():
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
            # Corpus-derived, not hand-written: which of this terminal's rows a
            # checked-in corpus case actually calls. Intersected with `functions`
            # because a corpus may legitimately use a name this terminal's
            # reference list does not carry (the contract makes no claim there).
            "corpus_cases": corpus_cases[key],
            "corpus_exercised": sorted(
                name for name in corpus_exercised[key] if name in functions
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
        # Spelling -> canonical name, as the engine resolves it. Recorded so the
        # gate can *prove* it: `pine_engine_mapping_matches_the_engine` lowers
        # every spelling through `map_pine_to_alphata` and compares the emitted
        # function name. Without that proof this table is just another mirror.
        "pine_engine_mapping": dict(sorted(pine_mapping.items())),
        # Which corpora supply the per-terminal `corpus_exercised` evidence, and
        # how many cases each contributed. Kept explicit so the two evidence
        # paths (domestic formula corpus vs Pine script corpus) are never
        # silently merged into one number.
        "corpus_sources": {
            "tests/formula_corpus": {
                "kind": "domestic_formula_cases",
                "cases": sum(
                    corpus_cases[key]
                    for key in ("tongdaxin", "tonghuashun", "dazhihui")
                ),
                "terminals": ["tongdaxin", "tonghuashun", "dazhihui"],
                "row_level_evidence": "corpus_exercised",
            },
            "tests/pine_corpus": {
                "kind": "pine_scripts",
                "cases": corpus_cases["tradingview_pine"],
                "terminals": ["tradingview_pine"],
                "row_level_evidence": "corpus_exercised",
            },
        },
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
            "counted. `corpus_exercised` is a separate, narrower fact again: the "
            "subset of rows a checked-in corpus case actually calls, derived "
            "from the corpus files (see `corpus_sources`) and recomputed by the "
            "gate. A row can be `registered` without being `corpus_exercised`; "
            "registration is the weaker claim."
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
