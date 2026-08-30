"""Remove FormulaEvalJit and FormulaEvalSimd from ta.go.

These reference C symbols (ta_formula_eval_jit, ta_formula_eval_simd) that the
Rust alpha-ta-go crate does not export, so the Go build fails to link.
"""
import pathlib
import re
import sys

src = pathlib.Path(sys.argv[1])
text = src.read_text()

# Remove the JIT and SIMD extern declarations from the cgo block.
text = re.sub(
    r"extern char\* ta_formula_eval_jit\([^)]*\);\n",
    "",
    text,
)
text = re.sub(
    r"extern char\* ta_formula_eval_simd\([^)]*\);\n",
    "",
    text,
)

# Remove the FormulaEvalJit and FormulaEvalSimd function definitions.
# Match from the first `// Name` comment that precedes the func, all the way
# through the matching closing brace at column 0.
def drop_func(name):
    pat = re.compile(
        r"^[ \t]*//[^\n]*\nfunc " + re.escape(name) + r".*?^\}",
        re.DOTALL | re.MULTILINE,
    )
    return pat.sub("", text, count=1)

text = drop_func("FormulaEvalJit")
text = drop_func("FormulaEvalSimd")

src.write_text(text)
print(f"[drop-jit-simd] cleaned {src}")
