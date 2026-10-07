#!/usr/bin/env python3
"""Audit how much of the FFI indicator surface each language binding exposes.

Why this exists
---------------
`docs/ffi_registry.json` is the *declared* FFI surface: 78 indicators, each with
a canonical ``c_name`` and machine-readable inputs/outputs/params.  Every
language binding under ``ffi/`` is supposed to expose that same set through its
own idiom (C prototypes, ``#[pyfunction]``, ``#[napi]``, Go exported funcs,
JVM natives, C# P/Invoke wrappers, ``alpha_ta_*`` for iOS, ``shim_indicator!``
for Android).

Nothing measured that.  The twenty-fourth-pass audit of this gap was done by
hand, and it disagreed with a second hand audit -- one counted Go at 51/95, a
recount said 56/95, because "95" mixed indicators with infrastructure
(``ta_version``/``ta_free_*``/``ta_formula_*``/streaming) and because Go spells
``ta_adosc`` as ``AdOsc`` and C# spells it ``AdOsc`` too while Python spells it
``adosc``.  A number nobody can reproduce is a number nobody can improve.

This script makes the gap computable.  The canonical reference is the 78
indicators in ``docs/ffi_registry.json``; each binding's public surface is
scraped from the file that actually defines it; names are matched through a
normalisation table that absorbs the underscore/camel/spelling differences
(``ad_osc`` == ``AdOsc`` == ``adosc``) but *not* genuine collisions.

Modes
-----
``--report`` (default)   print a per-language table plus the missing sets.
``--json-out PATH``      also write the full machine-readable result.
``--check``              ratchet gate: fail if any language drops below the
                         recorded baseline, or if the baseline is stale.
``--update-baseline``    rewrite docs/binding_parity_baseline.json from the
                         current measurement (only ever widens, never narrows,
                         unless --force).

The ratchet is deliberate.  A gate that fails on today's gap would be
permanently red and therefore ignored -- the same reasoning
``docs/language-bindings.md`` gives for the deferred-tier reporting in
``sync_bindings.py``.  A gate that fails when coverage *regresses* is not.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FFI_REGISTRY = ROOT / "docs" / "ffi_registry.json"
BASELINE = ROOT / "docs" / "binding_parity_baseline.json"

# ──────────────────────────────────────────────────────────────────────────
# Canonical reference: the 78 indicators in docs/ffi_registry.json
# ──────────────────────────────────────────────────────────────────────────


def _squash(name: str) -> str:
    """Normalise a spelling for matching: lowercase, drop all separators.

    ``ta_cdl_three_white_soldiers`` -> ``cdlthreewhitesoldiers``
    ``CdlThreeWhiteSoldiers``       -> ``cdlthreewhitesoldiers``
    ``cdl3whitesoldiers``           -> ``cdl3whitesoldiers``  (registry spelling)
    """
    return re.sub(r"[^a-z0-9]", "", name.lower())


# Spellings that legitimately refer to one canonical key but do not squash to
# the key or the registry display name.  Each entry was derived by listing the
# actual public name every binding uses; anything not here is a real gap.
EXTRA_ALIASES: dict[str, set[str]] = {
    "bbands": {"bollinger_bands", "bollingerbands"},
    "correlation": {"correl"},
    "linear_reg": {"linreg", "linearreg"},
    "stddev": {"std_dev", "standard_dev"},
    "chande_forecast": {"chande_forecast_oscillator", "chandeforecastoscillator", "cfo"},
    "twiggs_mf": {"twiggs_money_flow", "twiggsmoneyflow"},
    "inertia": {"inertia_indicator", "inertiaindicator"},
    "cdl_three_white_soldiers": {"cdl_3white_soldiers", "cdl3whitesoldiers"},
    "cdl_three_black_crows": {"cdl_3black_crows", "cdl3blackcrows"},
    "volume_roc": {"volumepriceroc"},
    "percent_rank": {"pctrank"},
    "midpoint": {"mid_point"},
    "midprice": {"mid_price"},
    "typprice": {"typ_price"},
    "wclprice": {"wcl_price", "weighted_close"},
    "avgprice": {"avg_price"},
    "medprice": {"med_price"},
}


def _candidates(raw: str) -> list[str]:
    """All spellings of ``raw`` a binding might be using for one indicator.

    Bindings decorate the transport name: C/iOS/Android prefix it
    (``ta_``/``alpha_ta_``), Go/Java re-expose chart and pattern indicators as
    JSON-returning ``...JSON``/``...Native`` variants, and Java appends
    ``Native`` to the JNI declaration.  Every decoration is stripped here so
    the matcher compares the underlying indicator, not the transport.
    """
    out = [raw]
    if raw.endswith("Native"):
        out.append(raw[: -len("Native")])
    prefixed = list(out)
    for name in prefixed:
        for prefix in ("alpha_ta_", "ta_"):
            if name.startswith(prefix):
                out.append(name[len(prefix):])
    expanded = list(out)
    for name in expanded:
        if name.endswith("JSON"):
            out.append(name[: -len("JSON")])
    return out


@dataclass
class CanonicalSurface:
    keys: list[str] = field(default_factory=list)
    c_names: dict[str, str] = field(default_factory=dict)
    display: dict[str, str] = field(default_factory=dict)
    squash_to_key: dict[str, str] = field(default_factory=dict)

    def match(self, raw: str) -> str | None:
        for cand in _candidates(raw):
            key = self.squash_to_key.get(_squash(cand))
            if key is not None:
                return key
        return None


def load_canonical() -> CanonicalSurface:
    data = json.loads(FFI_REGISTRY.read_text(encoding="utf-8"))
    surf = CanonicalSurface()
    for item in data["indicators"]:
        ffi = item["ffi"]
        c_name = ffi["c_name"]
        key = c_name[3:] if c_name.startswith("ta_") else c_name
        surf.keys.append(key)
        surf.c_names[key] = c_name
        surf.display[key] = item["name"]
        surf.squash_to_key[_squash(key)] = key
        surf.squash_to_key[_squash(item["name"])] = key
        for alias in EXTRA_ALIASES.get(key, ()):  # noqa: SIM118
            surf.squash_to_key[_squash(alias)] = key
    return surf


# ──────────────────────────────────────────────────────────────────────────
# Per-language public-surface extraction
# ──────────────────────────────────────────────────────────────────────────


def _read(p: Path) -> str:
    if not p.exists():
        return ""
    return p.read_text(encoding="utf-8", errors="replace")


def _collect(paths, pattern: str, group: int = 1) -> set[str]:
    out: set[str] = set()
    for p in paths:
        out |= {m.group(group) for m in re.finditer(pattern, _read(p), re.MULTILINE)}
    return out


# Each extractor returns the *public* names a caller would write.  Test
# functions, constructors of streaming handles and JSON infra are filtered out
# afterwards by the canonical matcher (they simply do not match a key) -- that
# keeps the extractors honest instead of hand-pruning each list.
EXTRACTORS = {
    "c": lambda: _collect(
        [ROOT / "ffi/c-binding/include/finkit.h"], r"\b(ta_[A-Za-z0-9_]+)\s*\("
    ),
    "python": lambda: _collect(
        [ROOT / "ffi/python-binding/src/generated.rs", ROOT / "ffi/python-binding/src/lib.rs"],
        r"#\[pyfunction\][\s\S]{0,600}?\bfn\s+([a-z][a-z0-9_]*)\s*\(",
    ),
    "node": lambda: _collect(
        [ROOT / "ffi/node-binding/index.d.ts"],
        r"export\s+(?:declare\s+)?function\s+([A-Za-z0-9_]+)\s*\(",
    ),
    "go": lambda: _collect(
        sorted((ROOT / "ffi/go-binding/go/ta").glob("*.go")),
        r"^func\s+([A-Z][A-Za-z0-9_]*)\s*\(",
    ),
    "java": lambda: _collect(
        sorted((ROOT / "ffi/java-binding/java/src/main/java/com/finkit").glob("*.java")),
        r"public\s+static\s+(?:native\s+)?[A-Za-z0-9_<>,\[\]\.\s]+?\s+([a-z][A-Za-z0-9_]*)\s*\(",
    ),
    "dotnet": lambda: _collect(
        sorted((ROOT / "ffi/dotnet-binding/src/Finkit").glob("*.cs")),
        r"public\s+static\s+(?:extern\s+)?[A-Za-z0-9_<>,\[\]\.\?\s]+?\s+([A-Z][A-Za-z0-9_]*)\s*\(",
    ),
    "ios": lambda: _collect(
        [ROOT / "ffi/ios-binding/src/generated.rs", ROOT / "ffi/ios-binding/src/lib.rs"],
        r'pub\s+extern\s+"C"\s+fn\s+(alpha_ta_[A-Za-z0-9_]+)',
    ),
    "android": lambda: _collect(
        [ROOT / "ffi/android-binding/src/generated.rs", ROOT / "ffi/android-binding/src/lib.rs"],
        r'shim_indicator!\(\s*[^,]+,\s*"(ta_[A-Za-z0-9_]+)"',
    ),
}

# Languages whose extractor yields the *native* symbol (ta_x / alpha_ta_x)
# rather than the ergonomic public name.  Reported the same way; the matcher
# only cares whether the name resolves to a canonical key.
LANG_ORDER = ["c", "python", "node", "go", "java", "dotnet", "ios", "android"]

# Binding tiers, mirroring scripts/sync_bindings.py so the two documents cannot
# disagree about which languages are load-bearing.
ACTIVE_LANGS = ("python", "node")
DEFERRED_LANGS = ("c", "go", "java", "dotnet", "ios", "android")


@dataclass
class LangReport:
    lang: str
    covered: list[str]
    missing: list[str]
    extra: list[str]

    @property
    def total(self) -> int:
        return len(self.covered) + len(self.missing)


def measure(surf: CanonicalSurface) -> dict[str, LangReport]:
    canon = set(surf.keys)
    reports: dict[str, LangReport] = {}
    for lang in LANG_ORDER:
        raw = EXTRACTORS[lang]()
        resolved = {surf.match(n) for n in raw}
        resolved.discard(None)
        covered = sorted(resolved & canon)
        missing = sorted(canon - resolved)
        extra = sorted(n for n in raw if surf.match(n) is None)
        reports[lang] = LangReport(lang, covered, missing, extra)
    return reports


# ──────────────────────────────────────────────────────────────────────────
# Reporting
# ──────────────────────────────────────────────────────────────────────────


def print_report(surf: CanonicalSurface, reports: dict[str, LangReport]) -> None:
    total = len(surf.keys)
    print(f"canonical FFI indicator surface: {total} (docs/ffi_registry.json)\n")
    width = max(len(lang) for lang in LANG_ORDER)
    print(f"{'binding':<{width}}  {'tier':<8}  {'covered':>9}  {'missing':>7}")
    print("-" * (width + 30))
    for lang in LANG_ORDER:
        r = reports[lang]
        tier = "active" if lang in ACTIVE_LANGS else "deferred"
        bar = "" if r.missing else "  OK"
        print(f"{lang:<{width}}  {tier:<8}  {len(r.covered):>4}/{total:<4}  {len(r.missing):>7}{bar}")
    print()
    for lang in LANG_ORDER:
        r = reports[lang]
        if r.missing:
            print(f"[{lang}] missing {len(r.missing)}: {', '.join(r.missing)}")
    print()
    for lang in LANG_ORDER:
        r = reports[lang]
        if r.extra:
            print(f"[{lang}] extra (beyond the 78-indicator registry): {len(r.extra)}")


def as_json(surf: CanonicalSurface, reports: dict[str, LangReport]) -> dict:
    return {
        "canonical_total": len(surf.keys),
        "canonical_source": "docs/ffi_registry.json",
        "bindings": {
            lang: {
                "tier": "active" if lang in ACTIVE_LANGS else "deferred",
                "covered": len(reports[lang].covered),
                "total": len(surf.keys),
                "covered_names": reports[lang].covered,
                "missing": reports[lang].missing,
                "extra_count": len(reports[lang].extra),
            }
            for lang in LANG_ORDER
        },
    }


# ──────────────────────────────────────────────────────────────────────────
# Ratchet gate
# ──────────────────────────────────────────────────────────────────────────


def load_baseline() -> dict:
    if not BASELINE.exists():
        return {}
    return json.loads(BASELINE.read_text(encoding="utf-8"))


def cmd_check(surf: CanonicalSurface, reports: dict[str, LangReport]) -> int:
    base = load_baseline()
    if not base:
        print(f"FAIL: {BASELINE.relative_to(ROOT)} is missing; run --update-baseline")
        return 1
    recorded = base.get("bindings", {})
    failures: list[str] = []
    stale: list[str] = []
    for lang in LANG_ORDER:
        actual = len(reports[lang].covered)
        want = recorded.get(lang, {}).get("covered")
        if want is None:
            stale.append(f"{lang} absent from baseline")
            continue
        if actual < want:
            failures.append(f"{lang}: {actual}/{len(surf.keys)} < baseline {want}")
        elif actual > want:
            stale.append(f"{lang}: {actual} > baseline {want} (run --update-baseline)")
    if failures:
        print("binding parity REGRESSED:")
        for f in failures:
            print(f"  - {f}")
        return 1
    if stale:
        print("binding parity baseline is stale (coverage improved):")
        for s in stale:
            print(f"  - {s}")
        return 1
    print(f"binding parity OK (baseline matches for all {len(LANG_ORDER)} bindings)")
    return 0


def cmd_update(surf: CanonicalSurface, reports: dict[str, LangReport], force: bool) -> int:
    old = load_baseline().get("bindings", {})
    new = {"canonical_total": len(surf.keys), "bindings": {}}
    narrowed: list[str] = []
    for lang in LANG_ORDER:
        actual = len(reports[lang].covered)
        prev = old.get(lang, {}).get("covered")
        if prev is not None and actual < prev and not force:
            narrowed.append(f"{lang}: {prev} -> {actual}")
            new["bindings"][lang] = {"covered": prev}
        else:
            new["bindings"][lang] = {"covered": actual}
    BASELINE.write_text(json.dumps(new, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {BASELINE.relative_to(ROOT)}")
    if narrowed:
        print("refused to narrow these without --force (kept old value):")
        for n in narrowed:
            print(f"  - {n}")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="ratchet gate against the baseline")
    ap.add_argument("--update-baseline", action="store_true", help="rewrite the baseline file")
    ap.add_argument("--force", action="store_true", help="allow --update-baseline to narrow coverage")
    ap.add_argument("--json-out", type=Path, default=None, help="write the full result as JSON")
    ap.add_argument("--quiet", action="store_true", help="suppress the human report")
    args = ap.parse_args(argv)

    surf = load_canonical()
    reports = measure(surf)

    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(as_json(surf, reports), indent=2) + "\n", encoding="utf-8")

    if not args.quiet:
        print_report(surf, reports)

    if args.update_baseline:
        return cmd_update(surf, reports, args.force)
    if args.check:
        return cmd_check(surf, reports)
    return 0


if __name__ == "__main__":
    sys.exit(main())
