#!/usr/bin/env python3
"""Registry-driven binding synchronizer for Finkit.

This is the multi-language extension of ``gen_binding.py``.  Where
``gen_binding.py`` *templates* C/Python/Node wrappers from language-agnostic
metadata, this script takes the safe, byte-faithful approach used to rebuild
the C binding:

    For every FFI binding it extracts the **indicator** function bodies
    verbatim from ``ffi/<lang>-binding/src/lib.rs`` and stores them in the
    single-source-of-truth ``docs/indicator_registry.json`` under
    ``ffi.bodies.<lang>`` (and ``ffi.names.<lang>`` when the public name
    differs from the core name).  It can then regenerate
    ``ffi/<lang>-binding/src/generated.rs`` from the registry and rewrite
    ``lib.rs`` to ``include!`` it, dropping the hand-written indicator
    functions.  Because the bodies are replayed verbatim, the regenerated
    binding is behaviour-identical to the current one — the move is purely
    mechanical relocation into a generated file.

Modes
-----
    --discover [--lang c|python|node|go|java|dotnet|ios|android]...
        Scan each binding's lib.rs, match indicator functions to registry
        entries, and write ``ffi.bodies.<lang>`` / ``ffi.names.<lang>`` back
        into the registry.  Prints a match report (lists unmatched).
    --generate [--lang ...] [--rewrite]
        Emit generated.rs for each language from the registry.  With
        ``--rewrite`` also drop the hand-written indicator spans from lib.rs
        and insert ``include!("generated.rs");``.
    --check [--lang ...]
        Re-extract from the current lib.rs and compare against the stored
        bodies to detect drift (hand edits that were not pushed to the
        registry).  Exits non-zero on drift.

The registry is the SSOT: adding an indicator becomes "add its body to
``ffi.bodies.<lang>`` for every language (or run --discover on the canonical
binding)" + regenerate.  CI should run ``--check`` for every language to keep
the bindings in sync.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
REG = ROOT / "docs" / "indicator_registry.json"

# Per-language configuration.  ``sig`` matches the function *signature* line;
# the extractor then walks backward over doc/attribute lines and forward over
# the brace-balanced body.  ``kind`` selects how the body is stored/emitted.
LANG_CFG = {
    "c": {
        "lib": "ffi/c-binding/src/lib.rs",
        "gen": "ffi/c-binding/src/generated.rs",
        "sig": r'pub\s+unsafe\s+extern\s+"C"\s+fn\s+(ta_[A-Za-z0-9_]+)',
        "kind": "extern_c",
    },
    "python": {
        "lib": "ffi/python-binding/src/lib.rs",
        "gen": "ffi/python-binding/src/generated.rs",
        "sig": r'#\[pyfunction\][^\n]*\n(?:#\[[^\n]*\]\n)*fn\s+([a-z][a-z0-9_]+)',
        "sig_name_group": 1,
        "kind": "pyfunction",
    },
    "node": {
        "lib": "ffi/node-binding/src/lib.rs",
        "gen": "ffi/node-binding/src/generated.rs",
        "sig": r'#\[napi\][^\n]*\n(?:#\[[^\n]*\]\n)*pub\s+(?:async\s+)?fn\s+([a-z][a-z0-9_]+)',
        "sig_name_group": 1,
        "kind": "napi",
    },
    "go": {
        "lib": "ffi/go-binding/src/lib.rs",
        "gen": "ffi/go-binding/src/generated.rs",
        "sig": r'pub\s+extern\s+"C"\s+fn\s+(ta_[A-Za-z0-9_]+)',
        "kind": "extern_c",
    },
    "dotnet": {
        "lib": "ffi/dotnet-binding/src/lib.rs",
        "gen": "ffi/dotnet-binding/src/generated.rs",
        "sig": r'pub\s+extern\s+"C"\s+fn\s+(ta_[A-Za-z0-9_]+)',
        "kind": "extern_c",
    },
    "ios": {
        "lib": "ffi/ios-binding/src/lib.rs",
        "gen": "ffi/ios-binding/src/generated.rs",
        "sig": r'pub\s+extern\s+"C"\s+fn\s+(alpha_ta_[A-Za-z0-9_]+)',
        "kind": "extern_c",
    },
    "java": {
        "lib": "ffi/java-binding/src/lib.rs",
        "gen": "ffi/java-binding/src/generated.rs",
        "sig": r'pub\s+extern\s+"system"\s+fn\s+(Java_[A-Za-z0-9_]+)',
        "kind": "jni",
    },
    "android": {
        "lib": "ffi/android-binding/src/lib.rs",
        "gen": "ffi/android-binding/src/generated.rs",
        "sig": r'shim_indicator!\s*\(',
        "kind": "android_shim",
    },
}

NAME_ALIASES = {
    "bbands": "bollinger_bands",
    "inertia": "inertia_indicator",
}

KNOWN_INFRA = {
    "ta_version", "ta_last_error", "ta_last_error_code", "ta_ffi_panic_test",
    "ta_free_result", "ta_free_string", "ta_free", "ta_free_array",
    "ta_free_cstring", "freeJString",
}


def load_registry() -> dict:
    return json.loads(REG.read_text(encoding="utf-8"))


def save_registry(reg: dict) -> None:
    REG.write_text(json.dumps(reg, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def indicators_with_ffi(reg: dict) -> list[dict]:
    out = [i for i in reg.get("indicators", []) if i.get("ffi", {}).get("c_name")]
    if not out:
        raise SystemExit(
            "docs/indicator_registry.json has no ffi metadata; refusing to generate or "
            "validate empty binding files. Run scripts/enrich_registry_ffi.py only as an "
            "intentional registry migration, then review the resulting diff before using "
            "sync_bindings.py."
        )
    out.sort(key=lambda i: i["ffi"].get("order", 0))
    return out


def _brace_span(src: str, open_idx: int) -> int:
    depth = 0
    i = open_idx
    while i < len(src):
        c = src[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    raise RuntimeError("unbalanced braces")


def extract_functions(src: str, lang: str) -> dict[str, dict]:
    cfg = LANG_CFG[lang]
    sig = cfg["sig"]
    name_group = cfg.get("sig_name_group", 1)
    out: dict[str, dict] = {}
    for m in re.finditer(sig, src):
        if lang == "android":
            j = m.end() - 1
            depth = 0
            while j < len(src):
                if src[j] == "(":
                    depth += 1
                elif src[j] == ")":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            end = j + 1
            if end < len(src) and src[end] == ";":
                end += 1
            body = src[m.start():end]
            first_arg = body.split("(", 1)[1].split(",", 1)[0].strip()
            name = first_arg
            out[name] = {"body": body, "start": m.start(), "end": end}
            continue

        name = m.group(name_group)
        line_start = src.rfind("\n", 0, m.start()) + 1
        cur = line_start
        while cur > 0:
            prev_line_end = src.rfind("\n", 0, cur - 1) + 1
            seg = src[prev_line_end:cur].rstrip("\n")
            if seg.startswith("///") or seg.lstrip().startswith("#["):
                cur = prev_line_end
            else:
                break
        start = cur
        depth = 0
        i = m.end() - 1
        while i < len(src):
            if src[i] == "(":
                depth += 1
            elif src[i] == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        k = src.find("{", i)
        end = _brace_span(src, k)
        body = src[start:end]
        out[name] = {"body": body, "start": start, "end": end}
    return out


def candidate_names(ind: dict, lang: str) -> list[str]:
    ff = ind["ffi"]
    c_name = ff["c_name"]
    pub = c_name[3:] if c_name.startswith("ta_") else c_name
    # `enrich_registry_ffi.py` can reconstruct FFI metadata from the C header
    # without knowing a Rust core-call path.  Treat `core_call` as an optional
    # legacy hint, never as a required registry field.  The public C-derived
    # name is the stable fallback shared by the binding surfaces.
    core = ff.get("core_call", pub).split("::")[-1]
    if lang in ("c", "go", "dotnet"):
        return [c_name, c_name + "_json"]
    if lang == "ios":
        return ["alpha_" + c_name]
    if lang in ("python", "node"):
        names: list[str] = []
        if ff.get("names", {}).get(lang):
            names.append(ff["names"][lang])
        if pub in NAME_ALIASES:
            names.append(NAME_ALIASES[pub])
        names.append(pub)
        if core not in names:
            names.append(core)
        return names
    if lang == "java":
        names = []
        if pub in NAME_ALIASES:
            names.append("Java_com_finkit_Indicators_" + NAME_ALIASES[pub])
        names.append("Java_com_finkit_Indicators_" + pub)
        core_name = "Java_com_finkit_Indicators_" + core
        if core_name not in names:
            names.append(core_name)
        return names
    if lang == "android":
        return [c_name]
    return [pub]


def match_indicator(ind: dict, lang: str, extracted: dict[str, dict]) -> str | None:
    for cand in candidate_names(ind, lang):
        if cand in extracted:
            return cand
    if lang == "android":
        ff = ind["ffi"]
        c_name = ff["c_name"]
        pub = c_name[3:] if c_name.startswith("ta_") else c_name
        core = ff.get("core_call", pub).split("::")[-1]
        alias = NAME_ALIASES.get(core, core)
        for nm, info in extracted.items():
            if (f", {core}," in info["body"]) or (f", {alias}," in info["body"]):
                return nm
    return None


def do_discover(langs: list[str]) -> int:
    reg = load_registry()
    inds = indicators_with_ffi(reg)
    total_unmatched = 0
    for lang in langs:
        cfg = LANG_CFG[lang]
        src = (ROOT / cfg["lib"]).read_text(encoding="utf-8")
        extracted = extract_functions(src, lang)
        matched = 0
        unmatched = []
        for ind in inds:
            ff = ind.setdefault("ffi", {})
            name = match_indicator(ind, lang, extracted)
            if name is None:
                unmatched.append(ind["ffi"]["c_name"])
                continue
            matched += 1
            body = extracted[name]["body"]
            ff.setdefault("bodies", {})[lang] = body
            public = name
            pub = ff["c_name"][3:] if ff["c_name"].startswith("ta_") else ff["c_name"]
            if lang in ("python", "node"):
                if public != pub and public != NAME_ALIASES.get(pub, pub):
                    ff.setdefault("names", {})[lang] = public
            elif lang == "ios":
                if public != "alpha_" + ff["c_name"]:
                    ff.setdefault("names", {})[lang] = public
            elif lang == "java":
                if public != "Java_com_finkit_Indicators_" + pub:
                    ff.setdefault("names", {})[lang] = public
        save_registry(reg)
        print(f"[discover/{lang}] matched {matched}/{len(inds)}; "
              f"unmatched: {unmatched if unmatched else 'none'}")
        total_unmatched += len(unmatched)
    return 1 if total_unmatched else 0


def guard_for(lang: str, ret: str) -> str | None:
    ret = ret.strip()
    if lang == "go":
        if ret in ("*mut TaResult", "*mut c_char"):
            return "ffi_catch_ptr"
    elif lang == "dotnet":
        if ret in ("c_int", "i32"):
            return "ffi_catch_i32"
    elif lang == "ios":
        if ret == "i32":
            return "ffi_catch_i32_neg"
    elif lang == "java":
        if ret in ("jdoubleArray", "jobject", "jintArray", "jstring", "jni::sys::jdoubleArray", "jni::sys::jobject", "jni::sys::jintArray", "jni::sys::jstring"):
            return "ffi_catch_ptr"
        if ret == "()":
            return "ffi_catch_void"
        if ret in ("jlong", "jni::sys::jlong"):
            return "ffi_catch_i64"
        if ret in ("jboolean", "jni::sys::jboolean"):
            return "ffi_catch_u8"
    return None


def wrap_body(lang: str, body: str) -> str:
    body = body.strip()
    if lang not in ("go", "dotnet", "ios", "java"):
        return body
    ri = body.find("->")
    if ri == -1:
        return body
    ob = body.find("{", ri)
    if ob == -1:
        return body
    try:
        cb = _brace_span(body, ob)
    except RuntimeError:
        return body
    inner = body[ob + 1 : cb]
    if inner.strip().startswith("ffi_catch"):
        return body
    ret = body[ri + 2 : ob].strip()
    guard = guard_for(lang, ret)
    if guard is None:
        return body
    sig = body[: ob + 1]
    return sig + "\n    " + guard + "(|| {\n" + inner + "\n    })\n}"


def emit_generated(lang: str, inds: list[dict]) -> str:
    cfg = LANG_CFG[lang]
    header = (
        "// ─────────────────────────────────────────────────────────────────────\n"
        "// GENERATED FILE — do not edit by hand.\n"
        "// Source of truth: docs/indicator_registry.json (ffi.bodies.<lang>).\n"
        f"// Regenerate with: python3 scripts/sync_bindings.py --lang {lang} --generate --rewrite\n"
        "// ─────────────────────────────────────────────────────────────────────\n\n"
    )
    bodies = []
    for ind in inds:
        body = ind.get("ffi", {}).get("bodies", {}).get(lang)
        if body:
            bodies.append(wrap_body(lang, body).rstrip("\n") + "\n")
    return header + "\n".join(bodies) + "\n"


def do_generate(langs: list[str], rewrite: bool) -> int:
    reg = load_registry()
    inds = indicators_with_ffi(reg)
    for lang in langs:
        cfg = LANG_CFG[lang]
        gen_path = ROOT / cfg["gen"]
        text = emit_generated(lang, inds)
        if rewrite:
            lib_path = ROOT / cfg["lib"]
            src = lib_path.read_text(encoding="utf-8")
            extracted = extract_functions(src, lang)
            drop_names = set()
            for ind in inds:
                nm = match_indicator(ind, lang, extracted)
                if nm and ind.get("ffi", {}).get("bodies", {}).get(lang):
                    drop_names.add(nm)
            spans = sorted(
                (info["start"], info["end"]) for nm, info in extracted.items()
                if nm in drop_names
            )
            if not spans:
                print(f"[gen/{lang}] nothing to rewrite (no registry-matched fns found)")
                continue
            result = []
            cursor = 0
            inserted = False
            for s, e in spans:
                result.append(src[cursor:s])
                if not inserted:
                    result.append('\ninclude!("generated.rs");\n')
                    inserted = True
                cursor = e
            result.append(src[cursor:])
            new_lib = "".join(result)
            lib_path.write_text(new_lib, encoding="utf-8")
            print(f"[gen/{lang}] rewrote {cfg['lib']} (dropped {len(spans)} indicator fns)")
        gen_path.write_text(text, encoding="utf-8")
        print(f"[gen/{lang}] wrote {cfg['gen']} ({len(inds)} indicators)")
    return 0


def do_check(langs: list[str]) -> int:
    reg = load_registry()
    inds = indicators_with_ffi(reg)
    rc = 0
    for lang in langs:
        cfg = LANG_CFG[lang]
        src = (ROOT / cfg["lib"]).read_text(encoding="utf-8")
        extracted = extract_functions(src, lang)
        gen_path = ROOT / cfg["gen"]
        if gen_path.exists():
            extracted.update(extract_functions(gen_path.read_text(encoding="utf-8"), lang))
        stored = {ind["ffi"]["c_name"]: ind for ind in inds}
        drift = []
        for c_name, ind in stored.items():
            body_stored = ind.get("ffi", {}).get("bodies", {}).get(lang)
            if body_stored is None:
                continue
            nm = match_indicator(ind, lang, extracted)
            if nm is None:
                drift.append(f"missing:{c_name}")
                continue
            body_now = extracted[nm]["body"]
            if wrap_body(lang, body_now).strip() != wrap_body(lang, body_stored).strip():
                drift.append(f"changed:{c_name}")
        print(f"[check/{lang}] registry={len(inds)} extracted={len(extracted)} "
              f"drift={drift if drift else 'none'}")
        if drift:
            rc = 1
    return rc


def main() -> int:
    args = sys.argv[1:]
    mode = None
    langs: list[str] = []
    rewrite = False
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--discover":
            mode = "discover"
        elif a == "--generate":
            mode = "generate"
        elif a == "--check":
            mode = "check"
        elif a == "--rewrite":
            rewrite = True
        elif a == "--lang":
            langs.append(args[i + 1])
            i += 1
        elif a.startswith("--lang="):
            langs.append(a.split("=", 1)[1])
        i += 1
    if not langs:
        langs = list(LANG_CFG.keys())
    if mode is None:
        print("usage: sync_bindings.py (--discover|--generate [--rewrite]|--check) "
              "[--lang c|python|node|go|java|dotnet|ios|android]...")
        return 2
    if mode == "discover":
        return do_discover(langs)
    if mode == "generate":
        return do_generate(langs, rewrite)
    if mode == "check":
        return do_check(langs)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
