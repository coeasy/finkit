#!/usr/bin/env python3
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
BASE_GO_SHA = "a6603b35b04d39a668a5c59287a2600abfa19076"


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding="utf-8")


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    if old not in text:
        raise SystemExit(f"missing anchor: {label}")
    return text.replace(old, new, 1)


def function_span(text: str, name: str) -> tuple[int, int]:
    match = re.search(rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+{re.escape(name)}\s*\(", text)
    if not match:
        raise SystemExit(f"function not found: {name}")
    brace = text.find("{", match.start())
    if brace < 0:
        raise SystemExit(f"opening brace not found: {name}")
    depth = 0
    for idx in range(brace, len(text)):
        if text[idx] == "{":
            depth += 1
        elif text[idx] == "}":
            depth -= 1
            if depth == 0:
                return match.start(), idx + 1
    raise SystemExit(f"closing brace not found: {name}")


def remove_function(text: str, name: str) -> str:
    start, end = function_span(text, name)
    while end < len(text) and text[end] == "\n":
        end += 1
    return text[:start] + text[end:]


def rename_and_alias(text: str, old: str, new: str) -> str:
    old_decl = f"pub fn {old}("
    new_decl = f"pub fn {new}("
    if old_decl in text:
        text = text.replace(old_decl, new_decl, 1)
    elif new_decl not in text:
        raise SystemExit(f"missing rename anchor: {old}")
    alias = f"pub use {new} as {old};"
    if alias not in text:
        _, end = function_span(text, new)
        text = text[:end] + f"\n\n/// Backward-compatible alias for [`{new}`].\n{alias}" + text[end:]
    return text


# Restore the complete Go source after a previous partial Contents-API write,
# then apply the actual ABI conversion locally.
subprocess.run(
    ["git", "checkout", BASE_GO_SHA, "--", "ffi/go-binding/go/ta/ta.go"],
    cwd=ROOT,
    check=True,
)
go_path = "ffi/go-binding/go/ta/ta.go"
go = read(go_path)
go = replace_once(
    go,
    "cInt(fastPeriod), cInt(fastMa), cInt(slowPeriod), cInt(slowMa), cInt(signalPeriod), cInt(signalMa),",
    "cInt(fastPeriod), cInt(int(fastMa)), cInt(slowPeriod), cInt(int(slowMa)), cInt(signalPeriod), cInt(int(signalMa)),",
    "Go MaType ABI conversion",
)
write(go_path, go)

# HT_SINE: reuse the SIMD sin/cos result for lead-sine instead of calling
# scalar sin once more per output bar. Reduction preserves sine but may flip
# cosine, so retain that sign explicitly.
cycle_path = "core/src/indicators/cycle.rs"
cycle = read(cycle_path)
cycle = replace_once(
    cycle,
    "let mut phase_radians = vec![0.0_f64; len - 63];\n    let pi = std::f64::consts::PI;",
    "let mut phase_radians = vec![0.0_f64; len - 63];\n    let mut phase_cos_sign = vec![1.0_f64; len - 63];\n    let pi = std::f64::consts::PI;",
    "HT_SINE cosine sign buffer",
)
cycle = replace_once(
    cycle,
    "if phase > pi / 2.0 {\n            phase = pi - phase;\n        } else if phase < -pi / 2.0 {\n            phase = -pi - phase;\n        }",
    "if phase > pi / 2.0 {\n            phase = pi - phase;\n            phase_cos_sign[offset] = -1.0;\n        } else if phase < -pi / 2.0 {\n            phase = -pi - phase;\n            phase_cos_sign[offset] = -1.0;\n        }",
    "HT_SINE cosine sign reduction",
)
cycle = replace_once(
    cycle,
    "for i in 63..len {\n        sine[i] = phase_sin[i];\n        lead_sine[i] = ((dc_phase[i] + 45.0) * deg2rad).sin();\n    }",
    "for (offset, i) in (63..len).enumerate() {\n        let sin = phase_sin[i];\n        let cos = phase_cos[offset] * phase_cos_sign[offset];\n        sine[i] = sin;\n        lead_sine[i] = (sin + cos) * std::f64::consts::FRAC_1_SQRT_2;\n    }",
    "HT_SINE vectorized lead sine",
)
write(cycle_path, cycle)

# Rank SSOT: statistics::spearman_rank delegates to the canonical rank kernel.
stats_path = "core/src/math/statistics.rs"
stats = read(stats_path)
stats = stats.replace("let rank_x = fractional_ranks(x);", "let rank_x = crate::math::rank::fractional_ranks(x);", 1)
stats = stats.replace("let rank_y = fractional_ranks(y);", "let rank_y = crate::math::rank::fractional_ranks(y);", 1)
if re.search(r"(?m)^fn fractional_ranks\s*\(", stats):
    stats = remove_function(stats, "fractional_ranks")
write(stats_path, stats)

# Regression SSOT: Hurst slope delegates to canonical simple_slope(y, x).
rolling_path = "core/src/features/rolling_stats.rs"
rolling = read(rolling_path)
rolling = replace_once(
    rolling,
    "linear_regression_slope(&log_n, &log_rs)",
    "crate::math::regression::simple_slope(&log_rs, &log_n).unwrap_or(f64::NAN)",
    "Hurst canonical regression slope",
)
if re.search(r"(?m)^fn linear_regression_slope\s*\(", rolling):
    rolling = remove_function(rolling, "linear_regression_slope")
write(rolling_path, rolling)

# These are rolling price/equity indicators, not the whole-series risk metrics.
# Give them explicit canonical names and preserve the old Rust API via aliases.
vol_path = "core/src/indicators/volatility_ext.rs"
vol = read(vol_path)
vol = rename_and_alias(vol, "sortino_ratio", "rolling_sortino_ratio")
vol = rename_and_alias(vol, "max_drawdown", "rolling_max_drawdown")
write(vol_path, vol)

# WASM functions are ABI facades. Keep their JavaScript names stable while
# avoiding pretending that they own the underlying risk algorithms.
wasm_path = "wasm/src/lib.rs"
wasm = read(wasm_path)
wasm = replace_once(
    wasm,
    "#[wasm_bindgen]\npub fn sortino_ratio(",
    "#[wasm_bindgen(js_name = sortino_ratio)]\npub fn wasm_sortino_ratio(",
    "WASM Sortino facade",
)
wasm = replace_once(
    wasm,
    "#[wasm_bindgen]\npub fn max_drawdown(",
    "#[wasm_bindgen(js_name = max_drawdown)]\npub fn wasm_max_drawdown(",
    "WASM max drawdown facade",
)
write(wasm_path, wasm)

# The forward-return legacy API is already a thin canonical delegate. Make the
# SSOT checker match its documented contract: allow a same-named compatibility
# facade only when its body references the canonical delegate marker and does
# not contain an explicit loop (which would indicate an algorithm reimplementation).
ssot_path = "scripts/check_research_ssot.py"
ssot = read(ssot_path)
ssot = replace_once(
    ssot,
    "SKIP_PARTS = {\"target\", \".git\"}\nviolations: list[str] = []",
    "DELEGATE_MARKERS = {\n    \"forward_return_arithmetic\": (\"core_forward_return(\", \"crate::returns::forward_return(\"),\n}\n\nSKIP_PARTS = {\"target\", \".git\"}\nviolations: list[str] = []\n\ndef function_body(text: str, start: int) -> str:\n    brace = text.find(\"{\", start)\n    if brace < 0:\n        return \"\"\n    depth = 0\n    for idx in range(brace, len(text)):\n        if text[idx] == \"{\":\n            depth += 1\n        elif text[idx] == \"}\":\n            depth -= 1\n            if depth == 0:\n                return text[brace + 1:idx]\n    return \"\"\n\ndef is_thin_delegate(name: str, body: str) -> bool:\n    markers = DELEGATE_MARKERS.get(name, ())\n    if not markers or not any(marker in body for marker in markers):\n        return False\n    return not re.search(r\"(?m)^\\s*(?:for|while|loop)\\b\", body)",
    "SSOT delegate-aware checker helpers",
)
ssot = replace_once(
    ssot,
    "if pattern.search(text) and relative not in owners:\n            violations.append(\n                f\"{relative}: defines {name} outside canonical owner(s) {sorted(owners)}\"\n            )",
    "match = pattern.search(text)\n        if match and relative not in owners:\n            body = function_body(text, match.start())\n            if is_thin_delegate(name, body):\n                continue\n            violations.append(\n                f\"{relative}: defines {name} outside canonical owner(s) {sorted(owners)}\"\n            )",
    "SSOT delegate-aware enforcement",
)
write(ssot_path, ssot)

# Visualization example follows the current chart exporter API. The GPU/LOD
# path is still exercised by build_draw_list; HTML serialization uses the
# canonical save_as_html entry point.
example_path = "visualization/examples/gpu_large_chart.rs"
example = read(example_path)
example = replace_once(
    example,
    ".save_as_webgpu_html(\"gpu_large_chart.html\")",
    ".save_as_html(\"gpu_large_chart.html\")",
    "visualization HTML exporter",
)
write(example_path, example)

print("PR #29 final source fixups applied")
