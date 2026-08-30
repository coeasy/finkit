#!/usr/bin/env python3
"""A5 audit: scan the workspace for unwrap()/expect()/panic! etc., bucketed
by (crate, kind) and by context (production / test / ffi-guarded).

Outputs a markdown report + prints a short summary. Not committed; it's a
diagnostic tool for the unwrap-governance workstream.
"""
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

ROOTS = [
    "core/src",
    "visualization/src",
    "cli/src",
    "wasm/src",
    "ffi/c-binding/src",
    "ffi/python-binding/src",
    "ffi/node-binding/src",
    "ffi/go-binding/src",
    "ffi/dotnet-binding/src",
    "ffi/ios-binding/src",
    "ffi/java-binding/src",
    "ffi/android-binding/src",
    "ffi/ffi-common/src",
]

# user-input-facing surface markers (highest risk: a panic here hits real input)
USER_INPUT_DIRS = ("formula", "parser", "eval", "cli", "json", "deserialize", "params")

DANGER_PATTERNS = {
    "panic!": re.compile(r"\bpanic!\s*\("),
    "unwrap()": re.compile(r"\.unwrap\s*\(\s*\)"),
    "expect(": re.compile(r"\.expect\s*\("),
    "unreachable!": re.compile(r"\bunreachable!\s*\("),
    "unimplemented!": re.compile(r"\bunimplemented!\s*\("),
    "todo!": re.compile(r"\btodo!\s*\("),
    "unwrap_unchecked": re.compile(r"\.unwrap_unchecked\s*\(\s*\)"),
}
SAFE_FALLBACK = re.compile(r"\.unwrap_or(_else)?\s*\(")


def strip_strings(s):
    # remove "..." and '...' (char/str) literals to avoid counting inside them
    s = re.sub(r'"[^"]*"', '""', s)
    s = re.sub(r"'[^']*'", "''", s)
    return s


def strip_comments(s, state):
    # state: dict with 'bc' (in block comment bool)
    out = []
    i = 0
    while i < len(s):
        if state["bc"]:
            if s[i : i + 2] == "*/":
                state["bc"] = False
                i += 2
                continue
            i += 1
            continue
        if s[i : i + 2] == "/*":
            state["bc"] = True
            i += 2
            continue
        if s[i : i + 2] == "//":
            break
        out.append(s[i])
        i += 1
    return "".join(out)


def bucket_for(path):
    norm = path.replace("\\", "/")
    if "/tests/" in norm or norm.endswith("/tests") or "/benches/" in norm:
        return "test"
    if re.search(r"/ffi/[^/]+/src/(generated\.rs|lib\.rs)$", norm):
        return "ffi"
    return "production"


def scan_file(path):
    with open(path, "r", encoding="utf-8", errors="replace") as f:
        lines = f.readlines()
    state = {"bc": False}
    brace = 0
    test_stack = [False]  # current is-test context
    pending_cfg = False
    entering_tests = False
    file_bucket = bucket_for(path)
    results = []  # (lineno, category, text)
    for idx, raw in enumerate(lines, 1):
        line = strip_strings(raw)
        clean = strip_comments(line, state)
        # detect test-module entry
        if re.search(r"#\[\s*cfg\s*\(\s*test\s*\)\s*\]", clean):
            pending_cfg = True
        if re.search(r"\bmod\s+tests\b", clean):
            entering_tests = True
        # brace tracking
        for ch in clean:
            if ch == "{":
                brace += 1
                if pending_cfg or entering_tests:
                    test_stack.append(True)
                else:
                    test_stack.append(test_stack[-1])
                pending_cfg = False
                entering_tests = False
            elif ch == "}":
                if len(test_stack) > 1:
                    test_stack.pop()
                if brace > 0:
                    brace -= 1
        is_test = test_stack[-1] or (file_bucket == "test")
        # pattern matching
        for cat, pat in DANGER_PATTERNS.items():
            if pat.search(clean):
                results.append((idx, cat, raw.rstrip("\n"), is_test))
        if SAFE_FALLBACK.search(clean):
            results.append((idx, "unwrap_or", raw.rstrip("\n"), is_test))
    return file_bucket, test_stack[-1], results


def is_test_any(_r, _ts):
    return _ts[-1]


def crate_of(path):
    # NOTE: paths are absolute (Windows: "P:\\repo\\core\\src\\..."), so we must
    # relativise against ROOT first — splitting the raw path yields the drive
    # letter ("P:") as the crate name. See 2026-08-30 fix.
    try:
        rel = os.path.relpath(path, ROOT)
    except ValueError:
        rel = path
    norm = rel.replace("\\", "/")
    parts = [p for p in norm.split("/") if p not in ("", "..")]
    if "ffi" in parts:
        i = parts.index("ffi")
        return "ffi/" + parts[i + 1] if i + 1 < len(parts) else "ffi"
    return parts[0] if parts else "unknown"


def main():
    per_crate = defaultdict(lambda: defaultdict(lambda: defaultdict(int)))  # crate -> bucket -> cat -> count
    per_crate_file = defaultdict(lambda: defaultdict(int))
    danger_lines = []  # (path, lineno, cat, text)
    for root in ROOTS:
        base = os.path.join(ROOT, root)
        if not os.path.isdir(base):
            continue
        for dirpath, _dirs, files in os.walk(base):
            for fn in files:
                if not fn.endswith(".rs"):
                    continue
                full = os.path.join(dirpath, fn)
                fbucket, _istest, res = scan_file(full)
                crate = crate_of(full)
                # effective bucket: ffi stays ffi; test file stays test;
                # otherwise if inside test module mark test.
                for (ln, cat, text, is_test_line) in res:
                    # determine effective bucket for this line
                    if fbucket == "ffi":
                        eb = "ffi"
                    elif fbucket == "test" or is_test_line:
                        eb = "test"
                    else:
                        eb = "production"
                    per_crate[crate][eb][cat] += 1
                    # NOTE: key by effective bucket, otherwise test-module hits
                    # leak into the "top production files" list (see 2026-08-30 fix).
                    per_crate_file[crate + "|" + eb + "|" + full][cat] += 1
                    if cat in DANGER_PATTERNS and eb == "production":
                        danger_lines.append((full, ln, cat, text))
    # print summary
    print("=== A5 unwrap/panic audit ===")
    cats_order = ["panic!", "unwrap()", "expect(", "unreachable!",
                  "unimplemented!", "todo!", "unwrap_unchecked", "unwrap_or"]
    print(f"{'crate':<22}{'bucket':<12}" + "".join(f"{c:<14}" for c in cats_order))
    total = defaultdict(int)
    for crate in sorted(per_crate):
        for bucket in ("production", "test", "ffi"):
            row = per_crate[crate].get(bucket, {})
            if not row:
                continue
            cells = "".join(f"{row.get(c,0):<14}" for c in cats_order)
            print(f"{crate:<22}{bucket:<12}{cells}")
            for c in cats_order:
                total[(bucket, c)] += row.get(c, 0)
    print("-" * 22)
    print("TOTAL")
    for bucket in ("production", "test", "ffi"):
        cells = "".join(f"{total[(bucket,c)]:<14}" for c in cats_order)
        print(f"{'':<22}{bucket:<12}{cells}")

    # top production files by danger count
    print("\n=== Top production files by DANGER count (unwrap/panic/expect/...) ===")
    file_scores = []
    for key, counts in per_crate_file.items():
        crate, bucket, full = key.split("|", 2)
        if bucket != "production":
            # test code panics are fine; ffi code is panic-guarded (A3).
            continue
        danger = sum(counts.get(c, 0) for c in DANGER_PATTERNS if c != "unwrap_or")
        if danger == 0:
            continue
        file_scores.append((danger, full, counts))
    file_scores.sort(reverse=True)
    for danger, full, counts in file_scores[:25]:
        rel = os.path.relpath(full, ROOT)
        u = counts.get("unwrap()", 0)
        e = counts.get("expect(", 0)
        p = counts.get("panic!", 0)
        print(f"  {danger:>4}  {rel}  (unwrap={u} expect={e} panic={p})")

    # user-input-facing danger lines
    print("\n=== User-input-facing danger lines (formula/parser/eval/cli/json) ===")
    ui = [d for d in danger_lines if any(m in d[0].replace("\\", "/") for m in USER_INPUT_DIRS)]
    print(f"  count={len(ui)}")
    for full, ln, cat, text in ui[:40]:
        rel = os.path.relpath(full, ROOT)
        print(f"  {rel}:{ln} [{cat}] {text.strip()[:90]}")

    # write markdown report
    out = os.path.join(ROOT, "docs", "A5_UNWRAP_AUDIT.md")
    with open(out, "w", encoding="utf-8") as f:
        f.write("# A5 — unwrap()/panic! 治理审计\n\n")
        f.write("_Generated by `scripts/_a5_scan.py`. Re-run any time to refresh._\n\n")
        f.write("## 总览（按 crate × bucket）\n\n")
        f.write("| crate | bucket | " + " | ".join(cats_order) + " |\n")
        f.write("|" + "---|" * (len(cats_order) + 2) + "\n")
        for crate in sorted(per_crate):
            for bucket in ("production", "test", "ffi"):
                row = per_crate[crate].get(bucket, {})
                if not row:
                    continue
                cells = " | ".join(str(row.get(c, 0)) for c in cats_order)
                f.write(f"| {crate} | {bucket} | {cells} |\n")
        f.write("\n## 生产代码危险点 Top 文件（已排除 test/ffi）\n\n")
        f.write("| # | 文件 | 危险点 | unwrap() | expect( | panic! |\n")
        f.write("|---|---|---:|---:|---:|---:|\n")
        for i, (danger, full, counts) in enumerate(file_scores[:25], 1):
            rel = os.path.relpath(full, ROOT).replace("\\", "/")
            f.write(
                f"| {i} | `{rel}` | {danger} | {counts.get('unwrap()',0)} | "
                f"{counts.get('expect(',0)} | {counts.get('panic!',0)} |\n"
            )
        f.write("\n## 用户可达入口的危险点（formula/parser/eval/cli/json）\n\n")
        f.write(f"共 {len(ui)} 处。这些是生产路径里最该优先治理的（输入来自用户/JSON）。\n\n")
        for full, ln, cat, text in ui:
            rel = os.path.relpath(full, ROOT)
            f.write(f"- `{rel}:{ln}` `{cat}` — `{text.strip()[:120]}`\n")
    print(f"\nreport written: {out}")


if __name__ == "__main__":
    main()
