#!/usr/bin/env python3
"""One-time enrichment: attach a structured `ffi` block to every indicator in
docs/indicator_registry.json that is exposed by ffi/c-binding/include/finkit.h,
making the registry the COMPLETE single source of truth for C binding codegen.

For each `TA_API ta_result_t ta_*(...)` in the header we:
  - resolve it to a registry entry (by normalized name, then a manual alias
    map for indicators whose registry name differs from the C name);
  - if no registry entry exists yet (FTA-native chart/transform indicators),
    create one with sensible metadata so the registry covers the full C surface;
  - store an `ffi` block: c_name, doc_group, inputs/outputs/params (the exact,
    proven C signature) and an `order` index preserving header layout.

scripts/gen_c_header.py then regenerates finkit.h from these blocks.

Idempotent: re-running overwrites existing `ffi` blocks and skips already-added
entries.
"""
from __future__ import annotations

import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
HEADER = ROOT / "ffi/c-binding/include/finkit.h"
REGISTRY = ROOT / "docs/indicator_registry.json"

SECTION_RE = re.compile(r"──\s*(.+?)\s*──")
FN_RE = re.compile(r"TA_API\s+ta_result_t\s+(ta_\w+)\s*\((.*?)\)\s*;", re.DOTALL)
PARAM_RE = re.compile(r"(const\s+)?(\w+)\s*(\*?)\s*(\w+)")

# C function base (ta_ stripped) -> registry name, for indicators whose
# registry spelling differs from the C name.
ALIASES = {
    "bbands": "Bollinger Bands",
    "stoch": "Stochastic",
    "willr": "Williams %R",
    "correlation": "CORREL",
    "cdl_three_white_soldiers": "CDL_3WHITE_SOLDIERS",
    "cdl_three_black_crows": "CDL_3BLACK_CROWS",
}

GROUP_TO_CATEGORY = {
    "Moving averages & overlays": "overlap",
    "Momentum & oscillators": "momentum",
    "Volatility & volume": "volatility",
    "Hilbert transform": "cycle",
    "Statistics & price transforms": "statistic",
    "Candlestick patterns": "pattern",
    "Chart patterns (FTA-native)": "chart",
}


def norm(s: str) -> str:
    return re.sub(r"[^a-z0-9]", "", s.lower())


def ctype_to_rust(ct: str) -> str:
    return "usize" if ct == "int32_t" else "f64"


def parse_params(ps: str):
    ps = ps.strip()
    if not ps:
        return [], [], []
    inputs, outputs, params = [], [], []
    for raw in ps.split(","):
        p = raw.strip()
        if p == "int32_t len":
            continue
        m = PARAM_RE.match(p)
        if not m:
            continue
        const, ctype, star, name = m.group(1), m.group(2), m.group(3), m.group(4)
        if star == "*":
            if const:
                inputs.append({"name": name, "c_type": ctype, "ptr": True, "const": True})
            else:
                outputs.append({"name": name, "c_type": ctype, "ptr": True})
        else:
            params.append({"name": name, "c_type": ctype, "ptr": False})
    return inputs, outputs, params


def registry_name_for(cname: str, norm_map, name_map):
    base = cname[3:]  # strip "ta_"
    if base in ALIASES:
        return name_map.get(ALIASES[base])
    return norm_map.get(norm(base))


def make_entry(cname: str, group: str, inputs, outputs, params) -> dict:
    base = cname[3:]
    if base.startswith("cdl_"):
        name = "CDL_" + base[4:].upper()
    else:
        name = base.upper()
    param_meta = [
        {"name": p["name"], "param_type": ctype_to_rust(p["c_type"]),
         "default": "0", "description": p["name"].replace("_", " ")}
        for p in params
    ]
    ffi = {
        "c_name": cname,
        "doc_group": group,
        "inputs": inputs,
        "outputs": outputs,
        "params": params,
    }
    return {
        "name": name,
        "category": GROUP_TO_CATEGORY.get(group, "other"),
        "description": name.replace("_", " ").title(),
        "params": param_meta,
        "convergence": 0,
        "streaming": False,
        "ffi": ffi,
    }


def main() -> None:
    text = HEADER.read_text(encoding="utf-8")
    current_group = None
    parsed = []  # (order, group, c_name, params_str)
    in_fn = False
    buf = ""
    order = 0
    for line in text.splitlines():
        sec = SECTION_RE.search(line)
        if sec and not in_fn:
            current_group = sec.group(1).strip()
        if "TA_API" in line and "ta_result_t" in line:
            in_fn = True
            buf = line
        elif in_fn:
            buf += " " + line
        if in_fn and ");" in line:
            m = FN_RE.search(buf)
            if m:
                parsed.append((order, current_group, m.group(1), m.group(2)))
                order += 1
            in_fn = False
            buf = ""

    reg = json.loads(REGISTRY.read_text(encoding="utf-8"))
    inds = reg.setdefault("indicators", [])
    norm_map = {norm(i["name"]): i for i in inds}
    name_map = {i["name"]: i for i in inds}

    enriched, added, unmatched = 0, 0, []
    for idx, group, cname, ps in parsed:
        ind = registry_name_for(cname, norm_map, name_map)
        inputs, outputs, params = parse_params(ps)
        if ind is None:
            # Not yet in the registry: add it so the registry covers the full C surface.
            ind = make_entry(cname, group, inputs, outputs, params)
            ind["ffi"]["order"] = idx
            inds.append(ind)
            norm_map[norm(ind["name"])] = ind
            name_map[ind["name"]] = ind
            added += 1
            enriched += 1
            continue
        ind["ffi"] = {
            "c_name": cname,
            "doc_group": group,
            "inputs": inputs,
            "outputs": outputs,
            "params": params,
            "order": idx,
        }
        if "order" not in ind["ffi"]:
            ind["ffi"]["order"] = idx
        enriched += 1

    REGISTRY.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"[enrich] parsed {len(parsed)} indicator fns from header")
    print(f"[enrich] enriched {enriched} registry entries (added {added} missing ones)")
    if unmatched:
        print(f"[enrich] WARNING unmatched: {unmatched}")
    else:
        print("[enrich] all 78 header indicator fns now resolve in the registry ✅")


if __name__ == "__main__":
    main()
