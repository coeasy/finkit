"""Patch ta.go cgo LDFLAGS to point at the staged lib directory.

Run by scripts/build-usage-go.sh after copying ffi/go-binding/go/ into
dist/go/<plat>/AlphaTA/. The source ta.go references `../target/release`
(relative to the source tree); the staged module no longer lives next to
target/, so rewrite the LDFLAGS to `${SRCDIR}/../../lib`, which lands
at dist/go/<plat>/lib relative to ta.go.
"""
import sys
import pathlib

src = pathlib.Path(sys.argv[1])
text = src.read_text()

OLD_WIN = '#cgo windows LDFLAGS: -L../target/release -lalpha_ta_go -lws2_32 -ladvapi32 -luserenv -lbcrypt -lncrypt -lschannel -luser32'
NEW_WIN = '#cgo windows LDFLAGS: -L${SRCDIR}/../../lib -lalpha_ta_go -lws2_32 -ladvapi32 -luserenv -lbcrypt -lncrypt -lschannel -luser32'

OLD_NIX = '#cgo !windows LDFLAGS: -L../target/release -lalpha_ta_go -lm -ldl -lpthread'
NEW_NIX = '#cgo !windows LDFLAGS: -L${SRCDIR}/../../lib -lalpha_ta_go -lm -ldl -lpthread'

new_text = text.replace(OLD_WIN, NEW_WIN)
new_text = new_text.replace(OLD_NIX, NEW_NIX)
src.write_text(new_text)
changed = (new_text != text)
print(f"[patch-cgo] rewrote LDFLAGS in {src} (changed={changed})")

