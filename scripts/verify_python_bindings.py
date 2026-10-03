#!/usr/bin/env python3
"""Fail when the tracked Python binding source drifts from BindingSpec.

`scripts/binding_spec.py` is the declarative contract for the Python binding
surface: the numeric hot-path ABI table, the canonical kernel bodies, the
single-write output contract, the batch zero-copy contract and the generator
hooks. Historically each of those was enforced by a migration script that both
*rewrote* tracked source and *described* the rewrite, so the shipping shape
existed only after a mutation and `cargo check --workspace` was not evidence
that the wheel would compile.

V4 plan Batch 2 replaced that arrangement: the canonical state is committed, the
build performs no writes to hand-written source, and this gate proves the tree
still matches the spec. It runs before any wheel matrix job so a drift costs one
fast job instead of four platform builds.

Usage
-----
    python scripts/verify_python_bindings.py
    python scripts/verify_python_bindings.py --json

Exit codes
----------
0  every binding surface agrees with the spec
1  drift detected; each violation is emitted as a GitHub annotation
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import binding_spec  # noqa: E402  (path setup must precede the import)


def evidence() -> dict[str, object]:
    """Describe what the spec covers, so a green run is not a vacuous one."""

    enum_names, classifiers, executors = binding_spec.formula_plan_sets(
        binding_spec.FORMULA_PLAN.read_text(encoding="utf-8")
    )
    return {
        "hot_paths": {
            hot.rust_fn: {
                "operations": len(hot.ops),
                "ids": sorted(hot.expected_ids(), key=int),
                "facade_calls": list(hot.facade_calls),
            }
            for hot in binding_spec.HOT_PATHS
        },
        "canonical_functions": [spec.label for spec in binding_spec.CANONICAL_FUNCTIONS],
        "rule_sets": [spec.label for spec in binding_spec.RULE_SETS],
        "formula_plan": {
            "enum_variants": len(enum_names),
            "classifier_arms": len(classifiers),
            "executor_arms": len(executors),
        },
        "cfo_tmf_owner": "generated.rs",
        "numpy_direct_surfaces": [
            str(path.relative_to(binding_spec.ROOT))
            for path in (binding_spec.GENERATED, binding_spec.LIB)
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit a JSON report")
    args = parser.parse_args()

    violations = binding_spec.verify()
    report = {"ok": not violations, "violations": violations, "evidence": evidence()}

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    elif violations:
        for item in violations:
            print(f"::error title=Python binding spec::{item}")
        print(
            f"\n{len(violations)} violation(s): the tracked Python binding source no "
            "longer matches scripts/binding_spec.py.\n"
            "Fix the source, or update the spec deliberately and run "
            "`python scripts/binding_spec.py --apply` — never patch the source to "
            "match a stale migration script."
        )
    else:
        coverage = report["evidence"]
        print("python binding spec verified")
        for rust_fn, info in coverage["hot_paths"].items():
            print(
                f"  {rust_fn}: {info['operations']} numeric ops "
                f"{info['ids']} -> {', '.join(info['facade_calls'])}()"
            )
        print(f"  canonical bodies : {len(coverage['canonical_functions'])}")
        print(f"  contract sets    : {len(coverage['rule_sets'])}")
        print(
            "  formula plan     : enum {enum_variants} == classifier {classifier_arms} "
            "== executor {executor_arms}".format(**coverage["formula_plan"])
        )
        print("  cfo/tmf owner    : generated.rs (single owner)")

    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
