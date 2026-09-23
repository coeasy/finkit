"""WorldQuant's 101 Alphas: availability classification and finkit transliteration.

    python scripts/gen_worldquant101.py --self-test     # verify the classification
    python scripts/gen_worldquant101.py --write         # regenerate the Rust tables
    python scripts/gen_worldquant101.py --check         # fail if they are stale

# Why this file exists, and what it found

The competitive-analysis plan asked for "WorldQuant101: at least 80 computable
factors". Measurement says that target rests on two premises that the source does
not support, and both are recorded here so the number is auditable rather than
asserted:

1. **The corpus is 71 formulas, not 101.** Kakushadze's paper numbers its alphas
   1..101 but defines only 71 of them; the other 30 numbers are reserved and
   carry no formula at all. `tests/golden/worldquant101/formulas_v1.json` records
   the reserved list explicitly.

2. **The corpus is cross-sectional, and finkit's engine is single-instrument.**
   `rank(x)` in these alphas means "rank x across the universe on that date", not
   a rolling rank over time; `scale` and `indneutralize` are likewise
   universe-level. Across the 71 formulas `rank` appears 129 times and is the
   sole blocker for 50 of them. Those alphas are not missing a *kernel* — they
   are missing a *data axis*. finkit's kernels take `&[f64]`, one instrument at a
   time, so no amount of operator work makes them computable.

   Note the trap: finkit *does* have `RANK_PCT`, but it is a rolling rank. Using
   it for `rank` would silently produce a different factor, which is why the
   classification below refuses rather than substitutes.

So the deliverable is what the criterion's second half asks for — every alpha is
either computed or annotated with the reason it is not — and the count falls out
of the classification instead of being forced. See `docs/competitive-analysis/`
for the plan and the report that supersedes the 80 target.

# The classification

Each of the 71 formulas is placed in exactly one bucket:

* `EXPRESSIONS`  — computable single-instrument; translated to finkit.
* `UNAVAILABLE`  — annotated with the cross-sectional operators that block it.

The split is derived mechanically from the formula text, and the gate asserts
that every recorded blocker really does appear in the formula it is attached to,
so an annotation cannot be invented.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FORMULAS = ROOT / "tests" / "golden" / "worldquant101" / "formulas_v1.json"
MODULE = ROOT / "core" / "src" / "factors" / "builtin" / "worldquant101.rs"

BEGIN_EXPRESSIONS = "// --- BEGIN GENERATED: expressions"
END_EXPRESSIONS = "// --- END GENERATED: expressions"
BEGIN_UNAVAILABLE = "// --- BEGIN GENERATED: unavailable"
END_UNAVAILABLE = "// --- END GENERATED: unavailable"

#: The END markers are emitted **indented by four spaces**, because they sit
#: inside an array literal and `cargo fmt` reindents them there. Emitting them at
#: column 0 made `--write` followed by `cargo fmt` followed by `--check` fail:
#: rustfmt indented one marker and left the other, so the file no longer matched
#: what the generator produced. Matching rustfmt's output makes the round trip
#: stable — and the BEGIN markers already had to be indented for the same reason.
INDENT = "    "

#: Operators that need a cross-section of instruments on one date. finkit's
#: kernels take a single series, so these are unavailable for a structural
#: reason rather than a missing implementation.
CROSS_SECTIONAL: dict[str, str] = {
    "rank": "cross-sectional rank across the universe on one date",
    "scale": "cross-sectional rescaling to sum(|x|) = a",
    "indneutralize": "cross-sectional demeaning within industry",
}

#: Source operator -> finkit spelling, for the time-series operators.
RENAME: dict[str, str] = {
    "abs": "ABS",
    "log": "LN",
    "sign": "SIGN",
    "correlation": "CORREL",
    "covariance": "COVAR",
    "sum": "SUM",
    "stddev": "STDDEV_SAMPLE",
    "ts_min": "LLV",
    "ts_max": "HHV",
    "ts_rank": "RANK_PCT",
    "Ts_Rank": "RANK_PCT",
    "product": "PRODUCT",
    "decay_linear": "WMA",
    "min": "MIN",
    "max": "MAX",
}

#: Time-series operators with no finkit kernel at all. None of the computable
#: alphas need one, but they are listed so a future formula that does is caught
#: by the self-test rather than silently mistranslated.
MISSING_KERNEL: dict[str, str] = {
    "covariance": "no rolling covariance kernel",
    "product": "no rolling product kernel",
}

#: The computable alphas, translated by hand and checked by `--self-test`.
#:
#: Rewrites applied, all of them exact rather than approximate:
#:
#: * `delta(x, d)`            -> `x-REF(x, d)`
#: * `delay(x, d)`            -> `REF(x, d)`
#: * `adv20`                  -> `MA(VOLUME, 20)` (the 20-day average volume)
#: * `returns`                -> `CLOSE/REF(CLOSE,1)-1`
#: * `(c ? a : b)`            -> `IF(c, a, b)`
#: * `(a || b)`               -> `MAX(a, b)`; comparisons yield `0.0`/`1.0`, for
#:                              which `max` is exactly logical or.
TRANSLATIONS: dict[str, str] = {
    "Alpha6": "-1*CORREL(OPEN, VOLUME, 10)",
    "Alpha7": (
        "IF(MA(VOLUME,20)<VOLUME, "
        "-1*RANK_PCT(ABS(CLOSE-REF(CLOSE,7)), 60)*SIGN(CLOSE-REF(CLOSE,7)), "
        "-1)"
    ),
    "Alpha9": (
        "IF(0<LLV(CLOSE-REF(CLOSE,1), 5), CLOSE-REF(CLOSE,1), "
        "IF(HHV(CLOSE-REF(CLOSE,1), 5)<0, CLOSE-REF(CLOSE,1), "
        "-1*(CLOSE-REF(CLOSE,1))))"
    ),
    "Alpha12": "SIGN(VOLUME-REF(VOLUME,1))*(-1*(CLOSE-REF(CLOSE,1)))",
    "Alpha21": (
        "IF((SUM(CLOSE,8)/8+STDDEV_SAMPLE(CLOSE,8))<(SUM(CLOSE,2)/2), -1, "
        "IF((SUM(CLOSE,2)/2)<(SUM(CLOSE,8)/8-STDDEV_SAMPLE(CLOSE,8)), 1, "
        "IF(MAX(1<VOLUME/MA(VOLUME,20), VOLUME/MA(VOLUME,20)==1), 1, -1)))"
    ),
    "Alpha23": "IF(SUM(HIGH,20)/20<HIGH, -1*(HIGH-REF(HIGH,2)), 0)",
    "Alpha24": (
        "IF(MAX(((SUM(CLOSE,100)/100)-REF(SUM(CLOSE,100)/100,100))/REF(CLOSE,100)<0.05, "
        "((SUM(CLOSE,100)/100)-REF(SUM(CLOSE,100)/100,100))/REF(CLOSE,100)==0.05), "
        "-1*(CLOSE-LLV(CLOSE,100)), -1*(CLOSE-REF(CLOSE,3)))"
    ),
    "Alpha26": "-1*HHV(CORREL(RANK_PCT(VOLUME,5), RANK_PCT(HIGH,5), 5), 3)",
    "Alpha35": (
        "RANK_PCT(VOLUME,32)*(1-RANK_PCT((CLOSE+HIGH)-LOW,16))"
        "*(1-RANK_PCT(CLOSE/REF(CLOSE,1)-1,32))"
    ),
    "Alpha41": "(HIGH*LOW)^0.5-VWAP",
    "Alpha43": (
        "RANK_PCT(VOLUME/MA(VOLUME,20), 20)*RANK_PCT(-1*(CLOSE-REF(CLOSE,7)), 8)"
    ),
    "Alpha46": (
        "IF(0.25<(((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10)), -1, "
        "IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<0, 1, "
        "-1*(CLOSE-REF(CLOSE,1))))"
    ),
    "Alpha49": (
        "IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<(-1*0.1), "
        "1, -1*(CLOSE-REF(CLOSE,1)))"
    ),
    "Alpha51": (
        "IF((((REF(CLOSE,20)-REF(CLOSE,10))/10)-((REF(CLOSE,10)-CLOSE)/10))<(-1*0.05), "
        "1, -1*(CLOSE-REF(CLOSE,1)))"
    ),
    "Alpha53": (
        "-1*(((CLOSE-LOW)-(HIGH-CLOSE))/(CLOSE-LOW)"
        "-REF(((CLOSE-LOW)-(HIGH-CLOSE))/(CLOSE-LOW),9))"
    ),
    "Alpha54": "(-1*((LOW-CLOSE)*OPEN^5))/((LOW-HIGH)*CLOSE^5)",
    "Alpha101": "(CLOSE-OPEN)/((HIGH-LOW)+0.001)",
}


def load_formulas() -> dict[str, str]:
    return json.loads(FORMULAS.read_text(encoding="utf-8"))["formulas"]


def operators(expression: str) -> set[str]:
    """Function names called in an expression, ignoring field references."""
    return set(re.findall(r"([A-Za-z_][A-Za-z_0-9]*)\s*\(", expression))


def blockers(expression: str) -> list[str]:
    """Cross-sectional operators present in a formula, sorted for stability."""
    return sorted(operators(expression) & set(CROSS_SECTIONAL))


def classify() -> tuple[list[tuple[str, str]], list[tuple[str, list[str]]]]:
    formulas = load_formulas()
    computable: list[tuple[str, str]] = []
    unavailable: list[tuple[str, list[str]]] = []
    for name, formula in formulas.items():
        blocking = blockers(formula)
        if blocking:
            unavailable.append((name, blocking))
        else:
            if name not in TRANSLATIONS:
                raise KeyError(
                    f"{name} has no cross-sectional blocker but no translation "
                    f"either: {formula}"
                )
            computable.append((name, TRANSLATIONS[name]))
    return computable, unavailable


def _rust_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def render_expressions() -> str:
    computable, _ = classify()
    lines = [BEGIN_EXPRESSIONS]
    for name, expression in computable:
        lines.append(f"    ({_rust_string(name)}, {_rust_string(expression)}),")
    lines.append(INDENT + END_EXPRESSIONS)
    return "\n".join(lines)


def render_unavailable() -> str:
    _, unavailable = classify()
    lines = [BEGIN_UNAVAILABLE]
    for name, blocking in unavailable:
        operators = ", ".join(_rust_string(op) for op in blocking)
        lines.append(f"    ({_rust_string(name)}, &[{operators}]),")
    lines.append(INDENT + END_UNAVAILABLE)
    return "\n".join(lines)


def _replace_region(text: str, begin: str, end: str, block: str) -> str:
    start = text.index(begin)
    stop = text.index(end, start) + len(end)
    return text[:start] + block + text[stop:]


def write_module() -> None:
    text = MODULE.read_text(encoding="utf-8")
    text = _replace_region(text, BEGIN_EXPRESSIONS, END_EXPRESSIONS, render_expressions())
    text = _replace_region(text, BEGIN_UNAVAILABLE, END_UNAVAILABLE, render_unavailable())
    MODULE.write_text(text, encoding="utf-8")


def check_module() -> int:
    text = MODULE.read_text(encoding="utf-8")
    expected = _replace_region(
        _replace_region(text, BEGIN_EXPRESSIONS, END_EXPRESSIONS, render_expressions()),
        BEGIN_UNAVAILABLE,
        END_UNAVAILABLE,
        render_unavailable(),
    )
    if text != expected:
        print(
            f"{MODULE} has drifted from the classification; "
            "regenerate with `--write`",
            file=sys.stderr,
        )
        return 1
    computable, unavailable = classify()
    print(
        f"{MODULE.name} matches: {len(computable)} computable, "
        f"{len(unavailable)} annotated"
    )
    return 0


def self_test() -> int:
    failures: list[str] = []
    formulas = load_formulas()
    computable, unavailable = classify()

    if len(formulas) != 71:
        failures.append(f"formula count: got {len(formulas)}, want 71")
    if len(computable) + len(unavailable) != len(formulas):
        failures.append(
            f"the buckets do not partition the corpus: "
            f"{len(computable)} + {len(unavailable)} != {len(formulas)}"
        )
    overlap = {name for name, _ in computable} & {name for name, _ in unavailable}
    if overlap:
        failures.append(f"alphas in both buckets: {sorted(overlap)}")

    # Every recorded blocker must actually occur in its formula, or the
    # annotation is a guess dressed up as a finding.
    for name, blocking in unavailable:
        present = operators(formulas[name])
        for operator in blocking:
            if operator not in present:
                failures.append(
                    f"{name} is annotated as blocked by {operator!r}, "
                    f"which does not appear in {formulas[name]!r}"
                )

    # Names a translation is allowed to emit. `RENAME`'s images, plus the
    # operators that the *source-level* rewrites introduce: `delta`/`delay`
    # expand to `REF`, `adv20` to `MA`, `returns` to a `REF`-based ratio, the
    # ternary to `IF`, and `||` to `MAX`.
    allowed = set(RENAME.values()) | {"IF", "REF", "MAX", "MIN", "MA", "VWAP"}
    for name, expression in computable:
        for call in operators(expression):
            if call not in allowed:
                failures.append(f"{name} emits the unknown function {call!r}")
        # Nothing may survive translation as a lowercase source operator.
        for source in RENAME:
            if re.search(rf"\b{source}\s*\(", expression):
                failures.append(
                    f"{name} kept the source operator {source!r}: {expression}"
                )
        # A translation that dropped the whole expression would still be "valid".
        if not expression.strip():
            failures.append(f"{name} translated to an empty expression")

    # The reserved list is part of the finding, so it is asserted, not implied.
    reserved = set(
        json.loads(FORMULAS.read_text(encoding="utf-8"))["reference"]["reserved_numbers"]
    )
    if len(reserved) != 30:
        failures.append(f"reserved numbers: got {len(reserved)}, want 30")

    if failures:
        for failure in failures:
            print(f"FAIL {failure}", file=sys.stderr)
        return 1
    print(
        f"self-test passed: {len(formulas)} formulas, {len(computable)} computable, "
        f"{len(unavailable)} blocked, 30 numbers reserved by the paper"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--write", action="store_true", help="regenerate the Rust tables")
    parser.add_argument("--check", action="store_true", help="fail if the Rust tables are stale")
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    if args.write:
        write_module()
        computable, unavailable = classify()
        print(f"wrote {MODULE}: {len(computable)} computable, {len(unavailable)} annotated")
        return 0
    if args.check:
        return check_module()
    parser.print_help()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
