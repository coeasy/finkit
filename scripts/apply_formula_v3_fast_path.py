#!/usr/bin/env python3
"""Compile common Formula ROC/MOM calls into canonical numeric kernels.

The installed-wheel v3 benchmark showed ROC10 spending most of its time in the
generic Formula executor. PyCompiledFormula already parses canonical formulas
once in its constructor; extend that compile-time classification to ROC/MOM so
repeated evals never parse/uppercase operation strings and execute the same
canonical indicator kernels as the public API.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "ffi" / "python-binding" / "src" / "formula_plan.rs"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if new in text:
        return text
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"{label}: expected one source fragment, found {count}")
    return text.replace(old, new, 1)


def main() -> int:
    text = PATH.read_text(encoding="utf-8")
    text = replace_once(
        text,
        """enum CanonicalFormula {\n    Atr { period: usize },\n    Std { period: usize },\n    Boll { period: usize, nbdev: f64 },\n}""",
        """enum CanonicalFormula {\n    Atr { period: usize },\n    Std { period: usize },\n    Boll { period: usize, nbdev: f64 },\n    Roc { period: usize },\n    Mom { period: usize },\n}""",
        "canonical enum",
    )
    text = replace_once(
        text,
        """        \"BOLL\" if args.len() == 3 && is_close(args[0]) => {\n            let period = args[1].parse::<usize>().ok()?;\n            let nbdev = args[2].parse::<f64>().ok()?;\n            (period > 0).then_some(CanonicalFormula::Boll { period, nbdev })\n        }\n        _ => None,""",
        """        \"BOLL\" if args.len() == 3 && is_close(args[0]) => {\n            let period = args[1].parse::<usize>().ok()?;\n            let nbdev = args[2].parse::<f64>().ok()?;\n            (period > 0).then_some(CanonicalFormula::Boll { period, nbdev })\n        }\n        \"ROC\" if args.len() == 2 && is_close(args[0]) => {\n            let period = args[1].parse::<usize>().ok()?;\n            (period > 0).then_some(CanonicalFormula::Roc { period })\n        }\n        \"MOM\" if args.len() == 2 && is_close(args[0]) => {\n            let period = args[1].parse::<usize>().ok()?;\n            (period > 0).then_some(CanonicalFormula::Mom { period })\n        }\n        _ => None,""",
        "canonical classifier",
    )
    text = replace_once(
        text,
        """        CanonicalFormula::Boll { period, nbdev } => {\n            rolling_stats::bbands_sma(close, period, nbdev, nbdev)\n                .map(|(upper, _, _)| Array1::from_vec(upper))\n                .map_err(|error| {\n                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())\n                })\n        }\n    }""",
        """        CanonicalFormula::Boll { period, nbdev } => {\n            rolling_stats::bbands_sma(close, period, nbdev, nbdev)\n                .map(|(upper, _, _)| Array1::from_vec(upper))\n                .map_err(|error| {\n                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())\n                })\n        }\n        CanonicalFormula::Roc { period } => ::finkit::indicators::roc(close, period)\n            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),\n        CanonicalFormula::Mom { period } => ::finkit::indicators::mom(close, period)\n            .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())),\n    }""",
        "canonical executor",
    )
    PATH.write_text(text, encoding="utf-8")
    print("Formula ROC/MOM canonical compile-time fast paths applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
