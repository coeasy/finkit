"""Strip CRLF line endings from .go files in the staged Go module.

This runs after `cp -r` (which on git-bash on Windows turns LF into CRLF).
cgo refuses to parse CRLF source, so we force LF here.

Usage: python3 normalize_go_line_endings.py <root>
"""
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
converted = 0
for f in root.rglob("*.go"):
    data = f.read_bytes()
    if b"\r\n" in data:
        f.write_bytes(data.replace(b"\r\n", b"\n"))
        converted += 1
print(f"[normalize-go] rewrote {converted} .go files under {root}")
