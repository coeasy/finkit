#!/usr/bin/env python3
"""Fail when streaming `IndicatorMeta` metadata disagrees with the registry.

Motivation
----------
The streaming layer publishes indicator metadata through **two** independent
sources:

* ``core/src/streaming/registry.rs`` -- the hand-written ``INDICATORS`` table
  that backs ``all_indicators()`` / ``registry_document()`` and the published
  ``docs/indicator_registry.json``. This is the discovery contract that the CLI,
  the language bindings and the docs all read.
* every ``impl IndicatorMeta`` -- each streaming type answers ``name()`` /
  ``category()`` / ``description()`` for itself.

Nothing tied the two together, so they drifted. The drift was invisible because
``registry.rs``'s own ``test_valid_categories`` only validates *registry*
categories, and the per-type tests merely restate the literal from their own
``impl`` block. Concretely, before this gate existed:

* eight types returned the slug ``"statistic"``, which is not a member of
  ``VALID_CATEGORIES`` (the declared vocabulary) -- a consumer grouping by
  ``IndicatorMeta::category()`` would invent a category the contract does not
  define, and five of them contradicted their own registry entry, which says
  ``"statistics"``;
* ``StreamingSuperTrend`` reported ``"volatility"`` while the registry entry it
  is published under says ``"overlap"``.

Exit codes
----------
0  every meta category is a declared slug and agrees with the registry.
1  a meta category is undeclared, or disagrees with the registry entry.

Notes
-----
``macros.rs`` is skipped: the ``impl_indicator_meta!`` occurrences there live in
doc comments as examples, not in real implementations.
"""

from __future__ import annotations

import glob
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "core" / "src" / "streaming" / "registry.rs"

META_MACRO = re.compile(
    r'impl_indicator_meta!\(\s*(\w+)\s*,\s*"([^"]*)"\s*,\s*"([^"]*)"\s*,\s*"([^"]*)"'
)
META_IMPL = re.compile(r"impl IndicatorMeta for (\w+) \{(.*?)\n\}", re.S)
FN_CATEGORY = r'fn category\(\) -> &\'static str \{\s*"([^"]*)"'


def read_registry() -> str:
    return REGISTRY.read_text(encoding="utf-8")


def parse_valid_categories(text: str) -> list[str]:
    block = re.search(r"VALID_CATEGORIES: &\[&str\] = &\[(.*?)\];", text, re.S)
    if not block:
        raise SystemExit("[streaming-registry] VALID_CATEGORIES not found in registry.rs")
    return re.findall(r'"([^"]*)"', block.group(1))


def parse_registry_entries(text: str) -> dict[str, str]:
    body = text[text.index("static INDICATORS") :]
    entries: dict[str, str] = {}
    for chunk in body.split("IndicatorInfo {")[1:]:
        chunk = chunk.split("IndicatorInfo {")[0]
        name = re.search(r'name:\s*"([^"]*)"', chunk)
        category = re.search(r'category:\s*"([^"]*)"', chunk)
        if name and category:
            entries[name.group(1)] = category.group(1)
    return entries


def parse_metas() -> list[dict]:
    metas: list[dict] = []
    for path in glob.glob(str(ROOT / "core/src/streaming/**/*.rs"), recursive=True):
        if pathlib.Path(path).name == "macros.rs":
            continue
        text = pathlib.Path(path).read_text(encoding="utf-8")
        for m in META_MACRO.finditer(text):
            metas.append(
                {
                    "type": m.group(1),
                    "name": m.group(2),
                    "category": m.group(3),
                    "file": str(pathlib.Path(path).relative_to(ROOT)),
                }
            )
        for m in META_IMPL.finditer(text):
            name = re.search(r'fn name\(\) -> &\'static str \{\s*"([^"]*)"', m.group(2))
            category = re.search(FN_CATEGORY, m.group(2))
            if name and category:
                metas.append(
                    {
                        "type": m.group(1),
                        "name": name.group(1),
                        "category": category.group(1),
                        "file": str(pathlib.Path(path).relative_to(ROOT)),
                    }
                )
    return metas


def main() -> int:
    text = read_registry()
    valid = parse_valid_categories(text)
    entries = parse_registry_entries(text)
    metas = parse_metas()

    undeclared = [m for m in metas if m["category"] not in valid]
    disagreements = [
        m for m in metas if m["name"] in entries and entries[m["name"]] != m["category"]
    ]

    print("[streaming-registry] IndicatorMeta vs registry contract")
    print(f"  declared categories : {len(valid)}")
    print(f"  registry entries    : {len(entries)}")
    print(f"  IndicatorMeta impls : {len(metas)}")

    if undeclared:
        print("\n[streaming-registry] UNDECLARED category slugs:")
        for m in sorted(undeclared, key=lambda x: (x["category"], x["name"])):
            print(f"  - {m['name']} ({m['type']}) -> {m['category']!r}  [{m['file']}]")

    if disagreements:
        print("\n[streaming-registry] CATEGORY DISAGREES with the registry entry:")
        for m in sorted(disagreements, key=lambda x: x["name"]):
            print(
                f"  - {m['name']} ({m['type']}): meta={m['category']!r} "
                f"registry={entries[m['name']]!r}  [{m['file']}]"
            )

    if undeclared or disagreements:
        print("\n[streaming-registry] FAIL")
        return 1

    print("\n[streaming-registry] OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
