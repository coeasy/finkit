import re
import sys

BS = chr(92)  # backslash


def brace_span(src, open_idx):
    """Return index of matching '}' for the '{' at open_idx, or -1.
    Skips string literals and // comments to avoid false braces."""
    depth = 0
    i = open_idx
    n = len(src)
    while i < n:
        c = src[i]
        if c == '"':
            i += 1
            while i < n:
                if src[i] == BS:
                    i += 2
                    continue
                if src[i] == '"':
                    i += 1
                    break
                i += 1
            continue
        if c == '/' and i + 1 < n and src[i + 1] == '/':
            while i < n and src[i] != '\n':
                i += 1
            continue
        if c == '{':
            depth += 1
        elif c == '}':
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


path = sys.argv[1]
src = open(path, encoding="utf-8").read()
parts = src.split('#[no_mangle]')
print("header len:", len(parts[0]), " num function chunks:", len(parts) - 1)
imbalanced = []
for idx, ch in enumerate(parts[1:], start=1):
    m = re.search(r'fn\s+(\w+)', ch)
    name = m.group(1) if m else '?'
    ob = ch.find('{')
    if ob == -1:
        print("  [%d] %s: NO OPEN BRACE (malformed)" % (idx, name))
        imbalanced.append(name)
        continue
    cb = brace_span(ch, ob)
    if cb == -1:
        print("  [%d] %s: MISSING CLOSE (+1 open)" % (idx, name))
        imbalanced.append(name)
        continue
    inner = ch[ob + 1:cb]
    d = 0
    for c in inner:
        if c == '{':
            d += 1
        elif c == '}':
            d -= 1
    if d != 0:
        print("  [%d] %s: INNER IMBALANCE (%d)" % (idx, name, d))
        imbalanced.append(name)
print("=== imbalanced functions:", imbalanced)
