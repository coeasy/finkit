#!/usr/bin/env python3
"""Enforce the single zero-comparison policy point (V5 R1-2).

TA-Lib defines `TA_IS_ZERO(v) = ((v) > -1e-8) && ((v) < 1e-8)` in
`ta_utility.h`. Finkit historically spelled that guard inline as a comparison
against a hand-picked `1e-15` literal, ~670 times, so a denominator in
`(1e-15, 1e-8)` was divided here but judged zero by TA-Lib and the two outputs
diverged on inputs no golden corpus covered.

Every one of those guards now routes through `utils::is_zero` (absolute form)
or `utils::TA_IS_ZERO_BANDWIDTH` (signed / positive-only form), which are the
only two places the band width is written down. This gate keeps it that way: a
new inline `1e-15` (or `1e-8`) comparison is a policy regression, not a style
preference.

Exit codes: 0 = clean, 1 = violations found.
"""
from __future__ import annotations

import pathlib
import re
import sys

ROOT = pathlib.Path("core/src")
# The one file allowed to spell the band width out.
POLICY_FILE = ROOT / "utils.rs"

# Any comparison against a bare near-zero literal.
GUARD_RE = re.compile(
    r"(?:[<>]=?)\s*(?:1e-15|1e-8\b|0\.0000000?1\b|1\.\d*e-0?8\b)"
)
# Comparisons that are legitimately a *tolerance*, not a zero judgement.
TOLERANCE_MARKERS = (
    "assert_relative_eq",
    "assert_abs_diff_eq",
    "assert_approx_eq",
    "epsilon =",
    "tolerance",
    "approx",
)
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


def main() -> int:
    violations: list[tuple[str, int, str]] = []
    for path in sorted(ROOT.rglob("*.rs")):
        if path == POLICY_FILE:
            continue
        text = path.read_text(encoding="utf-8")
        if "1e-15" not in text and "1e-8" not in text:
            continue
        mask = test_module_mask(text)
        for idx, line in enumerate(text.splitlines()):
            if mask[idx]:
                continue
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            if any(m in line for m in TOLERANCE_MARKERS):
                continue
            if GUARD_RE.search(line):
                violations.append((str(path), idx + 1, stripped))

    if violations:
        print(
            f"[check_zero_guard_policy] FAIL: {len(violations)} inline "
            f"near-zero comparison(s) bypass the two named bands.\n"
            f"  TA-Lib parity surface (indicators/, streaming/, math/kernels/):\n"
            f"    absolute test  -> `crate::utils::is_zero(x)`\n"
            f"    signed test    -> `x <op> crate::utils::TA_IS_ZERO_BANDWIDTH`\n"
            f"  formula runtime / analytics:\n"
            f"    `1e-15` -> `crate::utils::NUMERIC_EPSILON`\n"
            f"  See core/src/utils.rs for why the two bands differ:"
        )
        for path, line_no, text in violations[:40]:
            print(f"  {path}:{line_no}: {text[:100]}")
        if len(violations) > 40:
            print(f"  ... and {len(violations) - 40} more")
        return 1

    print(
        "[check_zero_guard_policy] OK: every zero comparison routes through a "
        "named band (utils::is_zero / TA_IS_ZERO_BANDWIDTH / NUMERIC_EPSILON)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
