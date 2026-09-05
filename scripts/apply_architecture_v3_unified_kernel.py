#!/usr/bin/env python3
"""Apply the Architecture v3 unified-kernel build transformation.

The transformation is intentionally idempotent and keeps public Python
signatures unchanged while removing runtime string operation dispatch from the
private native hot-path ABI. It also makes the registry SSOT generator emit
NumPy-direct numeric bindings, improves rolling-extrema allocation behavior,
routes TRANGE directly into caller-owned output, and collapses MFI/volume
indicators onto their single canonical math kernels.
"""

from __future__ import annotations

from pathlib import Path

from apply_talib_performance_plan import patch_sync_bindings

ROOT = Path(__file__).resolve().parents[1]
NATIVE = ROOT / "ffi" / "python-binding" / "src" / "native_fast_path.rs"
INIT = ROOT / "ffi" / "python-binding" / "finkit" / "__init__.py"
MOMENTUM = ROOT / "core" / "src" / "indicators" / "momentum.rs"
VOLUME = ROOT / "core" / "src" / "indicators" / "volume.rs"


def _replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected exactly one source fragment, found {count}")
    return text.replace(old, new, 1)


def _replace_function(text: str, name: str, replacements: list[tuple[str, str]]) -> str:
    marker = f"fn {name}"
    start = text.find(marker)
    if start < 0:
        raise RuntimeError(f"{name}: function not found")
    next_attr = text.find("\n#[pyfunction", start + len(marker))
    end = len(text) if next_attr < 0 else next_attr
    segment = text[start:end]
    for old, new in replacements:
        if new in segment:
            continue
        count = segment.count(old)
        if count != 1:
            raise RuntimeError(
                f"{name}: expected one {old!r} fragment, found {count}"
            )
        segment = segment.replace(old, new, 1)
    return text[:start] + segment + text[end:]


def _replace_rust_public_function(
    text: str,
    start_token: str,
    next_doc: str,
    canonical: str,
    label: str,
) -> str:
    start = text.find(start_token)
    if start < 0:
        raise RuntimeError(f"{label}: function not found")
    if text.find(start_token, start + len(start_token)) >= 0:
        raise RuntimeError(f"{label}: multiple functions found")
    end_token = f"\n}}\n\n{next_doc}"
    end = text.find(end_token, start)
    if end < 0:
        raise RuntimeError(f"{label}: function boundary not found")
    current = text[start : end + 2]
    if current == canonical:
        return text
    return text[:start] + canonical + text[end + 2 :]


def patch_native_fast_path() -> None:
    text = NATIVE.read_text(encoding="utf-8")

    old_extrema = '''fn rolling_extrema_map<F>(
    max_source: &[f64],
    min_source: &[f64],
    period: usize,
    mut map: F,
) -> Vec<f64>
where
    F: FnMut(usize, f64, f64) -> f64,
{
    let len = max_source.len();
    let mut output = vec![f64::NAN; len];
    if period == 0 || period > len {
        return output;
    }

    unsafe {
        let max_ptr = max_source.as_ptr();
        let min_ptr = min_source.as_ptr();
        let output_ptr = output.as_mut_ptr();

        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *max_ptr;
        let mut lowest = *min_ptr;
        for index in 1..period {
            let high = *max_ptr.add(index);
            let low = *min_ptr.add(index);
            if high >= highest {
                highest = high;
                highest_idx = index;
            }
            if low <= lowest {
                lowest = low;
                lowest_idx = index;
            }
        }
        *output_ptr.add(period - 1) = map(period - 1, highest, lowest);

        for index in period..len {
            let window_start = index + 1 - period;
            let new_high = *max_ptr.add(index);
            let new_low = *min_ptr.add(index);

            if highest_idx < window_start {
                highest = *max_ptr.add(window_start);
                highest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *max_ptr.add(candidate);
                    if value >= highest {
                        highest = value;
                        highest_idx = candidate;
                    }
                }
            } else if new_high >= highest {
                highest = new_high;
                highest_idx = index;
            }

            if lowest_idx < window_start {
                lowest = *min_ptr.add(window_start);
                lowest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *min_ptr.add(candidate);
                    if value <= lowest {
                        lowest = value;
                        lowest_idx = candidate;
                    }
                }
            } else if new_low <= lowest {
                lowest = new_low;
                lowest_idx = index;
            }

            *output_ptr.add(index) = map(index, highest, lowest);
        }
    }
    output
}
'''
    new_extrema = '''fn rolling_extrema_map<F>(
    max_source: &[f64],
    min_source: &[f64],
    period: usize,
    mut map: F,
) -> Vec<f64>
where
    F: FnMut(usize, f64, f64) -> f64,
{
    let len = max_source.len();
    if period == 0 || period > len {
        return vec![f64::NAN; len];
    }

    // Every element is written exactly once: only the short warm-up prefix is
    // initialized to NaN and the valid range is written directly. Avoiding a
    // full vec![NaN; len] pass matters for 1M-row MIDPOINT/MIDPRICE/WILLR.
    let mut raw = Vec::<MaybeUninit<f64>>::with_capacity(len);
    unsafe {
        raw.set_len(len);
        let max_ptr = max_source.as_ptr();
        let min_ptr = min_source.as_ptr();
        let output_ptr = raw.as_mut_ptr();
        for index in 0..period - 1 {
            output_ptr.add(index).write(MaybeUninit::new(f64::NAN));
        }

        let mut highest_idx = 0usize;
        let mut lowest_idx = 0usize;
        let mut highest = *max_ptr;
        let mut lowest = *min_ptr;
        for index in 1..period {
            let high = *max_ptr.add(index);
            let low = *min_ptr.add(index);
            if high >= highest {
                highest = high;
                highest_idx = index;
            }
            if low <= lowest {
                lowest = low;
                lowest_idx = index;
            }
        }
        output_ptr
            .add(period - 1)
            .write(MaybeUninit::new(map(period - 1, highest, lowest)));

        for index in period..len {
            let window_start = index + 1 - period;
            let new_high = *max_ptr.add(index);
            let new_low = *min_ptr.add(index);

            if highest_idx < window_start {
                highest = *max_ptr.add(window_start);
                highest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *max_ptr.add(candidate);
                    if value >= highest {
                        highest = value;
                        highest_idx = candidate;
                    }
                }
            } else if new_high >= highest {
                highest = new_high;
                highest_idx = index;
            }

            if lowest_idx < window_start {
                lowest = *min_ptr.add(window_start);
                lowest_idx = window_start;
                for candidate in window_start + 1..=index {
                    let value = *min_ptr.add(candidate);
                    if value <= lowest {
                        lowest = value;
                        lowest_idx = candidate;
                    }
                }
            } else if new_low <= lowest {
                lowest = new_low;
                lowest_idx = index;
            }

            output_ptr
                .add(index)
                .write(MaybeUninit::new(map(index, highest, lowest)));
        }

        let ptr = raw.as_mut_ptr().cast::<f64>();
        let capacity = raw.capacity();
        let length = raw.len();
        forget(raw);
        Vec::from_raw_parts(ptr, length, capacity)
    }
}
'''
    text = _replace_once(text, old_extrema, new_extrema, "rolling extrema full-write")

    # Private hot-path ABI: operation strings never cross the Python/Rust
    # runtime boundary. Stable numeric ids are resolved by the public Python
    # facade and matched directly in Rust.
    text = _replace_function(
        text,
        "fast_unary_period",
        [
            ("operation: &str,", "operation: u16,"),
            ('"midpoint" =>', "1 =>"),
            ('"mom" =>', "2 =>"),
            ('"dema" =>', "3 =>"),
            ('"tema" =>', "4 =>"),
            ('"rsi" =>', "5 =>"),
            ('"roc" =>', "6 =>"),
            ('"cmo" =>', "7 =>"),
            ("unsupported fast operation {operation}", "unsupported fast operation id {operation}"),
        ],
    )
    text = _replace_function(
        text,
        "fast_unary_period_scale",
        [
            ("operation: &str,", "operation: u16,"),
            ('"stddev" =>', "1 =>"),
            ('"var" =>', "2 =>"),
            ("unsupported fast operation {operation}", "unsupported fast operation id {operation}"),
        ],
    )
    text = _replace_function(
        text,
        "fast_binary_period",
        [
            ("operation: &str,", "operation: u16,"),
            ('"midprice" =>', "1 =>"),
            ('"correl" =>', "2 =>"),
            ("unsupported fast operation {operation}", "unsupported fast operation id {operation}"),
        ],
    )
    text = _replace_function(
        text,
        "fast_hlc_period",
        [
            ("operation: &str,", "operation: u16,"),
            ('"willr" =>', "1 =>"),
            ('"adx" =>', "2 =>"),
            ('"cci" =>', "3 =>"),
            ('"plus_di" =>', "4 =>"),
            ('"minus_di" =>', "5 =>"),
            ('"atr" =>', "6 =>"),
            ('"natr" =>', "7 =>"),
            ("unsupported fast operation {operation}", "unsupported fast operation id {operation}"),
        ],
    )

    old_trange = '''fn fast_trange<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    let output = py
        .detach(|| indicators::trange(high, low, close))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output.into_raw_vec()))
}
'''
    new_trange = '''fn fast_trange<'py>(
    py: Python<'py>,
    high: PyReadonlyArray1<'py, f64>,
    low: PyReadonlyArray1<'py, f64>,
    close: PyReadonlyArray1<'py, f64>,
) -> PyResult<Bound<'py, PyArray1<f64>>> {
    let high = high.as_slice().map_err(value_error)?;
    let low = low.as_slice().map_err(value_error)?;
    let close = close.as_slice().map_err(value_error)?;
    validate_same_len(high.len(), low.len())?;
    validate_same_len(high.len(), close.len())?;
    let mut output = vec![0.0; high.len()];
    py.detach(|| indicators::trange_into(high, low, close, &mut output))
        .map_err(value_error)?;
    Ok(PyArray1::from_vec(py, output))
}
'''
    text = _replace_once(text, old_trange, new_trange, "TRANGE direct output")
    NATIVE.write_text(text, encoding="utf-8")


def patch_public_mfi() -> None:
    """Collapse the public indicator implementation onto the canonical MFI kernel."""

    text = MOMENTUM.read_text(encoding="utf-8")
    canonical = '''pub fn mfi(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    period: usize,
) -> Result<Array1<f64>> {
    crate::math::mfi::mfi(high, low, close, volume, period)
}'''
    text = _replace_rust_public_function(
        text,
        "pub fn mfi(\n",
        "/// Minus Directional Indicator",
        canonical,
        "momentum.rs public MFI",
    )

    start = text.find("pub fn mfi(\n")
    section_end = text.find("/// Minus Directional Indicator", start)
    section = text[start:section_end]
    if "simd_typical_price" in section or "pos_ring" in section or "neg_ring" in section:
        raise RuntimeError("momentum.rs: duplicate public MFI implementation remains")
    if "crate::math::mfi::mfi(high, low, close, volume, period)" not in section:
        raise RuntimeError("momentum.rs: public MFI is not routed to canonical kernel")
    MOMENTUM.write_text(text, encoding="utf-8")


def patch_public_volume_kernels() -> None:
    """Route AD/ADOSC/OBV wrappers through the canonical one-pass kernels."""

    text = VOLUME.read_text(encoding="utf-8")

    ad = '''pub fn ad(high: &[f64], low: &[f64], close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;

    let mut output = Array1::<f64>::zeros(high.len());
    crate::math::volume_kernels::ad_into(
        high,
        low,
        close,
        volume,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}'''
    text = _replace_rust_public_function(
        text,
        "pub fn ad(high:",
        "/// AD zero-copy variant",
        ad,
        "volume.rs public AD",
    )

    ad_into = '''pub fn ad_into(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    output: &mut [f64],
) -> Result<()> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), 1)?;
    if output.len() != high.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "output".to_string(),
            constraint: "must have the same length as high".to_string(),
        });
    }
    crate::math::volume_kernels::ad_into(high, low, close, volume, output)
}'''
    text = _replace_rust_public_function(
        text,
        "pub fn ad_into(\n",
        "/// Chaikin A/D Oscillator",
        ad_into,
        "volume.rs public AD into",
    )

    adosc = '''pub fn adosc(
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    fast_period: usize,
    slow_period: usize,
) -> Result<Array1<f64>> {
    if high.len() != low.len() || high.len() != close.len() || high.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "high, low, close, volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(high.len(), slow_period)?;

    let mut output = Array1::<f64>::zeros(high.len());
    crate::math::volume_kernels::adosc_into(
        high,
        low,
        close,
        volume,
        fast_period,
        slow_period,
        output.as_slice_mut().expect("owned Array1 is contiguous"),
    )?;
    Ok(output)
}'''
    text = _replace_rust_public_function(
        text,
        "pub fn adosc(\n",
        "/// On Balance Volume (OBV)",
        adosc,
        "volume.rs public ADOSC",
    )

    obv = '''pub fn obv(close: &[f64], volume: &[f64]) -> Result<Array1<f64>> {
    if close.len() != volume.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "close and volume".to_string(),
            constraint: "must have the same length".to_string(),
        });
    }
    validate_input(close.len(), 1)?;

    let mut output = vec![0.0_f64; close.len()];
    crate::math::volume_kernels::obv_into(close, volume, &mut output)?;
    Ok(Array1::from_vec(output))
}'''
    text = _replace_rust_public_function(
        text,
        "pub fn obv(close:",
        "/// Volume Profile 结果结构体",
        obv,
        "volume.rs public OBV",
    )

    ad_section = text[text.find("pub fn ad(high:"):text.find("/// AD zero-copy variant")]
    adosc_section = text[text.find("pub fn adosc(\n"):text.find("/// On Balance Volume (OBV)")]
    obv_section = text[text.find("pub fn obv(close:"):text.find("/// Volume Profile 结果结构体")]
    if "simd_ad_line" in ad_section or "simd_ad_line" in adosc_section:
        raise RuntimeError("volume.rs: legacy SIMD AD scratch path remains public")
    if "cumulative = vec!" in adosc_section:
        raise RuntimeError("volume.rs: ADOSC cumulative scratch allocation remains")
    if "simd_obv" in obv_section:
        raise RuntimeError("volume.rs: OBV delta-vector path remains public")
    VOLUME.write_text(text, encoding="utf-8")


def patch_python_facade() -> None:
    text = INIT.read_text(encoding="utf-8")
    replacements = {
        '_unary_period("midpoint",': "_unary_period(1,",
        '_unary_period("mom",': "_unary_period(2,",
        '_unary_period("dema",': "_unary_period(3,",
        '_unary_period("tema",': "_unary_period(4,",
        '_unary_period("rsi",': "_unary_period(5,",
        '_unary_period("roc",': "_unary_period(6,",
        '_unary_period("cmo",': "_unary_period(7,",
        '_native._fast_unary_period_scale("stddev",': "_native._fast_unary_period_scale(1,",
        '_native._fast_unary_period_scale("var",': "_native._fast_unary_period_scale(2,",
        '_native._fast_binary_period("midprice",': "_native._fast_binary_period(1,",
        '_native._fast_binary_period("correl",': "_native._fast_binary_period(2,",
        '_hlc_period("willr",': "_hlc_period(1,",
        '_hlc_period("adx",': "_hlc_period(2,",
        '_hlc_period("cci",': "_hlc_period(3,",
        '_hlc_period("plus_di",': "_hlc_period(4,",
        '_hlc_period("minus_di",': "_hlc_period(5,",
        '_hlc_period("atr",': "_hlc_period(6,",
        '_hlc_period("natr",': "_hlc_period(7,",
    }
    for old, new in replacements.items():
        if new in text:
            continue
        count = text.count(old)
        if count != 1:
            raise RuntimeError(f"Python facade {old!r}: expected one occurrence, found {count}")
        text = text.replace(old, new, 1)
    INIT.write_text(text, encoding="utf-8")


def main() -> int:
    patch_native_fast_path()
    patch_public_mfi()
    patch_public_volume_kernels()
    patch_python_facade()
    # Move NumPy-direct conversion into the live SSOT generator. This is the
    # canonical generator-level fix; optimize_python_bindings.py remains as the
    # implementation helper and CI drift checker, not a required manual patch.
    patch_sync_bindings()
    print("Architecture v3 unified-kernel migration applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
