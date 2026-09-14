#!/usr/bin/env python3
"""Make the Rust source scanners lifetime-safe before applying migrations.

Rust lifetimes such as ``Python<'_>`` use a single apostrophe and are not
single-quoted strings.  The migration scanners only need to skip Rust string
literals, so treating apostrophes as quotes can make parenthesis/brace matching
run past the end of a function.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FILES = [
    ROOT / "scripts/optimize_python_bindings.py",
    ROOT / "scripts/apply_talib_semantic_fixes.py",
    ROOT / "scripts/apply_formula_runtime_performance_fixes.py",
]


def main() -> int:
    old = "if ch in ('\\\"', \"'\"):\n"
    new = "if ch == '\\\"':\n"
    changed = 0
    for path in FILES:
        text = path.read_text(encoding="utf-8")
        count = text.count(old)
        if count:
            path.write_text(text.replace(old, new), encoding="utf-8")
            changed += count
            print(f"{path.relative_to(ROOT)}: fixed {count} Rust quote scanners")
        else:
            print(f"{path.relative_to(ROOT)}: already lifetime-safe")
    print(f"total scanner fixes: {changed}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
