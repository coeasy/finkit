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
    # Android re-uses the JVM binding's plain-Rust shim functions via a macro;
    # we extract the macro invocations verbatim.
    "android": {
        "lib": "ffi/android-binding/src/lib.rs",
        "gen": "ffi/android-binding/src/generated.rs",
        "sig": r'shim_indicator!\s*\(',
        "kind": "android_shim",
    },
}

# Known public-name mismatches between the core function name (used by C/Go/
# .NET/iOS and as the registry key) and the Python/Node public name.
# key = core name, value = public name used by python & node.
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
    out.sort(key=lambda i: i["ffi"].get("order", 0))
    return out


# ──────────────────────────────────────────────────────────────────────────
# Verbatim body extraction
# ──────────────────────────────────────────────────────────────────────────
def _brace_span(src: str, open_idx: int) -> int:
    """Given index of an opening `{`, return index just past the matching `}`."""
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
    """Return {fn_name: {"body": str, "start": int, "end": int}} for every
    indicator-like function in ``src`` for the given language."""
    cfg = LANG_CFG[lang]
    sig = cfg["sig"]
    name_group = cfg.get("sig_name_group", 1)
    out: dict[str, dict] = {}
    for m in re.finditer(sig, src):
        if lang == "android":
            # capture the whole `shim_indicator!(...)` invocation (balanced parens)
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
            # The macro expands to items, so the original source terminates it
            # with `;` (or braces).  Keep the trailing `;` so the relocated copy
            # in generated.rs still compiles.
            end = j + 1
            if end < len(src) and src[end] == ";":
                end += 1
            body = src[m.start():end]
            # the JNI symbol is the first macro arg
            first_arg = body.split("(", 1)[1].split(",", 1)[0].strip()
            name = first_arg
            out[name] = {"body": body, "start": m.start(), "end": end}
            continue

        name = m.group(name_group)
        # walk backward to include preceding `///` doc lines and `#[...]` attrs
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
        # find signature close `)` then opening `{`
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
        # skip whitespace to `{`
        k = src.find("{", i)
        end = _brace_span(src, k)
        body = src[start:end]
        out[name] = {"body": body, "start": start, "end": end}
    return out


# ──────────────────────────────────────────────────────────────────────────
# Registry <-> binding name resolution
# ──────────────────────────────────────────────────────────────────────────
def candidate_names(ind: dict, lang: str) -> list[str]:
    ff = ind["ffi"]
    c_name = ff["c_name"]
    # Canonical public name: strip the C `ta_` prefix (ta_cdl_doji -> cdl_doji,
    # ta_sma -> sma).  The registry's `core_call` basename is NOT reliable for
    # candlestick patterns (e.g. ta_cdl_doji calls indicators::cdl::doji).
    pub = c_name[3:] if c_name.startswith("ta_") else c_name
    core = ff["core_call"].split("::")[-1]
    if lang in ("c", "go", "dotnet"):
        # go/dotnet expose chart/pattern indicators as `<c_name>_json`
        # (JSON-serialised) variants rather than the plain C ABI; accept both.
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
        names.append(core)  # last-resort fallback
        return names
    if lang == "java":
        names = []
        if pub in NAME_ALIASES:
            names.append("Java_com_finkit_Indicators_" + NAME_ALIASES[pub])
        names.append("Java_com_finkit_Indicators_" + pub)
        names.append("Java_com_finkit_Indicators_" + core)
        return names
    if lang == "android":
        # The `shim_indicator!` macro's 1st argument IS the C FFI name
        # (e.g. `shim_indicator!(ta_sma, jdoubleArray, jdoubleArray)`); the
        # 2nd/3rd args are JNI array types, not the core name.  Match on the
        # registry `c_name` directly.
        return [c_name]
    return [pub]


def match_indicator(ind: dict, lang: str, extracted: dict[str, dict]) -> str | None:
    """Return the extracted binding fn name that corresponds to ``ind``."""
    for cand in candidate_names(ind, lang):
        if cand in extracted:
            return cand
    # android: match by core name appearing as the 2nd macro arg
    if lang == "android":
        core = ind["ffi"]["core_call"].split("::")[-1]
        alias = NAME_ALIASES.get(core, core)
        for nm, info in extracted.items():
            if (f", {core}," in info["body"]) or (f", {alias}," in info["body"]):
                return nm
    return None


# ──────────────────────────────────────────────────────────────────────────
# Modes
# ──────────────────────────────────────────────────────────────────────────
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
    """Return the `ffi_catch_*` call expression for a given language + return
    type, or ``None`` if the function should not be panic-wrapped."""
    ret = ret.strip()
    if lang == "go":
        # `T` is inferred from the closure's return type (*mut TaResult /
        # *mut c_char); an explicit turbofish would fill `F` positionally
        # and break. `ffi_catch_ptr` is <F, T> where F: FnOnce()->*mut T.
        if ret in ("*mut TaResult", "*mut c_char"):
            return "ffi_catch_ptr"
    elif lang == "dotnet":
        if ret in ("c_int", "i32"):
            return "ffi_catch_i32"
    elif lang == "ios":
        if ret == "i32":
            return "ffi_catch_i32_neg"
    elif lang == "java":
        # jdoubleArray / jobject are both `*mut _jobject`; `ffi_catch_ptr`
        # infers `T` from the closure's return type, so no turbofish.
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
    """Wrap a registry-stored FFI function body in ``catch_unwind`` so a panic
    inside the core call cannot unwind across the FFI boundary (which would be
    UB and abort the host).

    Idempotent: if the body is already wrapped it is returned unchanged, so
    re-running the generator or ``--discover`` is stable.
    """
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
        return body  # already wrapped
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
            # Wrap each generated function in catch_unwind so a panic inside
            # the core call cannot unwind across the FFI boundary.
            bodies.append(wrap_body(lang, body).rstrip("\n") + "\n")
    return header + "\n".join(bodies) + "\n"


def do_generate(langs: list[str], rewrite: bool) -> int:
    reg = load_registry()
    inds = indicators_with_ffi(reg)
    by_c = {i["ffi"]["c_name"]: i for i in inds}
    for lang in langs:
        cfg = LANG_CFG[lang]
        gen_path = ROOT / cfg["gen"]
        text = emit_generated(lang, inds)
        if rewrite:
            lib_path = ROOT / cfg["lib"]
            src = lib_path.read_text(encoding="utf-8")
            # drop hand-written indicator spans
            extracted = extract_functions(src, lang)
            # Only drop a function if it is registry-matched *and* we actually
            # have a verbatim body for it in the registry.  Functions without a
            # stored body (e.g. non-registry `cdl_*`/`detect_*`/`dx` in Python,
            # `*_json` graph variants in Go/.NET) stay in lib.rs untouched so
            # their registrations keep resolving.  This is what previously
            # corrupted the Python/Node bindings.
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
            # Rebuild lib.rs keeping EVERY gap between dropped spans (those gaps
            # hold the non-registry functions we must preserve, e.g. `cdl_*`,
            # `detect_*`, `dx`, `var` in Python).  Only the dropped spans
            # themselves are removed; `include!("generated.rs")` is inserted once
            # just before the first dropped span (after the module preamble).
            result = []
            cursor = 0
            inserted = False
            for s, e in spans:
                result.append(src[cursor:s])  # keep gap (non-dropped fns) before span
                if not inserted:
                    result.append('\ninclude!("generated.rs");\n')
                    inserted = True
                cursor = e  # skip the dropped span itself
            result.append(src[cursor:])  # keep tail after last span
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
        # After a --rewrite, registry-matched bodies live in generated.rs; read
        # it too so we don't false-positive "missing" on relocated functions.
        gen_path = ROOT / cfg["gen"]
        if gen_path.exists():
            extracted.update(extract_functions(gen_path.read_text(encoding="utf-8"), lang))
        stored = {ind["ffi"]["c_name"]: ind for ind in inds}
        drift = []
        for c_name, ind in stored.items():
            body_stored = ind.get("ffi", {}).get("bodies", {}).get(lang)
            # Only indicators the registry says are exposed in this language
            # (i.e. have a stored body) are drift-checked.  Indicators a binding
            # intentionally does NOT expose as a standalone function (e.g.
            # `cdl_*` via a dispatcher, `darvas_box`/`renko` via `match`,
            # `midpoint`/`ht_*`) have no stored body and are not drift.
            if body_stored is None:
                continue
            nm = match_indicator(ind, lang, extracted)
            if nm is None:
                drift.append(f"missing:{c_name}")
                continue
            body_now = extracted[nm]["body"]
            # Compare through the same panic-wrapper normalisation so a
            # regenerated (wrapped) function is not flagged as drift against
            # the unwrapped source-of-truth body.
            if wrap_body(lang, body_now).strip() != wrap_body(lang, body_stored).strip():
                drift.append(f"changed:{c_name}")
        # also: hand-written fns present that the registry dropped?
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
