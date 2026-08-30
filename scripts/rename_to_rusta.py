#!/usr/bin/env python3
"""一次性把项目品牌名从 AlphaTA / finkit 统一改为 Rusta。

设计要点
--------
* **默认 dry-run**：不带 `--apply` 只报告，不落盘。
* **顺序敏感**：映射表按“长 → 短、具体 → 通用”排列，保证
  ``alpha-ta-core`` 先于 ``alpha-ta`` 命中，避免产生 ``rusta-core`` 之类
  的二次污染。
* **保留 C ABI 前缀**：``ta_sma`` / ``ta_result_t`` 等公开 ABI 符号**不动**，
  因为它们不含 ``alpha``/``finkit`` 词根，天然不会被本表命中。
* **排除构建产物与历史档案**：见 ``EXCLUDE_DIRS`` / ``EXCLUDE_SUFFIXES``。

用法
----
    python scripts/rename_to_rusta.py            # dry-run
    python scripts/rename_to_rusta.py --apply    # 落盘
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from collections import Counter

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# ---------------------------------------------------------------------------
# 替换映射表 —— 顺序敏感，切勿随意重排
# ---------------------------------------------------------------------------
REPLACEMENTS: list[tuple[str, str]] = [
    # --- 1. Java / JNI 符号（最长，必须最先） -------------------------------
    # 项目历史上有三套包名并存：com.alphata（实际）、com.rusttalib（legacy）、
    # com.finkit（android 注释）。统一收敛到 com.rusta。
    ("Java_com_alphata_", "Java_com_rusta_"),
    ("Java_com_rusttalib_", "Java_com_rusta_"),
    ("com.finkit.indicators", "com.rusta.indicators"),
    ("com.alphata.indicators", "com.rusta.indicators"),
    ("com.finkit", "com.rusta"),
    ("com.alphata", "com.rusta"),
    # --- 2. crate 名（连字符形态） -----------------------------------------
    ("alpha-ta-ffi-common", "rusta-ffi-common"),
    ("alpha-ta-visualization", "rusta-visualization"),
    ("alpha-ta-python", "rusta-python"),
    ("alpha-ta-dotnet", "rusta-dotnet"),
    ("alpha-ta-android", "rusta-android"),
    ("alpha-ta-core", "rusta-core"),
    ("alpha-ta-cli", "rusta-cli"),
    ("alpha-ta-wasm", "rusta-wasm"),
    ("alpha-ta-node", "rusta-node"),
    ("alpha-ta-go", "rusta-go"),
    ("alpha-ta-ios", "rusta-ios"),
    ("alpha-ta-java", "rusta-java"),
    ("alpha-ta-ffi", "rusta-ffi"),
    # --- 3. Rust 模块路径（下划线形态） ------------------------------------
    ("alpha_ta_ffi_common", "rusta_ffi_common"),
    ("alpha_ta_visualization", "rusta_visualization"),
    ("alpha_ta_android", "rusta_android"),
    ("alpha_ta_ffi", "rusta_ffi"),
    ("alpha_ta_core", "rusta_core"),
    # --- 4. finkit 代码残留（工作区目录本身不改，只改代码里的引用） ---------
    ("finkit_android_abi_version", "rusta_android_abi_version"),
    ("finkit_android_version", "rusta_android_version"),
    ("finkit-android", "rusta-android"),
    ("finkit-visualization", "rusta-visualization"),
    ("finkit-java", "rusta-java"),
    ("finkit_java", "rusta_java"),
    ("finkit_core", "rusta_core"),
    ("finkit_visualization", "rusta_visualization"),
    # --- 5. 品牌名（大小写变体） -------------------------------------------
    ("AlphaTA", "Rusta"),
    ("AlphaTa", "Rusta"),
    ("ALPHA_TA", "RUSTA"),
    ("ALPHATA", "RUSTA"),
    ("alphata", "rusta"),
    # --- 6. 通用兜底（必须在所有具体项之后） -------------------------------
    ("alpha-ta", "rusta"),
    ("alpha_ta", "rusta"),
]

# ---------------------------------------------------------------------------
# 排除规则
# ---------------------------------------------------------------------------
EXCLUDE_DIRS = {
    # 构建产物
    "target", "target-verify", "dist", "node_modules", "__pycache__",
    "build-usage", ".test_venv", ".forge", ".aza",
    # 版本控制 / IDE 历史档案
    ".git", ".trae", ".workbuddy", ".vscode", ".idea",
    # .NET / CMake 中间产物
    "obj", "bin", "CMakeFiles",
}

EXCLUDE_SUFFIXES = {
    # 二进制 / 不可文本替换
    ".dll", ".so", ".dylib", ".a", ".lib", ".exe", ".pdb", ".o", ".bc",
    ".wasm", ".nupkg", ".parquet", ".png", ".jpg", ".jpeg", ".gif", ".ico",
    ".ttf", ".woff", ".woff2", ".zip", ".tar", ".gz", ".pdf", ".sqlite",
    ".rmeta", ".rlib", ".d",
}

# 历史档案：内容描述的是过去的决策，不应被“现代化”改写
EXCLUDE_PATH_MARKERS = (
    os.path.join("docs", "archive") + os.sep,
    os.path.join(".trae", "documents") + os.sep,
)

# 需要一并改名的文件 / 目录（相对 ROOT，按深度倒序处理）
RENAMES: list[tuple[str, str]] = [
    (os.path.join("ffi", "c-binding", "alpha_ta.pc.in"),
     os.path.join("ffi", "c-binding", "rusta.pc.in")),
    (os.path.join("ffi", "android-binding", "android", "src", "main", "java",
                  "com", "alphata", "indicators", "AlphaTA.java"),
     os.path.join("ffi", "android-binding", "android", "src", "main", "java",
                  "com", "rusta", "indicators", "Rusta.java")),
    (os.path.join("ffi", "android-binding", "android", "src", "main", "java",
                  "com", "alphata"),
     os.path.join("ffi", "android-binding", "android", "src", "main", "java",
                  "com", "rusta")),
    (os.path.join("ffi", "java-binding", "java", "src", "main", "java",
                  "com", "alphata"),
     os.path.join("ffi", "java-binding", "java", "src", "main", "java",
                  "com", "rusta")),
    (os.path.join("ffi", "dotnet-binding", "src", "AlphaTA"),
     os.path.join("ffi", "dotnet-binding", "src", "Rusta")),
    (os.path.join("examples", "java_example", "AlphaTAExample.java"),
     os.path.join("examples", "java_example", "RustaExample.java")),
    (os.path.join("docs", "ALPHATA_VS_TALIB.md"),
     os.path.join("docs", "RUSTA_VS_TALIB.md")),
    (os.path.join("docs", "migration", "pine-to-alphata.md"),
     os.path.join("docs", "migration", "pine-to-rusta.md")),
]


def should_skip(rel_path: str) -> bool:
    """判断该路径是否应跳过（构建产物 / 二进制 / 历史档案）。"""
    norm = rel_path.replace("\\", "/")
    parts = norm.split("/")
    if any(p in EXCLUDE_DIRS for p in parts):
        return True
    if any(m.replace("\\", "/") in norm for m in EXCLUDE_PATH_MARKERS):
        return True
    ext = os.path.splitext(norm)[1].lower()
    if ext in EXCLUDE_SUFFIXES:
        return True
    return False


def iter_text_files():
    """遍历所有需要处理的文本文件。"""
    for dirpath, dirnames, filenames in os.walk(ROOT):
        dirnames[:] = [d for d in dirnames if d not in EXCLUDE_DIRS]
        rel_dir = os.path.relpath(dirpath, ROOT)
        if rel_dir == ".":
            rel_dir = ""
        for fn in filenames:
            rel = os.path.join(rel_dir, fn) if rel_dir else fn
            if should_skip(rel):
                continue
            yield os.path.join(dirpath, fn), rel


def apply_text(text: str) -> tuple[str, Counter]:
    """对文本执行一次有序替换，返回 (新文本, 各模式命中次数)。"""
    hits: Counter = Counter()
    for old, new in REPLACEMENTS:
        n = text.count(old)
        if n:
            text = text.replace(old, new)
            hits[old] += n
    return text, hits


def revert(manifest_path: str) -> int:
    """按 manifest 精确回滚一次 --apply（含路径重命名）。"""
    import json
    import shutil

    with open(manifest_path, encoding="utf-8") as f:
        man = json.load(f)
    # 1) 先回滚路径重命名（new -> old）
    for old_rel, new_rel in reversed(man["renames"]):
        old_abs = os.path.join(ROOT, old_rel)
        new_abs = os.path.join(ROOT, new_rel)
        if os.path.exists(new_abs) and not os.path.exists(old_abs):
            os.makedirs(os.path.dirname(old_abs), exist_ok=True)
            os.rename(new_abs, old_abs)
            print(f"  path reverted: {new_rel} -> {old_rel}")
    # 2) 再回滚文件内容
    n = 0
    for rel in man["files"]:
        src = os.path.join(os.path.dirname(manifest_path), "files", rel)
        dst = os.path.join(ROOT, rel)
        if os.path.exists(src):
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            shutil.copy2(src, dst)
            n += 1
    print(f"reverted {n} files from {manifest_path}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description="Rename AlphaTA/finkit -> Rusta")
    ap.add_argument("--apply", action="store_true",
                    help="实际写入；默认只做 dry-run 报告")
    ap.add_argument("--revert", metavar="MANIFEST",
                    help="按 manifest.json 精确回滚上一次 --apply")
    args = ap.parse_args()

    if args.revert:
        return revert(args.revert)

    # --apply：先建立文件级备份（替换是多对一映射，反向替换不可逆）
    backup_files: dict[str, str] = {}
    if args.apply:
        import json
        import shutil
        import time

        ts = time.strftime("%Y%m%d-%H%M%S")
        bdir = os.path.join(ROOT, ".workbuddy", f"rename-backup-{ts}")
        os.makedirs(os.path.join(bdir, "files"), exist_ok=True)
        print(f"backup dir: {os.path.relpath(bdir, ROOT)}")

    total_files = 0
    changed_files = 0
    total_hits: Counter = Counter()
    changed_list: list[tuple[str, int]] = []

    for abs_p, rel in iter_text_files():
        try:
            with open(abs_p, encoding="utf-8") as f:
                text = f.read()
        except (UnicodeDecodeError, OSError):
            continue
        total_files += 1
        new_text, hits = apply_text(text)
        if not hits:
            continue
        changed_files += 1
        n = sum(hits.values())
        changed_list.append((rel, n))
        total_hits.update(hits)
        if args.apply:
            bpath = os.path.join(bdir, "files", rel)
            os.makedirs(os.path.dirname(bpath), exist_ok=True)
            shutil.copy2(abs_p, bpath)
            backup_files[rel] = rel
            with open(abs_p, "w", encoding="utf-8", newline="") as f:
                f.write(new_text)

    mode = "APPLIED" if args.apply else "DRY-RUN"
    print(f"=== rename_to_rusta.py  [{mode}] ===")
    print(f"scanned files : {total_files}")
    print(f"changed files : {changed_files}")
    print(f"total hits    : {sum(total_hits.values())}")
    print("\n--- hits by pattern ---")
    for old, new in REPLACEMENTS:
        c = total_hits.get(old, 0)
        if c:
            print(f"  {old:26s} -> {new:22s} {c:5d}")
    print(f"\n--- top 20 changed files ---")
    for rel, n in sorted(changed_list, key=lambda x: -x[1])[:20]:
        print(f"  {n:5d}  {rel}")

    # --- 文件 / 目录重命名 -------------------------------------------------
    print("\n--- path renames ---")
    for old_rel, new_rel in RENAMES:
        old_abs = os.path.join(ROOT, old_rel)
        new_abs = os.path.join(ROOT, new_rel)
        if not os.path.exists(old_abs):
            print(f"  [skip, absent] {old_rel}")
            continue
        print(f"  {old_rel}  ->  {new_rel}")
        if args.apply:
            os.makedirs(os.path.dirname(new_abs), exist_ok=True)
            os.rename(old_abs, new_abs)

    if not args.apply:
        print("\n(dry-run: nothing written. Re-run with --apply to commit changes.)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
