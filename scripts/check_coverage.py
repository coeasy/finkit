#!/usr/bin/env python3
"""指标覆盖度检查：registry (SSOT) x streaming x batch。

动机：docs/REFACTORING_PLAN_2026-08.md 3.5 节要求"先建立可信度量，再排期补实现"。
此前用朴素字符串相等比对得到"流式覆盖 64%"，但因命名不匹配（Bollinger Bands ->
StreamingBoll、Williams %R -> StreamingWillr）存在大量假阳性。

本脚本做三级匹配：exact -> prefix -> substring，并输出 residual 缺失清单。
模糊匹配（prefix/substr）会明确标注，避免把"猜的"当成"确认的"。

用法：
    python scripts/check_coverage.py            # 汇总 + 缺失清单
    python scripts/check_coverage.py --strict   # 只认 exact（下界估计）
    python scripts/check_coverage.py --json     # 机器可读输出（供 CI 消费）
"""
from __future__ import annotations

import json
import os
import re
import sys
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REGISTRY = os.path.join(ROOT, "docs", "indicator_registry.json")
STREAMING_DIR = os.path.join(ROOT, "core", "src", "streaming")

# batch 侧不止 indicators/：CDL_* 形态在 patterns/candlestick.rs，
# 部分统计/回归在 features/、math/。漏扫会把已有实现误报为缺失。
BATCH_DIRS = [
    os.path.join(ROOT, "core", "src", "indicators"),
    os.path.join(ROOT, "core", "src", "patterns"),
    os.path.join(ROOT, "core", "src", "math"),
    os.path.join(ROOT, "core", "src", "features"),
    os.path.join(ROOT, "core", "src", "transforms"),
]

# 公式层注册表名（第三维度）：部分指标只有公式层实现，无 Rust 原生 API。
FORMULA_FUNCTIONS = os.path.join(ROOT, "core", "src", "formula", "functions.rs")


def norm(s: str) -> str:
    """归一化符号名：去非字母数字 + 小写。'Williams %R' -> 'williamsr'。"""
    return re.sub(r"[^a-z0-9]", "", s.lower())


def walk_rs(root: str):
    for dp, _, fs in os.walk(root):
        for f in fs:
            if f.endswith(".rs"):
                yield os.path.join(dp, f)


def collect_streaming() -> dict:
    out = {}
    for p in walk_rs(STREAMING_DIR):
        text = open(p, encoding="utf-8", errors="replace").read()
        for m in re.finditer(r"pub struct Streaming([A-Za-z0-9]+)", text):
            out.setdefault(norm(m.group(1)), (m.group(1), os.path.relpath(p, ROOT)))
    return out


def collect_batch() -> dict:
    out = {}
    for d in BATCH_DIRS:
        if not os.path.isdir(d):
            continue
        for p in walk_rs(d):
            text = open(p, encoding="utf-8", errors="replace").read()
            for m in re.finditer(r"^pub fn ([a-z0-9_]+)", text, re.M):
                out.setdefault(norm(m.group(1)), (m.group(1), os.path.relpath(p, ROOT)))
    return out


def collect_formula() -> dict:
    """公式层已注册的内置函数名。注册有两种形式：
    map.insert("X".to_string(), fn_y);
    map.insert("X".to_string(), fn_y as FormulaFn);
    漏掉 as 转换形式会把已注册函数误判为孤儿（本次审计踩过）。
    """
    out = {}
    if not os.path.exists(FORMULA_FUNCTIONS):
        return out
    text = open(FORMULA_FUNCTIONS, encoding="utf-8", errors="replace").read()
    pat = re.compile(
        r'map\.insert\(\s*"([^"]+)"(?:\.to_string\(\))?\s*,\s*(fn_[a-z0-9_]+)\s*(?:as\s+FormulaFn\s*)?\)'
    )
    for name, fn in pat.findall(text):
        out.setdefault(norm(name), (name, fn))
    return out


# 已知命名别名：registry 名 -> 源码符号。
# 这些是人工核验过的（不是猜的），用于消除命名不匹配假阴性。
ALIASES = {
    "bollingerbands": "bbands",
    "cfo": "chande_forecast_oscillator",
    "twiggsmf": "twiggs_money_flow",
}

# CDL_ 前缀在 patterns/candlestick.rs 里不统一（有 cdl_doji 也有 concealing_baby_swallow），
# 因此对 CDL_* 额外尝试去掉前缀后的名字。
STRIP_PREFIXES = ("cdl",)


def match(name: str, syms: dict, fuzzy: bool = True):
    """返回 (符号名, 匹配方式) 或 (None, None)。"""
    k = norm(name)
    if k in ALIASES:
        ak = norm(ALIASES[k])
        if ak in syms:
            return syms[ak][0], "alias"
    if k in syms:
        return syms[k][0], "exact"
    for pfx in STRIP_PREFIXES:
        if k.startswith(pfx):
            stripped = k[len(pfx):]
            if stripped and stripped in syms:
                return syms[stripped][0], "strip-prefix"
    if not fuzzy:
        return None, None
    # prefix：任一方是另一方的前缀（最短 >= 3，避免 'd'/'ma' 这类噪声）
    for key in [k] + [k[len(p):] for p in STRIP_PREFIXES if k.startswith(p)]:
        if not key:
            continue
        cands = [s for s in syms if len(s) >= 3 and (s.startswith(key) or key.startswith(s))]
        if cands:
            best = min(cands, key=len)
            tag = "prefix" if len(cands) == 1 else f"prefix(+{len(cands) - 1} amb)"
            return syms[best][0], tag
        cands = [s for s in syms if len(s) >= 3 and (s in key or key in s)]
        if cands:
            best = min(cands, key=len)
            return syms[best][0], "substr"
    return None, None


def main() -> int:
    strict = "--strict" in sys.argv
    as_json = "--json" in sys.argv
    fuzzy = not strict

    registry = json.load(open(REGISTRY, encoding="utf-8"))
    inds = registry["indicators"]
    stream = collect_streaming()
    batch = collect_batch()
    formula = collect_formula()

    rows = []
    for i in inds:
        name = i["name"]
        s, stag = match(name, stream, fuzzy)
        b, btag = match(name, batch, fuzzy)
        f, ftag = match(name, formula, fuzzy)
        rows.append(
            {
                "name": name,
                "category": i.get("category", ""),
                "registry_streaming": i.get("streaming"),
                "streaming": s,
                "streaming_match": stag,
                "batch": b,
                "batch_match": btag,
                "formula": f,
                "formula_match": ftag,
            }
        )

    n = len(rows)
    s_hit = sum(1 for r in rows if r["streaming"])
    b_hit = sum(1 for r in rows if r["batch"])
    f_hit = sum(1 for r in rows if r["formula"])
    s_exact = sum(1 for r in rows if r["streaming_match"] == "exact")
    b_exact = sum(1 for r in rows if r["batch_match"] == "exact")

    if as_json:
        print(
            json.dumps(
                {
                    "total": n,
                    "streaming_hit": s_hit,
                    "streaming_exact": s_exact,
                    "batch_hit": b_hit,
                    "batch_exact": b_exact,
                    "formula_hit": f_hit,
                    "rows": rows,
                },
                ensure_ascii=False,
                indent=2,
            )
        )
        return 0

    mode = "STRICT (exact only)" if strict else "FUZZY (exact+prefix+substr)"
    print(f"registry indicators : {n}")
    print(f"mode                : {mode}")
    print()
    print(f"streaming symbols   : {len(stream)}   batch symbols: {len(batch)}"
          f"   formula builtins: {len(formula)}")
    print(
        f"STREAMING coverage  : {s_hit}/{n} = {s_hit / n * 100:.0f}%"
        f"   (exact {s_exact}/{n} = {s_exact / n * 100:.0f}%)"
    )
    print(
        f"BATCH     coverage  : {b_hit}/{n} = {b_hit / n * 100:.0f}%"
        f"   (exact {b_exact}/{n} = {b_exact / n * 100:.0f}%)"
    )
    print(f"FORMULA   coverage  : {f_hit}/{n} = {f_hit / n * 100:.0f}%")

    smiss = [r for r in rows if not r["streaming"]]
    bmiss = [r for r in rows if not r["batch"]]
    bothmiss = [r for r in rows if not r["streaming"] and not r["batch"] and not r["formula"]]

    # 按注册表声明口径：只有 registry.streaming=true 的才需要流式实现。
    # 用「全部 236 项」当分母会把 CDL_* 形态（声明不需要流式）算成缺口，严重高估工作量。
    want_stream = [r for r in rows if r["registry_streaming"] is True]
    ws_hit = sum(1 for r in want_stream if r["streaming"])
    print(
        f"\n=== 按声明口径（分母=registry.streaming=true 的 {len(want_stream)} 项）==="
    )
    print(
        f"STREAMING 应支持 {len(want_stream)} 项，已有 {ws_hit} 项 "
        f"= {ws_hit / len(want_stream) * 100:.0f}%"
    )

    declared = [r for r in rows if r["registry_streaming"] is True and not r["streaming"]]
    print(f"\n*** 违约项：registry 标 streaming=true 但无 Streaming struct: {len(declared)} ***")
    for r in declared:
        where = []
        if r["batch"]:
            where.append(f"batch={r['batch']}")
        if r["formula"]:
            where.append(f"formula={r['formula']}")
        print(f"   {r['name']:26s} cat={r['category']:12s} {' '.join(where) or '(全无)'}")

    fonly = [r for r in rows if r["formula"] and not r["batch"] and not r["streaming"]]
    print(f"\n*** 仅在公式层可用（无 Rust 原生 API）: {len(fonly)} ***")
    for r in fonly[:40]:
        print(f"   {r['name']:26s} cat={r['category']}")

    print(f"\n--- STREAMING missing: {len(smiss)} ---")
    for r in smiss:
        flag = "registry.streaming=true" if r["registry_streaming"] else "registry.streaming=false"
        print(f"  {r['name']:30s} cat={r['category']:14s} {flag}")

    print(f"\n--- BATCH missing: {len(bmiss)} ---")
    for r in bmiss:
        print(f"  {r['name']:30s} cat={r['category']}")

    print(f"\n--- 三处全无（真·完全缺失）: {len(bothmiss)} ---")
    for r in bothmiss:
        print(f"  {r['name']:30s} cat={r['category']}")

    by_cat: dict = defaultdict(lambda: [0, 0, 0, 0])
    for r in rows:
        c = r["category"]
        by_cat[c][0] += 1
        by_cat[c][1] += 1 if r["streaming"] else 0
        by_cat[c][2] += 1 if r["batch"] else 0
        by_cat[c][3] += 1 if r["formula"] else 0
    print("\n--- 按类别 ---")
    print(f"  {'category':18s} {'total':>5s} {'stream':>7s} {'batch':>7s} {'formula':>8s}  stream%")
    for c, (t, s, b, f) in sorted(by_cat.items(), key=lambda kv: kv[1][1] / kv[1][0]):
        print(f"  {c:18s} {t:5d} {s:7d} {b:7d} {f:8d}  {s / t * 100:5.0f}%")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
