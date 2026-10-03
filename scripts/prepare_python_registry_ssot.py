#!/usr/bin/env python3
"""Populate the transient Python FFI SSOT overlay before binding generation.

The canonical docs registry is also validated by the Rust streaming registry,
so migration-only Python body recovery must not mutate it in place. This helper
builds an ephemeral enriched registry overlay under ``target/`` and requires the
binding synchronizer to consume that overlay when it exists.

The synchronizer hooks used to be *installed* by this script, which meant a build
step edited another build step. V4 plan Batch 2 moved those hooks into
``scripts/binding_spec.py`` as invariants, so this script verifies them and never
rewrites `sync_bindings.py`.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import binding_spec  # noqa: E402  (path setup must precede the import)
import sync_bindings as sb  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
CANONICAL_REGISTRY = ROOT / "docs" / "indicator_registry.json"
FFI_REGISTRY = ROOT / "docs" / "ffi_registry.json"
PYTHON_REGISTRY_OVERLAY = ROOT / "target" / "python_registry_ssot.json"


def norm(value: str) -> str:
    return re.sub(r"[^a-z0-9]", "", value.lower())


def verify_sync_bindings_hooks() -> None:
    """Require the generator's overlay/merge contract without editing it.

    The hooks matter in both directions: without the overlay lookup the generator
    reads the checked-in registry and loses the recovered Python bodies; without
    the core+FFI merge `--discover` can overwrite the rich core registry with a
    name-only stub.
    """

    generator = binding_spec.SYNC_BINDINGS.read_text(encoding="utf-8")
    missing = [
        needle
        for needle in binding_spec.SYNC_BINDINGS_RULESET.required
        if needle not in generator
    ]
    if missing:
        raise RuntimeError(
            "sync_bindings.py lost part of the Python generation contract: "
            + ", ".join(repr(needle) for needle in missing)
        )


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
    canonical = CANONICAL_REGISTRY.read_bytes()
    verify_sync_bindings_hooks()

    try:
        subprocess.run(
            [sys.executable, str(ROOT / "scripts/enrich_registry_ffi.py")],
            cwd=ROOT,
            check=True,
        )

        # FFI enrichment updates docs/ffi_registry.json, while the core
        # indicator registry intentionally remains FFI-free.  Read the
        # enriched FFI registry directly here and persist it only as the
        # transient Python overlay consumed by sync_bindings.
        reg = json.loads(FFI_REGISTRY.read_text(encoding="utf-8"))
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
            # After the NumPy-direct migration, generated.py may contain a
            # thin public wrapper plus its preserved Vec-returning
            # `vec_<name>_impl` body.  The impl is the canonical source body;
            # persisting the thin wrapper into the transient overlay makes
            # the next prepare pass lose the implementation and breaks the
            # wheel build.  Normalize it back to the original public name.
            impl_name = f"vec_{name}_impl"
            if impl_name in extracted:
                ff["bodies"]["python"] = extracted[impl_name]["body"].replace(
                    f"fn {impl_name}", f"fn {name}", 1
                )
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
        CANONICAL_REGISTRY.write_bytes(canonical)

    print(
        f"[prepare/python] stored {matched} Python binding bodies in transient overlay; "
        "canonical registry preserved"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
