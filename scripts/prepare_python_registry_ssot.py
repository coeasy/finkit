#!/usr/bin/env python3
"""Populate Python FFI SSOT before the performance migration.

The canonical docs registry is also validated by the Rust streaming registry,
so migration-only Python body recovery must not mutate it in-place. This helper
builds an ephemeral enriched registry overlay under ``target/`` and hardens the
binding synchronizer to consume that overlay when it exists.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

import sync_bindings as sb

ROOT = Path(__file__).resolve().parents[1]
CANONICAL_REGISTRY = ROOT / "docs" / "indicator_registry.json"
PYTHON_REGISTRY_OVERLAY = ROOT / "target" / "python_registry_ssot.json"


def norm(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def harden_sync_bindings() -> None:
    """Remove hidden FFI assumptions and teach generation about the overlay."""

    path = ROOT / "scripts/sync_bindings.py"
    text = path.read_text(encoding="utf-8")
    old = '    core = ff["core_call"].split("::")[-1]\n'
    new = '    core = ff.get("core_call", pub).split("::")[-1]\n'
    if old in text:
        text = text.replace(old, new, 1)
    elif new not in text:
        raise RuntimeError("sync_bindings candidate_names core_call lookup changed unexpectedly")

    old_android = '        core = ind["ffi"]["core_call"].split("::")[-1]\n'
    new_android = (
        '        ff = ind["ffi"]\n'
        '        c_name = ff["c_name"]\n'
        '        pub = c_name[3:] if c_name.startswith("ta_") else c_name\n'
        '        core = ff.get("core_call", pub).split("::")[-1]\n'
    )
    if old_android in text:
        text = text.replace(old_android, new_android, 1)

    old_loader = '''def load_registry() -> dict:\n    return json.loads(REG.read_text(encoding="utf-8"))\n'''
    new_loader = '''PYTHON_REGISTRY_OVERLAY = ROOT / "target" / "python_registry_ssot.json"\n\n\ndef load_registry() -> dict:\n    registry_path = PYTHON_REGISTRY_OVERLAY if PYTHON_REGISTRY_OVERLAY.exists() else REG\n    return json.loads(registry_path.read_text(encoding="utf-8"))\n'''
    if old_loader in text:
        text = text.replace(old_loader, new_loader, 1)
    elif "PYTHON_REGISTRY_OVERLAY = ROOT / \"target\" / \"python_registry_ssot.json\"" not in text:
        raise RuntimeError("sync_bindings registry loader changed unexpectedly")

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

    wanted = {norm(candidate) for candidate in candidates if candidate}
    matches = [name for name in extracted if norm(name) in wanted]
    if len(matches) == 1:
        return matches[0]
    return None


def main() -> int:
    canonical = CANONICAL_REGISTRY.read_text(encoding="utf-8")
    harden_sync_bindings()

    try:
        subprocess.run(
            [sys.executable, str(ROOT / "scripts/enrich_registry_ffi.py")],
            cwd=ROOT,
            check=True,
        )

        # This process imported sync_bindings before hardening it, intentionally:
        # read the just-enriched canonical file once, then persist only the overlay.
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
                continue
            ff.setdefault("bodies", {})["python"] = extracted[name]["body"]
            c_name = ff["c_name"]
            public = c_name[3:] if c_name.startswith("ta_") else c_name
            expected = sb.NAME_ALIASES.get(public, public)
            if name != expected:
                ff.setdefault("names", {})["python"] = name
            matched += 1

        required = {"ta_bbands", "ta_sar", "ta_stoch"}
        present = {
            ind.get("ffi", {}).get("c_name")
            for ind in inds
            if ind.get("ffi", {}).get("bodies", {}).get("python")
        }
        missing = sorted(required - present)
        if missing:
            raise RuntimeError(f"required Python registry bodies still missing: {missing}")

        PYTHON_REGISTRY_OVERLAY.parent.mkdir(parents=True, exist_ok=True)
        PYTHON_REGISTRY_OVERLAY.write_text(
            json.dumps(reg, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )
    finally:
        # Core registry parity is against this canonical file. Enrichment is a
        # migration input, never a persistent mutation of docs/indicator_registry.json.
        CANONICAL_REGISTRY.write_text(canonical, encoding="utf-8")

    print(
        f"[prepare/python] stored {matched} Python binding bodies in transient overlay; "
        "canonical registry preserved"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
