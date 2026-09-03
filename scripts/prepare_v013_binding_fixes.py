#!/usr/bin/env python3
"""One-shot v0.1.3 source cleanup for warnings observed in the real release build.

This script is intentionally strict and idempotent: every replacement either applies
once or verifies that the canonical form is already present. It is removed after the
warning-clean sources have been materialized and validated.
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    if old in text:
        path.write_text(text.replace(old, new, 1), encoding="utf-8")
        return
    if new not in text:
        raise RuntimeError(f"{label} anchor not found: {path}")


def main() -> None:
    core_registry = ROOT / "core/src/registry.rs"
    replace_exact(
        core_registry,
        'const PERIOD_20: &[ParamSpec] = &[ParamSpec::new("period", "usize", Some("20"), Some("> 0"))];\n',
        "",
        "unused PERIOD_20",
    )

    c_lib = ROOT / "ffi/c-binding/src/lib.rs"
    replace_exact(c_lib, "use std::ffi::{CStr, CString};\n", "use std::ffi::CString;\n", "CStr import")
    replace_exact(
        c_lib,
        '''/// Best-effort calculation error handler for non-`TaError` error types
/// (e.g. `VisualizationError`). Records the formatted message and
/// returns the generic legacy code.
fn calc_error_display(err: impl std::fmt::Display) -> i32 {
    set_last_error(err);
    set_last_error_code(TA_ERR_CALCULATION);
    TA_ERR_CALCULATION
}

''',
        "",
        "dead calc_error_display",
    )

    # Rust 2024 requires explicit unsafe blocks even inside unsafe functions.
    text = c_lib.read_text(encoding="utf-8")
    old_slice = "    let dst_slice = std::slice::from_raw_parts_mut(dst, copy_len);\n"
    new_slice = "    let dst_slice = unsafe { std::slice::from_raw_parts_mut(dst, copy_len) };\n"
    count = text.count(old_slice)
    if count:
        text = text.replace(old_slice, new_slice)
        c_lib.write_text(text, encoding="utf-8")
    elif c_lib.read_text(encoding="utf-8").count(new_slice) < 2:
        raise RuntimeError("copy_result unsafe anchors not found")

    replace_exact(
        c_lib,
        '''fn ffi_catch_i64<F>(f: F) -> i64
where
    F: FnOnce() -> i64,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            set_last_error("internal error: panic at FFI boundary");
            set_last_error_code(FfiStatus::InternalError.as_i32());
            0
        }
    }
}

''',
        "",
        "dead ffi_catch_i64",
    )
    replace_exact(c_lib, "    ffi_catch_ptr(|| unsafe {\n", "    ffi_catch_ptr(|| {\n", "unnecessary ta_version unsafe")
    replace_exact(
        c_lib,
        "            drop(CString::from_raw(s));\n",
        "            drop(unsafe { CString::from_raw(s) });\n",
        "CString::from_raw unsafe block",
    )
    replace_exact(c_lib, '#[no_mangle]\n\ninclude!("generated.rs");\n', 'include!("generated.rs");\n', "no_mangle macro attribute")
    replace_exact(c_lib, "mod tests {\n    use super::*;\n", "mod tests {\n    use super::*;\n    use std::ffi::CStr;\n", "test CStr import")

    # generated.rs is the file that actually contains the deprecated call seen in
    # the release log. The registry JSON does not contain this body, so do not
    # manufacture an SSOT edit that is not present in the repository.
    generated = ROOT / "ffi/c-binding/src/generated.rs"
    replace_exact(
        generated,
        "indicators::linear_reg(data, period as usize)",
        "indicators::linearreg(data, period as usize)",
        "deprecated linear_reg call",
    )

    chart = ROOT / "ffi/java-binding/java/src/main/java/com/finkit/ChartPatterns.java"
    text = chart.read_text(encoding="utf-8")
    text = text.replace("Head and Shoulders Bottom (Inverse H&S) detection.",
                        "Head and Shoulders Bottom (inverse head-and-shoulders) detection.")
    text = text.replace("Head and Shoulders Bottom (Inverse Head and Shoulders) detection.",
                        "Head and Shoulders Bottom (inverse head-and-shoulders) detection.")
    chart.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
