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
# The `impl` header may spell the trait with a path (`impl crate::streaming::
# IndicatorMeta for T`), which is what three types did -- and an extractor that
# only accepted the bare spelling silently skipped them, so they were never
# validated. Accept any path prefix.
META_IMPL = re.compile(
    r"impl\s+[A-Za-z0-9_:]*IndicatorMeta\s+for\s+(\w+)\s*\{(.*?)\n\}", re.S
)
# Deliberately *broader* than the two extractor regexes above, and used only by
# `count_declared_impls`. A self-check built from the extractor's own patterns is
# a tautology: the form the extractor cannot read is also the form the self-check
# cannot count, so both agree on the wrong total.
CENSUS_IMPL = re.compile(r"impl\s+[A-Za-z0-9_:]*IndicatorMeta\s+for\s+([A-Za-z0-9_]+)")
CENSUS_MACRO = re.compile(r"impl_indicator_meta!\(")
FN_NAME_HEAD = r"fn name\(\) -> &'static str \{"
FN_CATEGORY_HEAD = r"fn category\(\) -> &'static str \{"

BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)

# Registry entries whose published `name` is a *display* name that cannot be
# reduced to the canonical `IndicatorMeta::name()` by case/punctuation folding
# alone ("Bollinger Bands" -> "BBANDS", "Stochastic" -> "STOCH"). Everything
# else is resolved by `norm()` below.
#
# Every pair here must be declared, not inferred: the point of the check is to
# make an unbacked claim loud, and a fuzzy matcher would instead invent a
# mapping and hide the next one. Keep this list minimal and delete an entry the
# moment the registry adopts the canonical name -- a stale alias fails the gate.
STREAMING_NAME_ALIASES = {
    "Bollinger Bands": "BBANDS",
    "Chaikin Volatility": "ChaikinVol",
    "Donchian Channel": "Donchian",
    "Ichimoku Cloud": "Ichimoku",
    "Keltner Channel": "Keltner",
    "Stochastic": "STOCH",
    "Williams %R": "WILLR",
}


def norm(name: str) -> str:
    """Fold case and punctuation: 'Williams %R' -> 'williamsr'."""
    return re.sub(r"[^a-z0-9]", "", name.lower())


def first_literal_after(blob: str, head: str) -> str | None:
    """Return the first string literal in the body of the accessor `head`.

    Comments are stripped first: a `//` note inside the body must not make the
    accessor invisible to this gate (that is exactly how a real drift hides --
    an extractor that only accepts `{ "literal"` silently skips the type).
    """
    match = re.search(head, blob)
    if not match:
        return None
    tail = BLOCK_COMMENT.sub("", blob[match.end() :])
    tail = "\n".join(line.split("//", 1)[0] for line in tail.splitlines())
    literal = re.search(r'"([^"]*)"', tail)
    return literal.group(1) if literal else None


def read_registry() -> str:
    return REGISTRY.read_text(encoding="utf-8")


def parse_valid_categories(text: str) -> list[str]:
    block = re.search(r"VALID_CATEGORIES: &\[&str\] = &\[(.*?)\];", text, re.S)
    if not block:
        raise SystemExit("[streaming-registry] VALID_CATEGORIES not found in registry.rs")
    return re.findall(r'"([^"]*)"', block.group(1))


def parse_registry_entries(text: str) -> dict[str, dict]:
    body = text[text.index("static INDICATORS") :]
    entries: dict[str, dict] = {}
    for chunk in body.split("IndicatorInfo {")[1:]:
        chunk = chunk.split("IndicatorInfo {")[0]
        name = re.search(r'name:\s*"([^"]*)"', chunk)
        category = re.search(r'category:\s*"([^"]*)"', chunk)
        streaming = re.search(r"streaming:\s*(true|false)", chunk)
        if name and category:
            entries[name.group(1)] = {
                "category": category.group(1),
                "streaming": streaming is not None and streaming.group(1) == "true",
            }
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
            name = first_literal_after(m.group(2), FN_NAME_HEAD)
            category = first_literal_after(m.group(2), FN_CATEGORY_HEAD)
            if name and category:
                metas.append(
                    {
                        "type": m.group(1),
                        "name": name,
                        "category": category,
                        "file": str(pathlib.Path(path).relative_to(ROOT)),
                    }
                )
    return metas


def count_declared_impls() -> int:
    """Number of `IndicatorMeta` implementations in the crate, counted broadly.

    Used as a self-check: the extractor must see every implementation, otherwise
    a type silently escapes validation -- which is how the drift this gate exists
    for would hide. The census is deliberately written with *different, looser*
    patterns than the extractor, so a form the extractor cannot read still shows
    up in this count and trips the check.
    """
    total = 0
    for path in glob.glob(str(ROOT / "core/src/streaming/**/*.rs"), recursive=True):
        if pathlib.Path(path).name == "macros.rs":
            continue
        text = pathlib.Path(path).read_text(encoding="utf-8")
        total += len(CENSUS_IMPL.findall(text))
        total += len(CENSUS_MACRO.findall(text))
    return total


def main() -> int:
    text = read_registry()
    valid = parse_valid_categories(text)
    entries = parse_registry_entries(text)
    metas = parse_metas()
    declared = count_declared_impls()

    undeclared = [m for m in metas if m["category"] not in valid]
    disagreements = [
        m
        for m in metas
        if m["name"] in entries and entries[m["name"]]["category"] != m["category"]
    ]
    unreadable = declared - len(metas)

    # Every `streaming: true` entry is a published claim that an incremental
    # implementation exists. Nothing checked that: `test_registry_coverage` only
    # asserts a count floor and that three categories are false, so an entry
    # could advertise streaming with nothing behind it (MONEY_FLOW did).
    meta_names = {m["name"] for m in metas}
    norm_metas = {norm(m["name"]): m["name"] for m in metas}

    def resolve(registry_name: str) -> str | None:
        """Canonical meta name for a registry entry, or None if unbacked."""
        alias = STREAMING_NAME_ALIASES.get(registry_name)
        if alias is not None:
            return alias if alias in meta_names else None
        return norm_metas.get(norm(registry_name))

    unbacked = [
        (name, STREAMING_NAME_ALIASES.get(name, norm(name)))
        for name, info in sorted(entries.items())
        if info["streaming"] and resolve(name) is None
    ]
    # An alias that is no longer needed is dead config and must not be kept:
    # otherwise the table silently accumulates and hides a real gap.
    stale_aliases = sorted(
        alias for alias in STREAMING_NAME_ALIASES if resolve(alias) is not None
        and norm(alias) in norm_metas
    )

    print("[streaming-registry] IndicatorMeta vs registry contract")
    print(f"  declared categories : {len(valid)}")
    print(f"  registry entries    : {len(entries)}")
    print(f"  IndicatorMeta impls : {len(metas)} (declared {declared})")
    print(f"  streaming claims    : {sum(1 for i in entries.values() if i['streaming'])}")

    if unreadable:
        print(
            f"\n[streaming-registry] {unreadable} `impl IndicatorMeta` block(s) could not be "
            "read by the extractor, so they are NOT validated.\n"
            "  Fix the extractor rather than the count: an unreadable block is an "
            "unvalidated type."
        )
        print("\n[streaming-registry] FAIL")
        return 1

    if undeclared:
        print("\n[streaming-registry] UNDECLARED category slugs:")
        for m in sorted(undeclared, key=lambda x: (x["category"], x["name"])):
            print(f"  - {m['name']} ({m['type']}) -> {m['category']!r}  [{m['file']}]")

    if disagreements:
        print("\n[streaming-registry] CATEGORY DISAGREES with the registry entry:")
        for m in sorted(disagreements, key=lambda x: x["name"]):
            print(
                f"  - {m['name']} ({m['type']}): meta={m['category']!r} "
                f"registry={entries[m['name']]['category']!r}  [{m['file']}]"
            )

    if unbacked:
        print(
            "\n[streaming-registry] ADVERTISED AS streaming BUT NO IMPLEMENTATION:\n"
            "  (the registry says `streaming: true`; no IndicatorMeta publishes this\n"
            "   name, and it is not in STREAMING_NAME_ALIASES)"
        )
        for name, canonical in unbacked:
            print(f"  - {name!r} (looked for {canonical!r})")

    if stale_aliases:
        print(
            "\n[streaming-registry] STALE STREAMING_NAME_ALIASES entries:\n"
            "  (the registry name already matches an IndicatorMeta name, so the\n"
            "   alias is dead -- delete it)"
        )
        for alias in stale_aliases:
            print(f"  - {alias!r}")

    if undeclared or disagreements or unbacked or stale_aliases:
        print("\n[streaming-registry] FAIL")
        return 1

    print("\n[streaming-registry] OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
