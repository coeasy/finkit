#!/usr/bin/env python3
"""Binding feature-contract gate (V5 Batch 5 ④).

Finkit's cross-language promise is "one numeric semantics, every language".
That promise extends to the build state: each language binding declares an
explicit, *canonical* set of core (`finkit`) features, and what actually gets
compiled for that binding must equal the declaration — not the result of
Cargo feature unification quietly pulling `finkit`'s `default` back in
through an intermediate crate (the V5 finding N4).

This script enforces, per binding:

1.  The `finkit` dependency line in the binding's `Cargo.toml` carries
    `default-features = false` and exactly the contract's base features.
2.  `cargo tree -p <binding> -e features -i finkit` reports exactly the
    contract's effective feature set (base ∪ features mapped by the
    binding's own default features).

Effective sets are compared as *sets*: `indicators-all` transitively enables
every `indicators-*` category, and those subcategories are an implementation
detail of the core feature tree, not part of the contract, so they are
normalised away.

Exit code 0 = every binding compiles what it declares. Any drift is printed
with the offending binding and feature.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

# ── The canonical contract (SSOT) ────────────────────────────────────────────
# Each entry: binding package name → binding manifest path (relative to REPO),
# the declared base features on its `finkit` dependency, and the *effective*
# set (base ∪ what the binding's own default features map onto finkit/*).
# Rationale (V5 Batch 5 / open question 5):
#   * every full-parity binding compiles `formula`; the frozen
#     `formula-jit` / `formula-simd` are added **only** where the binding's
#     exported API calls `eval_jit` / `eval_simd` (java, dotnet, python, node).
#   * `tracing` is a wasm-only need (browser panic/console wiring).
#   * ios/android deliberately expose a narrowed ABI subset
#     (indicators only), which stays 15/78 by design.
BINDING_CONTRACT: dict[str, dict[str, object]] = {
    # package:            manifest,                              base,                                  effective
    "finkit-ffi": {
        "manifest": "ffi/c-binding/Cargo.toml",
        "base": ["std", "serde", "formula", "indicators-all"],
        "effective": ["std", "serde", "formula", "indicators-all"],
    },
    "finkit-python": {
        "manifest": "ffi/python-binding/Cargo.toml",
        "base": ["std", "serde", "indicators-all"],
        # own `formula` feature maps formula+formula-jit+formula-simd;
        # own `rayon` feature maps rayon. Both are default-on.
        "effective": [
            "std", "serde", "indicators-all",
            "formula", "formula-jit", "formula-simd", "rayon",
        ],
    },
    "finkit-node": {
        "manifest": "ffi/node-binding/Cargo.toml",
        "base": ["std", "serde", "indicators-all"],
        "effective": ["std", "serde", "indicators-all", "formula", "formula-jit", "formula-simd"],
    },
    "finkit-go": {
        "manifest": "ffi/go-binding/Cargo.toml",
        "base": ["std", "serde", "formula", "indicators-all"],
        "effective": ["std", "serde", "formula", "indicators-all"],
    },
    "finkit-java": {
        "manifest": "ffi/java-binding/Cargo.toml",
        "base": ["std", "serde", "formula", "formula-jit", "formula-simd", "indicators-all"],
        "effective": ["std", "serde", "formula", "formula-jit", "formula-simd", "indicators-all"],
    },
    "finkit-dotnet": {
        "manifest": "ffi/dotnet-binding/Cargo.toml",
        "base": ["std", "serde", "formula", "formula-jit", "formula-simd", "indicators-all"],
        "effective": ["std", "serde", "formula", "formula-jit", "formula-simd", "indicators-all"],
    },
    "finkit-wasm": {
        "manifest": "wasm/Cargo.toml",
        "base": ["std", "formula", "serde", "tracing", "indicators-all"],
        "effective": ["std", "formula", "serde", "tracing", "indicators-all"],
    },
    "finkit-ios": {
        "manifest": "ffi/ios-binding/Cargo.toml",
        "base": ["indicators-all"],
        # The mobile shims link `finkit-ffi-common`, whose code needs core's
        # `formula` + `serde`; feature unification necessarily pulls both in.
        # Recorded here so the audit reflects reality instead of an
        # unreachable ideal — slimming further means making ffi-common's
        # formula/serde use optional first.
        "effective": ["std", "indicators-all", "formula", "serde"],
    },
    "finkit-android": {
        "manifest": "ffi/android-binding/Cargo.toml",
        "base": ["indicators-all"],
        # See finkit-ios: ffi-common transitively requires formula + serde.
        "effective": ["std", "indicators-all", "formula", "serde"],
    },
}

_INDICATOR_RE = re.compile(r"^indicators-")


def normalise(features: set[str]) -> set[str]:
    """Drop indicator subcategories — `indicators-all` stands for all of them."""
    return {f for f in features if not _INDICATOR_RE.match(f) or f == "indicators-all"}


def check_dep_line(pkg: str, entry: dict) -> list[str]:
    """The binding's finkit dependency must declare the contract verbatim."""
    errors = []
    manifest = REPO / entry["manifest"]
    text = manifest.read_text(encoding="utf-8")
    match = re.search(r'^finkit\s*=\s*\{(.*)\}\s*$', text, re.MULTILINE)
    if not match:
        return [f"{entry['manifest']}: no `finkit = {{ ... }}` dependency line found"]
    decl = match.group(1)
    if "default-features = false" not in decl:
        errors.append(
            f"{entry['manifest']}: finkit dependency must carry `default-features = false`"
        )
    features_match = re.search(r'features\s*=\s*\[([^\]]*)\]', decl)
    declared = set(re.findall(r'"([^"]+)"', features_match.group(1))) if features_match else set()
    expected_base = set(entry["base"])
    if declared != expected_base:
        errors.append(
            f"{entry['manifest']}: declared base features {sorted(declared)} != contract "
            f"{sorted(expected_base)}"
        )
    return errors


def actual_tree_features(pkg: str) -> set[str] | None:
    """Run cargo tree and harvest the enabled finkit feature names."""
    proc = subprocess.run(
        ["cargo", "tree", "-p", pkg, "-e", "features", "-i", "finkit"],
        cwd=REPO, capture_output=True, text=True,
    )
    if proc.returncode != 0:
        print(f"[FAIL] cargo tree -p {pkg} failed:\n{proc.stderr.strip()}")
        return None
    features = set()
    for line in proc.stdout.splitlines():
        match = re.search(r'finkit feature "([^"]+)"', line)
        if match:
            features.add(match.group(1))
    return features


def main() -> int:
    failures: list[str] = []
    for pkg, entry in BINDING_CONTRACT.items():
        failures += check_dep_line(pkg, entry)

        actual = actual_tree_features(pkg)
        if actual is None:
            failures.append(f"{pkg}: cargo tree could not resolve the feature graph")
            continue
        expected = normalise(set(entry["effective"]))
        got = normalise(actual)
        if got != expected:
            missing = sorted(expected - got)
            extra = sorted(got - expected)
            failures.append(
                f"{pkg}: compiled feature state drifts from the contract — "
                f"missing: {missing or '—'}, unexpected: {extra or '—'}"
            )
        else:
            print(f"[OK]   {pkg}: {sorted(got)}")

    if failures:
        print("\nBinding feature contract violated:")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print("\nAll bindings compile exactly the feature contract they declare.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
