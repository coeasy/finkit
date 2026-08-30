"""Rewrite the replace directive in packaging/usage/go/tests/go.mod to
point at the absolute path of the built AlphaTA Go module."""
import re
import pathlib
import sys

mod = pathlib.Path(r"p:\llm_code\fta\packaging\usage\go\tests\go.mod")
target = r"p:/llm_code/fta/dist/go/windows-x64/AlphaTA"

text = mod.read_text()
text = re.sub(
    r"replace github\.com/alpha-ta-rs/AlphaTA => .*",
    f"replace github.com/alpha-ta-rs/AlphaTA => {target}",
    text,
)
mod.write_text(text)
print("[rewrite] rewrote", mod)
print(text)
