#!/usr/bin/env python3
"""Fail fast on warning regressions that previously escaped cross-platform CI.

The checks here are intentionally structural and cheap. Real compilers remain the
source of truth; this script prevents the known bad source forms from returning
before the Windows/macOS native jobs spend minutes compiling the workspace.
"""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def fail(message: str) -> None:
    raise SystemExit(f"warning contract failed: {message}")


def main() -> int:
    simd_path = ROOT / "core/src/formula/simd.rs"
    simd = simd_path.read_text(encoding="utf-8")

    # On aarch64 these dispatch blocks used to `return` into NEON and then
    # compile a scalar fallback immediately afterwards, producing 18
    # unreachable-code warnings on Apple Silicon. Architecture-exclusive final
    # expressions keep the fallback available everywhere else without compiling
    # dead code on aarch64.
    if re.search(
        r'#\[cfg\(target_arch = "aarch64"\)\]\s*\{\s*return unsafe',
        simd,
        flags=re.MULTILINE,
    ):
        fail("aarch64 SIMD dispatch must not use an unconditional `return unsafe` before fallback code")

    if re.search(
        r'#\[cfg\(target_arch = "aarch64"\)\]\s*\{\s*return SimdLevel::Neon;',
        simd,
        flags=re.MULTILINE,
    ):
        fail("simd_level must use cfg-exclusive final expressions on aarch64")

    statistics = (ROOT / "core/src/indicators/statistics.rs").read_text(encoding="utf-8")
    if re.search(
        r'#\[deprecated\([^\]]*\)\]\s*pub fn linear_reg\s*\(',
        statistics,
        flags=re.MULTILINE,
    ):
        fail("linear_reg compatibility spelling must not be compiler-deprecated before its release transition")

    registry_path = ROOT / "docs/indicator_registry.json"
    if "indicators::linear_reg(" in registry_path.read_text(encoding="utf-8"):
        fail("binding SSOT must call canonical indicators::linearreg, not deprecated linear_reg")

    offenders: list[str] = []
    for path in sorted((ROOT / "ffi").glob("**/generated.rs")):
        if "indicators::linear_reg(" in path.read_text(encoding="utf-8"):
            offenders.append(str(path.relative_to(ROOT)))
    if offenders:
        fail("generated bindings still call linear_reg: " + ", ".join(offenders))

    print("warning contracts: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
