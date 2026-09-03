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

    replace_once(
        "core/tests/common/golden_loader.rs",
        '''/// 百分比模式容差（hv / natr 等以百分号为单位的指标）\npub const PERCENT_TOLERANCE: f64 = 1e-6;\n\n''',
        "",
        "unused golden percentage tolerance",
    )

    replace_once(
        "core/tests/common/mod.rs",
        '''//! 子模块：\n//! - [`golden_loader`] — 黄金 CSV 加载与容差断言 helper\n//! - [`property_templates`] — proptest 不变量断言 helper（Phase 9 使用）\n\npub mod golden_loader;\npub mod property_templates;''',
        '''//! 子模块：\n//! - [`golden_loader`] — 黄金 CSV 加载与容差断言 helper\n\npub mod golden_loader;''',
        "unused property helper module export",
    )

    property_templates = ROOT / "core/tests/common/property_templates.rs"
    if not property_templates.exists():
        raise SystemExit("property_templates.rs: expected unused shared helper file to exist")
    property_templates.unlink()
    print("removed unused core/tests/common/property_templates.rs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
