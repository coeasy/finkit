#!/usr/bin/env python3
"""Verify generated float64 Python bindings return NumPy arrays directly.

Returning a Python `list` from a numeric binding costs a full element-wise
conversion on every call; the shipping contract is that `generated.rs` and
`lib.rs` expose `Py<PyArray1<f64>>` (and tuples of them) and keep the original
`Vec`-returning body as a `vec_<name>_impl` helper.

This module used to rewrite the two files in place. V4 plan Batch 2 retired that:
`scripts/optimize_python_bindings.py` owns the transformation and exposes
`--check`, the generator applies it while emitting `generated.rs`, and this
module is the focused verifier that neither surface regressed.

Exit codes
----------
0  both surfaces are NumPy-direct
1  a numeric binding still returns Python lists
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import binding_spec  # noqa: E402  (path setup must precede the import)


def main() -> int:
    violations = binding_spec.verify_numpy_direct()
    if violations:
        for item in violations:
            print(f"::error title=NumPy-direct contract::{item}")
        return 1

    print(
        "NumPy-direct contract verified: "
        + ", ".join(
            str(path.relative_to(binding_spec.ROOT))
            for path in (binding_spec.GENERATED, binding_spec.LIB)
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
