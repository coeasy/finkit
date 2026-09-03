#!/usr/bin/env python3
"""Fail fast on warning regressions that previously escaped cross-platform CI.

The checks here are intentionally structural and cheap. Real compilers remain the
source of truth; this script prevents the known bad source forms and legacy action
runtimes from returning before platform jobs spend minutes compiling the workspace.
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

    # GitHub-hosted runners now execute modern JavaScript actions with Node 24.
    # These exact legacy action majors/pins were observed in Finkit's real workflow
    # logs being forced from deprecated Node 20. Keep this list narrow: a generic
    # ban on every `@v4` would incorrectly reject unrelated actions whose major
    # version does not imply the JavaScript runtime.
    legacy_actions = {
        "actions/checkout@v4": "actions/checkout@v7",
        "actions/setup-node@v4": "actions/setup-node@v7",
        "actions/setup-java@v4": "actions/setup-java@v6",
        "actions/setup-go@v5": "actions/setup-go@v7",
        "actions/setup-dotnet@v4": "actions/setup-dotnet@v6",
        "actions/upload-artifact@v4": "actions/upload-artifact@v7",
        "actions/download-artifact@v4": "actions/download-artifact@v8",
        "android-actions/setup-android@v3": "android-actions/setup-android@v4",
        "gradle/actions/setup-gradle@v4": "gradle/actions/setup-gradle@v6",
        "PyO3/maturin-action@86b9d133d34bc1b40018696f782949dac11bd380": (
            "PyO3/maturin-action@e83996d129638aa358a18fbd1dfb82f0b0fb5d3b"
        ),
    }
    workflow_dir = ROOT / ".github/workflows"
    runtime_offenders: list[str] = []
    workflow_paths = sorted(workflow_dir.glob("*.yml")) + sorted(workflow_dir.glob("*.yaml"))
    for path in workflow_paths:
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            for legacy, replacement in legacy_actions.items():
                if re.search(rf"\buses:\s*{re.escape(legacy)}(?:\s|$)", line):
                    runtime_offenders.append(
                        f"{path.relative_to(ROOT)}:{line_number}: {legacy} -> {replacement}"
                    )
    if runtime_offenders:
        fail("legacy Node 20 action runtime(s): " + "; ".join(runtime_offenders))

    print("warning contracts: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
