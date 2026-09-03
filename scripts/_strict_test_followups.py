#!/usr/bin/env python3
"""One-shot follow-up fixes discovered by the strict test compiler.

Temporary branch helper; removed before PR creation.
"""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: str, old: str, new: str, label: str) -> None:
    file = ROOT / path
    text = file.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match in {path}, found {count}")
    file.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"fixed {label}")


def main() -> int:
    replace_once(
        "core/tests/pine_corpus_runner.rs",
        "use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine};\n",
        "use finkit::formula::{FormulaContext, FormulaEngine};\n",
        "unused Pine corpus FormulaDialect import",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
