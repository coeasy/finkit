#!/usr/bin/env python3
"""Apply TA-Lib 0.7.x semantic alignment fixes.

The expanded installed-wheel benchmark exposed a small set of differences that
were not algorithmic precision failures: warm-up masks, duplicated formula
implementations, and one statistics convention mismatch.  Keep the migration
script in-tree so the changes are reviewable/reproducible while the large Rust
files remain generated/maintained in their existing locations.
"""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _function_span(text: str, needle: str) -> tuple[int, int]:
    start = text.find(needle)
    if start < 0:
        raise RuntimeError(f"function marker not found: {needle}")
    brace = text.find("{", start)
    if brace < 0:
        raise RuntimeError(f"opening brace not found: {needle}")
    depth = 0
    quote: str | None = None
    escaped = False
    i = brace
    while i < len(text):
        ch = text[i]
        if quote is not None:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            i += 1
            continue
        if ch in ('"', "'"):
            quote = ch
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return start, i + 1
        i += 1
    raise RuntimeError(f"unbalanced function: {needle}")


def _replace_function(path: Path, needle: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    if replacement.strip() in text:
        return
    start, end = _function_span(text, needle)
    text = text[:start] + replacement.rstrip() + text[end:]
    path.write_text(text, encoding="utf-8")


def _edit_function(path: Path, needle: str, edits: list[tuple[str, str]]) -> None:
    text = path.read_text(encoding="utf-8")
    start, end = _function_span(text, needle)
    body = text[start:end]
    changed = False
    for old, new in edits:
        if old in body:
            body = body.replace(old, new, 1)
            changed = True
        elif new not in body:
            raise RuntimeError(f"{path}:{needle}: fragment not found: {old[:80]!r}")
    if changed:
        path.write_text(text[:start] + body + text[end:], encoding="utf-8")


def fix_trange() -> None:
    path = ROOT / "core/src/indicators/volatility.rs"
    _edit_function(
        path,
        "pub fn trange(",
        [
            (
                "    let mut output = Array1::zeros(len);\n\n    output[0] = high[0] - low[0];",
                "    let mut output = init_output(len);\n\n    // TA-Lib lookback is one bar because true range requires previous close.\n    output[0] = f64::NAN;",
            )
        ],
    )


def fix_adosc_warmup() -> None:
    path = ROOT / "core/src/indicators/volume.rs"
    _edit_function(
        path,
        "pub fn adosc(",
        [
            (
                "    let mut output = Array1::<f64>::zeros(len);",
                "    let mut output = init_output(len);",
            )
        ],
    )


def fix_kama_warmup() -> None:
    path = ROOT / "core/src/math/moving_avg.rs"
    _edit_function(
        path,
        "pub fn kama(",
        [
            ("    validate_input(input.len(), period)?;", "    validate_input(input.len(), period + 1)?;"),
            (
                "    output[period - 1] = input[period - 1];",
                "    // TA-Lib consumes the seed at period-1 but does not expose it.\n    let seed = input[period - 1];",
            ),
            (
                "        output[period] = output[period - 1] + sc * (input[period] - output[period - 1]);",
                "        output[period] = seed + sc * (input[period] - seed);",
            ),
        ],
    )


def fix_macd_warmup() -> None:
    path = ROOT / "core/src/indicators/momentum.rs"
    _edit_function(
        path,
        "fn macd_inner(",
        [
            (
                "    Ok(MacdResult {\n        macd: macd_line,",
                "    // TA-Lib exposes all three MACD outputs from the common signal lookback.\n    // Keep the earlier MACD values only as internal seed material, then mask them.\n    for i in macd_start..signal_start {\n        macd_line[i] = f64::NAN;\n    }\n\n    Ok(MacdResult {\n        macd: macd_line,",
            )
        ],
    )


def fix_sar() -> None:
    path = ROOT / "core/src/indicators/overlap.rs"
    replacement = r'''pub fn sar(high: &[f64], low: &[f64], acceleration: f64, maximum: f64) -> Result<SarResult> {
    if high.len() != low.len() {
        return Err(TaError::InvalidParameter {
            name: "high and low".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    if high.len() < 2 {
        return Err(TaError::InsufficientData {
            length: high.len(),
            required: 2,
        });
    }
    if !acceleration.is_finite()
        || !maximum.is_finite()
        || acceleration < 0.0
        || maximum < 0.0
    {
        return Err(TaError::InvalidParameter {
            name: "acceleration/maximum".to_string(),
            constraint: "must be finite and >= 0".to_string(),
        });
    }

    let len = high.len();
    let mut sar_values = init_output(len);
    let mut af_values = init_output(len);

    // TA-Lib chooses the initial direction from +DM/-DM between bars 0 and 1.
    // A strictly larger positive -DM starts short; ties default to long.
    let up_move = high[1] - high[0];
    let down_move = low[0] - low[1];
    let mut is_long = !(down_move > up_move && down_move > 0.0);

    let step = acceleration.min(maximum);
    let mut af = step;
    let mut ep;
    let mut sar;
    if is_long {
        ep = high[1];
        sar = low[0];
    } else {
        ep = low[1];
        sar = high[0];
    }

    // TA-Lib SAR has lookback 1.  The loop emits the SAR for the current bar,
    // then advances the state for the next bar.
    let mut new_high = high[1];
    let mut new_low = low[1];
    for i in 1..len {
        let prev_high = new_high;
        let prev_low = new_low;
        new_high = high[i];
        new_low = low[i];

        if is_long {
            if new_low <= sar {
                is_long = false;
                sar = ep.max(prev_high).max(new_high);
                sar_values[i] = sar;
                af_values[i] = step;
                af = step;
                ep = new_low;
                sar = (ep - sar).mul_add(af, sar);
                sar = sar.max(prev_high).max(new_high);
            } else {
                sar_values[i] = sar;
                af_values[i] = af;
                if new_high > ep {
                    ep = new_high;
                    af = (af + step).min(maximum);
                }
                sar = (ep - sar).mul_add(af, sar);
                sar = sar.min(prev_low).min(new_low);
            }
        } else if new_high >= sar {
            is_long = true;
            sar = ep.min(prev_low).min(new_low);
            sar_values[i] = sar;
            af_values[i] = step;
            af = step;
            ep = new_high;
            sar = (ep - sar).mul_add(af, sar);
            sar = sar.min(prev_low).min(new_low);
        } else {
            sar_values[i] = sar;
            af_values[i] = af;
            if new_low < ep {
                ep = new_low;
                af = (af + step).min(maximum);
            }
            sar = (ep - sar).mul_add(af, sar);
            sar = sar.max(prev_high).max(new_high);
        }
    }

    Ok(SarResult {
        sar: sar_values,
        af: af_values,
    })
}'''
    _replace_function(path, "pub fn sar(", replacement)


def fix_formula_delegation() -> None:
    path = ROOT / "core/src/formula/functions.rs"

    _replace_function(
        path,
        "fn fn_std(",
        r'''fn fn_std(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("STD", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "STD")?;
    let data_len = ctx.data_len;
    match crate::indicators::std_dev(input.as_slice().unwrap(), n, 1.0) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )
    _replace_function(
        path,
        "fn fn_var(",
        r'''fn fn_var(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("VAR", args, 2)?;
    let input = &args[0];
    let n = extract_n(args, 1, "VAR")?;
    let data_len = ctx.data_len;
    match crate::indicators::var(input.as_slice().unwrap(), n, 1.0) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )

    # Bollinger formula helpers must use the same population standard deviation
    # as the public TA-Lib-compatible indicator implementation.
    text = path.read_text(encoding="utf-8")
    for name in ("fn_boll", "fn_bolldn", "fn_bollwidth"):
        start, end = _function_span(text, f"fn {name}(")
        segment = text[start:end]
        segment = segment.replace(
            "lib_stat::rolling_std_dev(values, n)",
            "crate::indicators::std_dev(values, n, 1.0)",
        )
        text = text[:start] + segment + text[end:]
    path.write_text(text, encoding="utf-8")

    _replace_function(
        path,
        "fn fn_atr(",
        r'''fn fn_atr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ATR", args, 4)?;
    let n = extract_n(args, 3, "ATR")?;
    let data_len = ctx.data_len;
    match crate::indicators::atr(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        n,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )
    _replace_function(
        path,
        "fn fn_natr(",
        r'''fn fn_natr(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("NATR", args, 4)?;
    let n = extract_n(args, 3, "NATR")?;
    let data_len = ctx.data_len;
    match crate::indicators::natr(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        n,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )
    _replace_function(
        path,
        "fn fn_trange(",
        r'''fn fn_trange(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("TRANGE", args, 3)?;
    let data_len = ctx.data_len;
    match crate::indicators::trange(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )
    _replace_function(
        path,
        "fn fn_adosc(",
        r'''fn fn_adosc(ctx: &FormulaContext, args: &[Array1<f64>]) -> Result<Array1<f64>, FormulaError> {
    ensure_args_len("ADOSC", args, 4)?;
    let fast = if args.len() > 4 && !args[4].is_empty() && !args[4][0].is_nan() {
        args[4][0] as usize
    } else {
        3
    };
    let slow = if args.len() > 5 && !args[5].is_empty() && !args[5][0].is_nan() {
        args[5][0] as usize
    } else {
        10
    };
    let data_len = ctx.data_len;
    match crate::indicators::adosc(
        args[0].as_slice().unwrap(),
        args[1].as_slice().unwrap(),
        args[2].as_slice().unwrap(),
        args[3].as_slice().unwrap(),
        fast,
        slow,
    ) {
        Ok(result) => Ok(result),
        Err(_) => Ok(nan_vec(data_len)),
    }
}''',
    )


def main() -> int:
    fix_trange()
    fix_adosc_warmup()
    fix_kama_warmup()
    fix_macd_warmup()
    fix_sar()
    fix_formula_delegation()
    print("TA-Lib semantic alignment fixes applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
