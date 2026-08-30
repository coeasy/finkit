#!/usr/bin/env python3
"""Robustly recover over-wrapped FFI generated.rs files (v3).

The generated.rs files were wrapped with nested ffi_catch(|| {...}) layers
(double/triple wrap). The buggy first wrap embedded the *original* fn-close
`}` inside the guard body, so a naive peel leaves one extra `}`.

This script:
  1. takes the content between the fn's own `{}` (always the outermost),
  2. peels every `ffi_catch(|| { ... })` layer down to the real statements,
  3. re-wraps exactly once, stripping a trailing `}` only if the result
     would otherwise be brace-unbalanced (that is the embedded fn-close).

It is idempotent: a correctly single-wrapped (or clean) function passes
through unchanged.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


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


def guard_for(lang: str, ret: str):
    ret = ret.strip()
    if lang == "go":
        # `T` is inferred from the closure's return type (*mut TaResult /
        # *mut c_char); an explicit turbofish would fill `F` positionally
        # and break. `ffi_catch_ptr` has signature <F, T> where
        # F: FnOnce() -> *mut T.
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
        if ret in ("jdoubleArray", "jobject"):
            return "ffi_catch_ptr"
    return None


def peel(s: str) -> str:
    """Unwrap every nested ``ffi_catch(|| { ... })`` layer -> inner statements."""
    s = s.strip()
    while s.startswith("ffi_catch"):
        p = s.find("(||")
        if p == -1:
            break
        ob = s.find("{", p)
        if ob == -1:
            break
        try:
            cb = _brace_span(s, ob)
        except RuntimeError:
            break
        s = s[ob + 1 : cb].strip()
    return s


def is_balanced(s: str) -> bool:
    depth = 0
    for c in s:
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


def recover_block(block: str, lang: str) -> str:
    block = block.strip()
    ri = block.find("->")
    if ri == -1:
        return block
    ob = block.find("{", ri)
    if ob == -1:
        return block
    try:
        cb = _brace_span(block, ob)
    except RuntimeError:
        return block
    sig = block[: ob + 1]                   # includes fn-open '{'
    ret = block[ri + 2 : ob].strip()
    fn_inner = block[ob + 1 : cb].strip()    # content between fn's own {}
    body = peel(fn_inner)                     # real statements (maybe +embedded fn-close '}')
    guard = guard_for(lang, ret)
    if guard is None:
        print(f"  ! skip (no guard for ret={ret!r})")
        return block
    # Strip a trailing '}' only while it would make the wrapped fn unbalanced
    # (that trailing '}' is the embedded original fn-close).
    while body.rstrip().endswith("}") and not is_balanced(
        sig + "\n    " + guard + "(|| {\n" + body + "\n    })\n}"
    ):
        body = body.rstrip()[:-1].rstrip()
    return sig + "\n    " + guard + "(|| {\n" + body + "\n    })\n}"


def recover_file(rel_path: str, lang: str) -> int:
    path = ROOT / rel_path
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"(\S)#\[no_mangle\]", r"\1\n#[no_mangle]", text)
    parts = text.split("#[no_mangle]")
    header = parts[0].rstrip("\n")
    out = [header]
    count = 0
    for ch in parts[1:]:
        rec = recover_block(ch, lang)
        out.append("#[no_mangle]\n" + rec)
        count += 1
    result = "\n\n".join(out).rstrip("\n") + "\n"
    path.write_text(result, encoding="utf-8")
    return count


if __name__ == "__main__":
    targets = {
        "go": "ffi/go-binding/src/generated.rs",
        "dotnet": "ffi/dotnet-binding/src/generated.rs",
        "ios": "ffi/ios-binding/src/generated.rs",
        "java": "ffi/java-binding/src/generated.rs",
    }
    for lang, p in targets.items():
        n = recover_file(p, lang)
        print(f"{lang}: recovered {n} functions in {p}")
