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


def remove_file(path: str, label: str) -> None:
    file = ROOT / path
    if not file.exists():
        raise SystemExit(f"{label}: expected {path} to exist")
    file.unlink()
    print(f"removed {label}: {path}")


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

    remove_file(
        "core/tests/common/property_templates.rs",
        "unused property helper module",
    )
    remove_file(
        "core/tests/common/streaming_test_templates.rs",
        "broken orphan streaming template file",
    )

    replace_once(
        "core/tests/golden_talib_tests.rs",
        '''struct Ohlcv {\n    open: Vec<f64>,\n    high: Vec<f64>,\n''',
        '''struct Ohlcv {\n    high: Vec<f64>,\n''',
        "unused Ohlcv open field",
    )
    replace_once(
        "core/tests/golden_talib_tests.rs",
        '''    let mut open = Vec::new();\n    let mut high = Vec::new();\n''',
        '''    let mut high = Vec::new();\n''',
        "unused open fixture buffer",
    )
    replace_once(
        "core/tests/golden_talib_tests.rs",
        '''        open.push(parts[col_index["open"]].trim().parse().expect("open"));\n        high.push(parts[col_index["high"]].trim().parse().expect("high"));\n''',
        '''        let _: f64 = parts[col_index["open"]].trim().parse().expect("open");\n        high.push(parts[col_index["high"]].trim().parse().expect("high"));\n''',
        "open fixture validation without dead storage",
    )
    replace_once(
        "core/tests/golden_talib_tests.rs",
        '''    Ohlcv {\n        open,\n        high,\n''',
        '''    Ohlcv {\n        high,\n''',
        "Ohlcv initializer open field",
    )

    replace_once(
        "core/tests/TEST_INDEX.md",
        '''- `property_tests.rs` - 流式指标属性测试\n- `common/property_templates.rs` - 属性测试模板\n''',
        '''- `property_tests.rs` - 流式指标属性测试\n''',
        "stale property helper index entry",
    )
    replace_once(
        "core/tests/TEST_INDEX.md",
        '''- `mod.rs` - 模块导出\n- `golden_loader.rs` - 黄金测试数据加载\n- `property_templates.rs` - 属性测试模板\n- `streaming_test_templates.rs` - 流式指标测试模板宏\n''',
        '''- `mod.rs` - 模块导出\n- `golden_loader.rs` - 黄金测试数据加载\n''',
        "stale common helper index entries",
    )

    index_path = ROOT / "core/tests/TEST_INDEX.md"
    index = index_path.read_text(encoding="utf-8")
    section_start = index.index("## 测试模板使用\n")
    section_end = index.index("## 测试覆盖统计\n", section_start)
    index = index[:section_start] + index[section_end:]
    index = index.replace(
        "1. **使用模板宏**: 优先使用 `streaming_test_templates.rs` 中的宏\n",
        "1. **共享工具最小化**: `tests/common/` 只保留被现有多个测试实际使用的 helper\n",
        1,
    )
    index_path.write_text(index, encoding="utf-8")
    print("removed stale streaming template documentation")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
