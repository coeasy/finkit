#!/usr/bin/env python3
"""Populate Python FFI SSOT before the performance migration.

The current v0.1.4 registry contains indicator metadata without its historical
one-time ``ffi`` enrichment, while Python indicator bodies already live in
``ffi/python-binding/src/generated.rs``.  This helper performs the enrichment,
recovers Python bodies from both lib.rs and generated.rs, and hardens the
synchronizer so ``ffi.core_call`` is optional rather than an undocumented
required field.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import sync_bindings as sb

ROOT = Path(__file__).resolve().parents[1]


def norm(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def harden_sync_bindings() -> None:
    """Remove the hidden dependency on ffi.core_call.

    enrich_registry_ffi.py intentionally emits the public ABI contract and does
    not know the Rust core implementation path.  Name resolution can always
    fall back to the C/public basename, so core_call must remain optional.
    """

    path = ROOT / "scripts/sync_bindings.py"
    text = path.read_text(encoding="utf-8")
    old = '    core = ff["core_call"].split("::")[-1]\n'
    new = '    core = ff.get("core_call", pub).split("::")[-1]\n'
    if old in text:
        text = text.replace(old, new, 1)
    elif new not in text:
        raise RuntimeError("sync_bindings candidate_names core_call lookup changed unexpectedly")

    # Android has a second optional core_call lookup in its fallback matcher.
    old_android = '        core = ind["ffi"]["core_call"].split("::")[-1]\n'
    new_android = (
        '        ff = ind["ffi"]\n'
        '        c_name = ff["c_name"]\n'
        '        pub = c_name[3:] if c_name.startswith("ta_") else c_name\n'
        '        core = ff.get("core_call", pub).split("::")[-1]\n'
    )
    if old_android in text:
        text = text.replace(old_android, new_android, 1)
    path.write_text(text, encoding="utf-8")


def python_match(ind: dict, extracted: dict[str, dict]) -> str | None:
    ff = ind["ffi"]
    c_name = ff["c_name"]
    public = c_name[3:] if c_name.startswith("ta_") else c_name
    candidates = []
    explicit = ff.get("names", {}).get("python")
    if explicit:
        candidates.append(explicit)
    candidates.extend(
        [
            sb.NAME_ALIASES.get(public, public),
            public,
            public.replace("_", ""),
        ]
    )
    for candidate in candidates:
        if candidate in extracted:
            return candidate

    # Generated Python names historically use a mix of cdl_xxx and cdlxxx.
    # Normalized matching handles that relocation without guessing core paths.
    wanted = {norm(candidate) for candidate in candidates if candidate}
    matches = [name for name in extracted if norm(name) in wanted]
    if len(matches) == 1:
        return matches[0]
    return None


def main() -> int:
    harden_sync_bindings()
    subprocess.run(
        [sys.executable, str(ROOT / "scripts/enrich_registry_ffi.py")],
        cwd=ROOT,
        check=True,
    )

    reg = sb.load_registry()
    inds = sb.indicators_with_ffi(reg)
    cfg = sb.LANG_CFG["python"]
    extracted = sb.extract_functions(
        (ROOT / cfg["lib"]).read_text(encoding="utf-8"), "python"
    )
    gen_path = ROOT / cfg["gen"]
    if gen_path.exists():
        extracted.update(sb.extract_functions(gen_path.read_text(encoding="utf-8"), "python"))

    matched = 0
    for ind in inds:
        ff = ind.setdefault("ffi", {})
        name = python_match(ind, extracted)
        if name is None:
            # Not every C ABI indicator has a standalone Python function.
            continue
        ff.setdefault("bodies", {})["python"] = extracted[name]["body"]
        c_name = ff["c_name"]
        public = c_name[3:] if c_name.startswith("ta_") else c_name
        expected = sb.NAME_ALIASES.get(public, public)
        if name != expected:
            ff.setdefault("names", {})["python"] = name
        matched += 1

    sb.save_registry(reg)

    required = {"ta_bbands", "ta_sar", "ta_stoch"}
    present = {
        ind.get("ffi", {}).get("c_name")
        for ind in inds
        if ind.get("ffi", {}).get("bodies", {}).get("python")
    }
    missing = sorted(required - present)
    if missing:
        raise RuntimeError(f"required Python registry bodies still missing: {missing}")

    print(f"[prepare/python] stored {matched} Python binding bodies; required contracts present")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
