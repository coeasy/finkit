#!/usr/bin/env python3
"""Prepare the canonical NumPy-direct Python binding surface for wheel builds.

This is a permanent build step, not a migration helper. It keeps the canonical
indicator registry unchanged, builds the transient Python SSOT overlay, regenerates
the generated PyO3 surface, and rewrites numeric PyO3 functions to return NumPy
arrays directly instead of materializing Python lists first.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from optimize_python_bindings import optimize_file

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
    # Build an ephemeral Python-only registry overlay. The helper restores the
    # canonical docs registry before it exits and teaches sync_bindings to read
    # the overlay for this build workspace.
    run(str(ROOT / "scripts" / "prepare_python_registry_ssot.py"))

    # Regenerate from SSOT first; optimization is deliberately post-generation
    # so generated bindings can never silently fall back to Vec -> Python list.
    run(
        str(ROOT / "scripts" / "sync_bindings.py"),
        "--lang",
        "python",
        "--generate",
    )

    generated_count = optimize_file(GENERATED)
    lib_count = optimize_file(LIB)

    # Idempotence is part of the build contract: a second optimization pass must
    # find nothing left to rewrite.
    optimize_file(GENERATED, check=True)
    optimize_file(LIB, check=True)

    print(
        "[prepare/python-hot] NumPy-direct binding surface ready: "
        f"generated={generated_count}, lib={lib_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
