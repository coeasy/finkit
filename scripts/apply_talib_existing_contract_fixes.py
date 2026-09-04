#!/usr/bin/env python3
"""Migrate legacy tests and into-path warm-up contracts to TA-Lib semantics.

These edits intentionally live beside the semantic migration because several
pre-existing unit tests encoded Finkit's older warm-up behavior.  Keeping those
assertions unchanged would reject the corrected implementation.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_once(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old in text:
        path.write_text(text.replace(old, new, 1), encoding="utf-8")
        print(f"{label}: updated")
    elif new in text:
        print(f"{label}: already updated")
    else:
        raise RuntimeError(f"{label}: expected source fragment not found")


def fix_trange_test() -> None:
    path = ROOT / "core/src/indicators/volatility.rs"
    replace_once(
        path,
        "        assert_relative_eq!(result[0], 2.0, epsilon = 1e-10);\n        assert_eq!(result.len(), 3);",
        "        assert!(result[0].is_nan());\n        assert_relative_eq!(result[1], 3.0, epsilon = 1e-10);\n        assert_eq!(result.len(), 3);",
        "TRANGE legacy test",
    )


def fix_kama_test() -> None:
    path = ROOT / "core/src/math/moving_avg.rs"
    replace_once(
        path,
        "        assert!(result[0].is_nan());\n        assert!(result[3].is_nan());\n        assert_relative_eq!(result[4], 5.0, epsilon = 1e-10);",
        "        assert!(result[0].is_nan());\n        assert!(result[4].is_nan());\n        assert_relative_eq!(result[5], 5.444444444444445, epsilon = 1e-10);",
        "KAMA legacy test",
    )


def fix_momentum_kama_test() -> None:
    """Update the second legacy KAMA test introduced by the momentum test module.

    After TA-Lib alignment the seed at period-1 is internal only and the first
    public value is emitted at index `period`.  The old unit contract expected
    that index to remain NaN.  Patch only the named KAMA test block so unrelated
    NaN assertions in momentum.rs are untouched.
    """
    path = ROOT / "core/src/indicators/momentum.rs"
    text = path.read_text(encoding="utf-8")
    marker = "fn test_kama_basic()"
    start = text.find(marker)
    if start < 0:
        # Some source revisions do not contain this generated/legacy test until
        # the earlier migration steps run; absence is therefore not an error.
        print("momentum KAMA legacy test: not present")
        return
    brace = text.find("{", start)
    if brace < 0:
        raise RuntimeError("momentum KAMA legacy test opening brace not found")
    depth = 0
    end = -1
    for i in range(brace, len(text)):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    if end < 0:
        raise RuntimeError("momentum KAMA legacy test closing brace not found")
    block = text[start:end]
    old = "        assert!(result[3].is_nan());"
    new = "        assert!(result[3].is_finite());"
    if old in block:
        block = block.replace(old, new, 1)
        path.write_text(text[:start] + block + text[end:], encoding="utf-8")
        print("momentum KAMA legacy test: updated")
    elif new in block:
        print("momentum KAMA legacy test: already updated")
    else:
        raise RuntimeError("momentum KAMA legacy assertion not found")


def fix_macd_into_and_test() -> None:
    path = ROOT / "core/src/indicators/momentum.rs"
    text = path.read_text(encoding="utf-8")

    old_validate = "    validate_input(input.len(), slow_period)?;"
    new_validate = "    validate_input(input.len(), slow_period + signal_period - 1)?;"
    # Only change the validation inside macd_into; locate the function first.
    start = text.find("pub fn macd_into(")
    if start < 0:
        raise RuntimeError("macd_into not found")
    end = text.find("\n/// ADX zero-copy variant", start)
    if end < 0:
        raise RuntimeError("macd_into end marker not found")
    segment = text[start:end]
    if old_validate in segment:
        segment = segment.replace(old_validate, new_validate, 1)
    elif new_validate not in segment:
        raise RuntimeError("macd_into validation fragment not found")

    old_mask = '''    // 预热区填 NaN
    for i in 0..macd_start.min(len) {
        macd_line[i] = f64::NAN;
    }
    for i in 0..signal_start.min(len) {
        signal[i] = f64::NAN;
        histogram[i] = f64::NAN;
    }
'''
    new_mask = '''    // TA-Lib exposes MACD, signal and histogram from one common lookback.
    // Earlier MACD values are only seed material and must not escape the API.
    for i in 0..signal_start.min(len) {
        macd_line[i] = f64::NAN;
        signal[i] = f64::NAN;
        histogram[i] = f64::NAN;
    }
'''
    if old_mask in segment:
        segment = segment.replace(old_mask, new_mask, 1)
    elif new_mask not in segment:
        raise RuntimeError("macd_into warm-up mask fragment not found")
    text = text[:start] + segment + text[end:]

    old_test = '''        let result = macd(&input, 12, 26, 9).unwrap();
        assert!(!result.macd[25].is_nan());'''
    old_generated_test = '''        let result = macd(&input, 12, 26, 9).unwrap();
        let lookback = 26 + 9 - 2;
        assert!(result.macd[..lookback].iter().all(|value| value.is_nan()));
        assert!(result.signal[..lookback].iter().all(|value| value.is_nan()));
        assert!(result.hist[..lookback].iter().all(|value| value.is_nan()));
        assert!(result.macd[lookback].is_finite());
        assert!(result.signal[lookback].is_finite());
        assert!(result.hist[lookback].is_finite());'''
    new_test = '''        let result = macd(&input, 12, 26, 9).unwrap();
        let lookback = 26 + 9 - 2;
        assert!(result.macd.iter().take(lookback).all(|value| value.is_nan()));
        assert!(result.signal.iter().take(lookback).all(|value| value.is_nan()));
        assert!(result.hist.iter().take(lookback).all(|value| value.is_nan()));
        assert!(result.macd[lookback].is_finite());
        assert!(result.signal[lookback].is_finite());
        assert!(result.hist[lookback].is_finite());'''
    if old_test in text:
        text = text.replace(old_test, new_test, 1)
    elif old_generated_test in text:
        text = text.replace(old_generated_test, new_test, 1)
    elif new_test not in text:
        raise RuntimeError("MACD legacy test fragment not found")

    path.write_text(text, encoding="utf-8")
    print("MACD into/test contracts: updated")


def main() -> int:
    fix_trange_test()
    fix_kama_test()
    fix_momentum_kama_test()
    fix_macd_into_and_test()
    print("legacy TA-Lib contract migration complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
