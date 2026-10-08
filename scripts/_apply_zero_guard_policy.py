#!/usr/bin/env python3
"""Two-band zero-guard codemod (V5 R1-2, revised after the golden corpus spoke).

The first attempt replaced every `1e-15` comparison in the tree with the
TA-Lib `TA_IS_ZERO` band. `core/tests/alpha158_parity.rs` failed: the Alpha158
factor library regularises denominators with Qlib's `+1e-12` idiom, so a
*normal* divisor sits at ~1e-12 and a 1e-8 band turns most of the library
into `NaN`. The formula runtime's guard is finkit's own contract, not a TA-Lib
parity knob, and widening it is a behaviour change with no parity claim.

So the two bands are a contract distinction, and this script encodes it:

  Surface A -- TA-Lib parity surface: `core/src/indicators/**`,
              `core/src/streaming/**`, `core/src/math/kernels/**`
      `x.abs() > 1e-15`   -> `!crate::utils::is_zero(x)`
      `x.abs() <= 1e-15`  -> `crate::utils::is_zero(x)`
      `x > 1e-15`         -> `x > crate::utils::TA_IS_ZERO_BANDWIDTH`   (semantic change: aligned to TA-Lib)

  Surface B -- everything else (formula runtime, features, transforms, risk)
      `1e-15` -> `crate::utils::NUMERIC_EPSILON`                        (pure rename, no numeric change)
      `1e-8`  -> `crate::utils::TA_IS_ZERO_BANDWIDTH`                    (pure rename, no numeric change)

`#[cfg(test)]` modules, comments, and assertion tolerances are left alone.
`scripts/check_zero_guard_policy.py` then keeps both bands from ever being
spelled inline again.

Usage: python scripts/_apply_zero_guard_policy.py [--apply]
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path("core/src")
# Surface A: output is contractually "the number TA-Lib prints".
TA_LIB_SURFACES = (
    ROOT / "indicators",
    ROOT / "streaming",
    ROOT / "math" / "kernels",
)

EXPR = r"[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*|\[[^\[\]]*\]|\([^()]*\))*"
ABS_RE = re.compile(rf"({EXPR})\.abs\(\)\s*(>=|<=|>|<)\s*1e-15")
BARE_RE = re.compile(rf"({EXPR})\s*(>=|<=|>|<)\s*1e-15")
SIMD_RE = re.compile(
    r"(_mm256_set1_pd|_mm512_set1_pd|_mm_set1_pd|vdupq_n_f64)\(1e-15\)"
)
# Surface B literal renames: any comparison against a bare near-zero literal.
B_LITERAL_RE = re.compile(r"(?<![0-9A-Za-z_.])(1e-15|1e-8)(?![0-9A-Za-z_])")
SKIP_MARKERS = ("assert_relative_eq", "assert_abs_diff_eq", "epsilon =", "approx")
STR = re.compile(r'"(?:\\.|[^"\\])*"')


def _strip_literals(line: str) -> str:
    line = STR.sub('""', line)
    return re.sub(r"'(?:\\.|[^'\\])'", "''", line)


def test_module_mask(text: str) -> list[bool]:
    """Per-line mask: True for lines inside a `#[cfg(test)] mod ... { }` block."""
    lines = text.splitlines(keepends=True)
    mask = [False] * len(lines)
    depth = 0
    pending = False
    for i, raw in enumerate(lines):
        code = _strip_literals(raw)
        stripped = code.strip()
        if depth == 0:
            if stripped.startswith("#[cfg(test)]"):
                rest = stripped[len("#[cfg(test)]") :].strip()
                if re.match(r"(pub\s+)?mod\s+\w+\s*\{", rest):
                    depth = code.count("{") - code.count("}")
                    mask[i] = True
                    pending = False
                    continue
                pending = True
                continue
            if pending:
                pending = False
                if re.match(r"(pub\s+)?mod\s+\w+\s*\{", stripped):
                    depth = code.count("{") - code.count("}")
                    mask[i] = True
            continue
        mask[i] = True
        depth += code.count("{") - code.count("}")
    return mask


def surface_a(path: pathlib.Path) -> bool:
    p = path.as_posix()
    return any(p.startswith(s.as_posix() + "/") for s in TA_LIB_SURFACES)


def transform_surface_a(line: str) -> tuple[str, int]:
    n = 0

    def _abs_sub(m: re.Match) -> str:
        nonlocal n
        expr, op = m.group(1), m.group(2)
        n += 1
        if op in (">", ">="):
            return f"!crate::utils::is_zero({expr})"
        return f"crate::utils::is_zero({expr})"

    line = ABS_RE.sub(_abs_sub, line)

    def _bare_sub(m: re.Match) -> str:
        nonlocal n
        n += 1
        return f"{m.group(1)} {m.group(2)} crate::utils::TA_IS_ZERO_BANDWIDTH"

    line = BARE_RE.sub(_bare_sub, line)

    def _simd_sub(m: re.Match) -> str:
        nonlocal n
        n += 1
        return f"{m.group(1)}(crate::utils::TA_IS_ZERO_BANDWIDTH)"

    line = SIMD_RE.sub(_simd_sub, line)
    return line, n


def transform_surface_b(line: str) -> tuple[str, int]:
    n = 0

    def _lit_sub(m: re.Match) -> str:
        nonlocal n
        n += 1
        return (
            "crate::utils::NUMERIC_EPSILON"
            if m.group(1) == "1e-15"
            else "crate::utils::TA_IS_ZERO_BANDWIDTH"
        )

    return B_LITERAL_RE.sub(_lit_sub, line), n


def main() -> int:
    apply = "--apply" in sys.argv
    counts = {"A": 0, "B": 0}
    files = {"A": 0, "B": 0}
    for path in sorted(ROOT.rglob("*.rs")):
        text = path.read_text(encoding="utf-8")
        if "1e-15" not in text and "1e-8" not in text:
            continue
        is_a = surface_a(path)
        transform = transform_surface_a if is_a else transform_surface_b
        bucket = "A" if is_a else "B"
        mask = test_module_mask(text)
        out_lines = []
        changed = False
        for idx, line in enumerate(text.splitlines(keepends=True)):
            if mask[idx] or line.strip().startswith("//"):
                out_lines.append(line)
                continue
            if any(m in line for m in SKIP_MARKERS):
                out_lines.append(line)
                continue
            new, n = transform(line)
            if n:
                changed = True
                counts[bucket] += n
                if not apply:
                    print(f"  [{bucket}] {path}:{idx + 1}: {line.strip()[:70]}")
                    print(f"        -> {new.strip()[:70]}")
            out_lines.append(new)
        if changed:
            files[bucket] += 1
            if apply:
                path.write_text("".join(out_lines), encoding="utf-8")

    print(
        f"\n{'APPLIED' if apply else 'DRY-RUN'}: "
        f"surface A (TA-Lib band) {counts['A']} sites / {files['A']} files; "
        f"surface B (engine band) {counts['B']} sites / {files['B']} files"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
