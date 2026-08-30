import re
import sys

BS = chr(92)


def brace_span(src, open_idx):
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


def net_braces(s):
    d = 0
    for c in s:
        if c == '{':
            d += 1
        elif c == '}':
            d -= 1
    return d


path = sys.argv[1]
out_path = path + ".new"
src = open(path, encoding="utf-8").read()
parts = src.split('#[no_mangle]')
header = parts[0]
chunks = parts[1:]
n = len(chunks)
out = [header]
review = []
for idx, ch in enumerate(chunks):
    # Every function chunk in the original was preceded by a #[no_mangle]
    # delimiter (that is how split produced it), so always re-prepend it.
    pre = '#[no_mangle]'
    m = re.search(r'fn\s+(\w+)', ch)
    name = m.group(1) if m else ('chunk%d' % (idx + 1))
    ob = ch.find('{')
    if ob == -1:
        review.append(name + ': no open brace')
        out.append(pre + ch)
        continue
    cb = brace_span(ch, ob)
    if cb != -1:
        # already balanced: re-emit inner + close
        inner = ch[ob + 1:cb]
        newfn = ch[:ob + 1] + inner + '}'
    else:
        # missing close: append one '}' to the whole function body
        inner = ch[ob + 1:]
        newfn = ch[:ob + 1] + inner.rstrip() + '\n}'
    # verify
    if net_braces(newfn) != 0:
        review.append(name + ': still imbalanced after fix (net=%d)' % net_braces(newfn))
    out.append(pre + newfn)

open(out_path, 'w', encoding='utf-8').write(''.join(out))
print('wrote', out_path)
print('functions:', n)
print('NEEDS REVIEW:', review if review else 'none')
