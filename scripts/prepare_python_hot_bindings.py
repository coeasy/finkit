#!/usr/bin/env python3
"""Prepare the canonical NumPy-direct Python binding surface for wheel builds.

This is a permanent build step, not a migration helper. V4 plan Batch 2 changed
what it is allowed to do. It used to reach the shipping shape by rewriting
tracked hand-written source in place — `native_fast_path.rs`, `lib.rs`,
`momentum.rs`, `volume.rs`, `formula_plan.rs`, the Python facade and even
`sync_bindings.py`. The consequence was that the shipped source existed only
after a mutation: the four wheel platforms failed on a `u16` vs `"var"` mismatch
while `cargo check --workspace` stayed green.

The steps are now:

1. **Verify** the whole binding contract against ``scripts/binding_spec.py``
   before anything is written, so drift costs seconds instead of four platform
   builds.
2. **Generate** the transient registry overlay (``target/``, untracked) that
   teaches the Python generator about the FFI bodies.
3. **Regenerate** the registry-owned ``generated.rs`` from that overlay.
4. **Check** the NumPy-direct return contract on both generated surfaces.
5. **Prove** that no hand-written tracked source changed content during the run.

Step 5 is the load-bearing one: it turns "the build no longer rewrites tracked
source" from a promise into a check that fails loudly if a future contributor
reintroduces a patcher.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import binding_spec  # noqa: E402  (path setup must precede the import)
from optimize_python_bindings import optimize_file  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
GENERATED = ROOT / "ffi" / "python-binding" / "src" / "generated.rs"
LIB = ROOT / "ffi" / "python-binding" / "src" / "lib.rs"


def run(*args: str) -> None:
    env = os.environ.copy()
    # Windows hosted runners default redirected stdout to a legacy code page.
    # Force one UTF-8 process contract so SSOT/generator diagnostics are
    # identical on Linux, macOS, and Windows and cannot abort on Unicode text.
    env["PYTHONIOENCODING"] = "utf-8"
    env["PYTHONUTF8"] = "1"
    subprocess.run([sys.executable, *args], cwd=ROOT, env=env, check=True)


def main() -> int:
    # 1. Fail before writing anything.
    violations = binding_spec.verify()
    if violations:
        for item in violations:
            print(f"::error title=Python binding spec::{item}")
        print(
            f"\n{len(violations)} binding violation(s); refusing to start the build. "
            "Run `python scripts/verify_python_bindings.py` for the full report."
        )
        return 1

    before = binding_spec.fingerprints()

    # 2. Transient Python-only registry overlay. The helper builds
    #    `target/python_registry_ssot.json` and restores the canonical registry
    #    it temporarily touches, so nothing under version control is left
    #    modified; step 5 verifies that claim.
    run(str(ROOT / "scripts" / "prepare_python_registry_ssot.py"))

    # 3. Regenerate the registry-owned binding file. `generated.rs` is a
    #    generated artefact: regeneration is the only writer, and the NumPy
    #    transformation is applied by the live SSOT generator itself
    #    (`optimize_python_source` inside sync_bindings), not by a later pass.
    run(
        str(ROOT / "scripts" / "sync_bindings.py"),
        "--lang",
        "python",
        "--generate",
    )

    # 4. Read-only verification of the generated surfaces.
    optimize_file(GENERATED, check=True)
    optimize_file(LIB, check=True)

    # 5. The build must not have edited hand-written tracked source.
    mutated = binding_spec.mutated_sources(before, binding_spec.fingerprints())
    if mutated:
        for path in mutated:
            print(
                f"::error title=Python build-state::hand-written source was rewritten "
                f"by the build: {path}"
            )
        print(
            "\nThe preparation step is read-only for hand-written source. Move the "
            "transformation into scripts/binding_spec.py, land the canonical state in "
            "the tree, and let this step verify it instead."
        )
        return 1

    print(
        "[prepare/python-hot] NumPy-direct binding surface ready: "
        f"hot-paths={len(binding_spec.HOT_PATHS)}, "
        f"contracts={len(binding_spec.RULE_SETS)}, "
        f"canonical-bodies={len(binding_spec.CANONICAL_FUNCTIONS)}, "
        "batch=zero-copy, formula=canonical, native=v3, output=single-write, "
        "handwritten-source=unmodified"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
