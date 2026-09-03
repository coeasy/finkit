#!/usr/bin/env python3
"""One-shot branch maintenance for the cross-platform warning cleanup.

This file is intentionally temporary. It performs asserted, deterministic
source transformations that are awkward to express through GitHub's whole-file
contents API, then the maintenance workflow deletes it before the PR is merged.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


def rewrite_simd() -> None:
    path = ROOT / "core/src/formula/simd.rs"
    text = path.read_text(encoding="utf-8")

    text = replace_once(
        text,
        '''        #[cfg(target_arch = "aarch64")]
        {
            return SimdLevel::Neon;
        }
        SimdLevel::Scalar
''',
        '''        #[cfg(target_arch = "aarch64")]
        {
            SimdLevel::Neon
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            SimdLevel::Scalar
        }
''',
        "simd_level aarch64 dispatch",
    )

    dispatches = [
        ("add_neon(a, b, result)", "add_fallback(a, b, result)"),
        ("sub_neon(a, b, result)", "sub_fallback(a, b, result)"),
        ("mul_neon(a, b, result)", "mul_fallback(a, b, result)"),
        ("div_neon(a, b, result)", "div_fallback(a, b, result)"),
        ("mod_neon(a, b, result)", "mod_fallback(a, b, result)"),
        ("gt_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a > b)"),
        ("lt_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a < b)"),
        ("gte_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a >= b)"),
        ("lte_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a <= b)"),
        ("eq_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a == b)"),
        ("neq_neon(a, b, result)", "cmp_fallback(a, b, result, |a, b| a != b)"),
        ("abs_neon(data, result)", "abs_fallback(data, result)"),
        ("max_elementwise_neon(a, b, result)", "max_elementwise_fallback(a, b, result)"),
        ("min_elementwise_neon(a, b, result)", "min_elementwise_fallback(a, b, result)"),
        ("hhv_neon(data, period, result)", "hhv_fallback(data, period, result)"),
        ("llv_neon(data, period, result)", "llv_fallback(data, period, result)"),
    ]

    for neon_call, fallback_call in dispatches:
        old = f'''        #[cfg(target_arch = "aarch64")]
        {{
            return unsafe {{ {neon_call} }};
        }}
        {fallback_call}
'''
        new = f'''        #[cfg(target_arch = "aarch64")]
        {{
            unsafe {{ {neon_call} }}
        }}
        #[cfg(not(target_arch = "aarch64"))]
        {{
            {fallback_call}
        }}
'''
        text = replace_once(text, old, new, f"SIMD dispatch {neon_call}")

    old_select = '''        #[cfg(target_arch = "aarch64")]
        {
            return unsafe { select_neon(condition, then_val, else_val, result, len) };
        }
        for i in 0..len {
            result[i] = if condition[i] != 0.0 {
                then_val[i]
            } else {
                else_val[i]
            };
        }
'''
    new_select = '''        #[cfg(target_arch = "aarch64")]
        {
            unsafe { select_neon(condition, then_val, else_val, result, len) }
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            for i in 0..len {
                result[i] = if condition[i] != 0.0 {
                    then_val[i]
                } else {
                    else_val[i]
                };
            }
        }
'''
    text = replace_once(text, old_select, new_select, "SIMD select dispatch")

    path.write_text(text, encoding="utf-8")
    print("updated core/src/formula/simd.rs: 18 aarch64 unreachable paths removed")


def rewrite_statistics() -> None:
    path = ROOT / "core/src/indicators/statistics.rs"
    text = path.read_text(encoding="utf-8")
    old = '''#[deprecated(since = "0.2.0", note = "Use `linearreg` (TA-Lib naming convention)")]
pub fn linear_reg(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
'''
    new = '''///
/// Legacy spelling retained for source compatibility in the 0.1.x line.
/// New internal code and generated bindings use [`linearreg`].
pub fn linear_reg(input: &[f64], timeperiod: usize) -> Result<Array1<f64>> {
'''
    text = replace_once(text, old, new, "linear_reg premature deprecation")
    path.write_text(text, encoding="utf-8")
    print("updated statistics.rs: compatibility alias is warning-free until its actual transition")


def rewrite_generated_bindings() -> None:
    old = "indicators::linear_reg("
    new = "indicators::linearreg("
    replacements = 0
    changed: list[str] = []
    for path in sorted((ROOT / "ffi").glob("**/generated.rs")):
        text = path.read_text(encoding="utf-8")
        count = text.count(old)
        if not count:
            continue
        replacements += count
        path.write_text(text.replace(old, new), encoding="utf-8")
        changed.append(str(path.relative_to(ROOT)))
    if replacements < 1:
        raise SystemExit("generated bindings: expected at least one indicators::linear_reg call")
    print(f"updated generated bindings: canonicalized {replacements} call(s) in {', '.join(changed)}")


def make_sync_bindings_fail_closed() -> None:
    path = ROOT / "scripts/sync_bindings.py"
    text = path.read_text(encoding="utf-8")
    old = '''def indicators_with_ffi(reg: dict) -> list[dict]:
    out = [i for i in reg.get("indicators", []) if i.get("ffi", {}).get("c_name")]
    out.sort(key=lambda i: i["ffi"].get("order", 0))
    return out
'''
    new = '''def indicators_with_ffi(reg: dict) -> list[dict]:
    out = [i for i in reg.get("indicators", []) if i.get("ffi", {}).get("c_name")]
    if not out:
        raise SystemExit(
            "docs/indicator_registry.json has no ffi metadata; refusing to generate or "
            "validate empty binding files. Run scripts/enrich_registry_ffi.py only as an "
            "intentional registry migration, then review the resulting diff before using "
            "sync_bindings.py."
        )
    out.sort(key=lambda i: i["ffi"].get("order", 0))
    return out
'''
    text = replace_once(text, old, new, "sync_bindings empty-registry guard")
    path.write_text(text, encoding="utf-8")
    print("updated sync_bindings.py: fail closed when registry lacks FFI metadata")


def main() -> int:
    rewrite_simd()
    rewrite_statistics()
    rewrite_generated_bindings()
    make_sync_bindings_fail_closed()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
