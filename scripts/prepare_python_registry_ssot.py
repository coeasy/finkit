#!/usr/bin/env python3
"""Populate Python FFI SSOT before the performance migration.

The current v0.1.4 registry intentionally contains indicator metadata without
its historical one-time ``ffi`` enrichment, while the Python indicator bodies
already live in ``ffi/python-binding/src/generated.rs``.  The binding
synchronizer's discovery mode only scans ``lib.rs`` and therefore cannot
recover those relocated bodies by itself.

This helper performs the intended one-time enrichment and then discovers Python
bodies from both ``lib.rs`` and ``generated.rs``.  It is idempotent and uses the
same matching rules as ``sync_bindings.py``.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import sync_bindings as sb

ROOT = Path(__file__).resolve().parents[1]


def main() -> int:
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
    missing: list[str] = []
    for ind in inds:
        ff = ind.setdefault("ffi", {})
        name = sb.match_indicator(ind, "python", extracted)
        if name is None:
            # Not every C ABI indicator has a standalone Python function.  Such
            # entries intentionally remain without a Python body.
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
