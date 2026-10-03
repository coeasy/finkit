#!/usr/bin/env python3
"""Verify the TA-Lib performance binding contract.

This module used to be a migration program: it rewrote the BBANDS / STOCH / SAR
kernel bodies, the Python registry contract, the package `__init__`, the batch
compute entry point, the generated bindings and `sync_bindings.py` — in place —
and then ran a drift check. V4 plan Batch 2 moved every one of those rules into
``scripts/binding_spec.py`` and landed the canonical state in the tree, so this
module keeps only what is still useful at build time: a verification pass.

`patch_sync_bindings()` is retained under its original name because it was the
imported entry point for the generator hooks; it now *requires* those hooks
instead of installing them.

Exit codes
----------
0  the contract holds and the Python generator reports no drift
1  drift, printed as GitHub annotations
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import binding_spec  # noqa: E402  (path setup must precede the import)

ROOT = Path(__file__).resolve().parents[1]


def patch_sync_bindings() -> None:
    """Require the generator's overlay/merge hooks (historical entry point)."""

    generator = binding_spec.SYNC_BINDINGS.read_text(encoding="utf-8")
    missing = [
        needle
        for needle in binding_spec.SYNC_BINDINGS_RULESET.required
        if needle not in generator
    ]
    if missing:
        raise RuntimeError(
            "sync_bindings.py lost part of the Python generation contract: "
            + ", ".join(repr(needle) for needle in missing)
        )


def main() -> int:
    violations = binding_spec.verify()
    if violations:
        for item in violations:
            print(f"::error title=TA-Lib performance contract::{item}")
        return 1

    patch_sync_bindings()

    subprocess.run(
        [sys.executable, str(ROOT / "scripts" / "sync_bindings.py"), "--lang", "python", "--check"],
        cwd=ROOT,
        check=True,
    )
    print(
        "TA-Lib performance contract verified: kernel bodies, registry contract, "
        "package facade, batch contract and NumPy-direct generation all in place"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
