#!/usr/bin/env python3
"""Apply formula-runtime and batch API performance fixes from the TA-Lib plan."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _span(text: str, needle: str) -> tuple[int, int]:
    start = text.find(needle)
    if start < 0:
        raise RuntimeError(f"marker not found: {needle}")
    brace = text.find("{", start)
    if brace < 0:
        raise RuntimeError(f"brace not found: {needle}")
    depth = 0
    quote: str | None = None
    escaped = False
    for i in range(brace, len(text)):
        ch = text[i]
        if quote is not None:
            if escaped:
                escaped = False
            elif ch == "\\":
                escaped = True
            elif ch == quote:
                quote = None
            continue
        if ch in ('"', "'"):
            quote = ch
        elif ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return start, i + 1
    raise RuntimeError(f"unbalanced function: {needle}")


def _replace_function(path: Path, needle: str, replacement: str) -> None:
    text = path.read_text(encoding="utf-8")
    if replacement.strip() in text:
        return
    start, end = _span(text, needle)
    path.write_text(text[:start] + replacement.rstrip() + text[end:], encoding="utf-8")


def patch_engine() -> None:
    path = ROOT / "core/src/formula/engine.rs"
    text = path.read_text(encoding="utf-8")

    marker = "    fn try_execute_simple_formula_slices(\n"
    method = r'''    /// Evaluate a range while borrowing the caller's OHLCV buffers.
    ///
    /// Only the dependency window selected by `eval_range` is materialised;
    /// the full NumPy-backed history is never copied at the language boundary.
    pub fn eval_range_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        start: usize,
        end: usize,
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
            || start > end
            || end > close.len()
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy range inputs must be non-empty/equal-length and satisfy 0 <= start <= end <= len"
                    .to_string(),
            ));
        }

        let context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.eval_range(formula, &context, start, end)
    }

'''
    if "pub fn eval_range_zero_copy_inputs(" not in text:
        if marker not in text:
            raise RuntimeError("engine insertion marker missing")
        text = text.replace(marker, method + marker, 1)
        path.write_text(text, encoding="utf-8")

    replacement = r'''    fn try_execute_simple_formula_slices(
        &self,
        ast: &AstNode,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> Option<Array1<f64>> {
        fn series<'a>(
            node: &AstNode,
            open: &'a [f64],
            high: &'a [f64],
            low: &'a [f64],
            close: &'a [f64],
            volume: &'a [f64],
        ) -> Option<&'a [f64]> {
            let AstNode::Variable(name) = node else {
                return None;
            };
            if name.eq_ignore_ascii_case("C") || name.eq_ignore_ascii_case("CLOSE") {
                Some(close)
            } else if name.eq_ignore_ascii_case("O") || name.eq_ignore_ascii_case("OPEN") {
                Some(open)
            } else if name.eq_ignore_ascii_case("H") || name.eq_ignore_ascii_case("HIGH") {
                Some(high)
            } else if name.eq_ignore_ascii_case("L") || name.eq_ignore_ascii_case("LOW") {
                Some(low)
            } else if name.eq_ignore_ascii_case("V")
                || name.eq_ignore_ascii_case("VOL")
                || name.eq_ignore_ascii_case("VOLUME")
            {
                Some(volume)
            } else {
                None
            }
        }

        fn period(node: &AstNode) -> Option<usize> {
            let AstNode::Number(value) = node else {
                return None;
            };
            if !value.is_finite() || *value <= 0.0 {
                return None;
            }
            let n = *value as usize;
            if n == 0 || (n as f64 - *value).abs() > f64::EPSILON {
                None
            } else {
                Some(n)
            }
        }

        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } => (name.to_ascii_uppercase(), args.as_slice()),
            _ => return None,
        };
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
        {
            return None;
        }
        let len = close.len();

        match name.as_str() {
            "TRANGE" if args.len() >= 3 => {
                let h = series(&args[0], open, high, low, close, volume)?;
                let l = series(&args[1], open, high, low, close, volume)?;
                let c = series(&args[2], open, high, low, close, volume)?;
                crate::indicators::trange(h, l, c).ok()
            }
            "ATR" | "NATR" if args.len() >= 4 => {
                let h = series(&args[0], open, high, low, close, volume)?;
                let l = series(&args[1], open, high, low, close, volume)?;
                let c = series(&args[2], open, high, low, close, volume)?;
                let n = period(&args[3])?;
                if name == "ATR" {
                    crate::indicators::atr(h, l, c, n).ok()
                } else {
                    crate::indicators::natr(h, l, c, n).ok()
                }
            }
            "AD" if args.len() >= 4 => {
                let h = series(&args[0], open, high, low, close, volume)?;
                let l = series(&args[1], open, high, low, close, volume)?;
                let c = series(&args[2], open, high, low, close, volume)?;
                let v = series(&args[3], open, high, low, close, volume)?;
                crate::indicators::ad(h, l, c, v).ok()
            }
            "OBV" if args.len() >= 2 => {
                let c = series(&args[0], open, high, low, close, volume)?;
                let v = series(&args[1], open, high, low, close, volume)?;
                crate::indicators::obv(c, v).ok()
            }
            "ADOSC" if args.len() >= 4 => {
                let h = series(&args[0], open, high, low, close, volume)?;
                let l = series(&args[1], open, high, low, close, volume)?;
                let c = series(&args[2], open, high, low, close, volume)?;
                let v = series(&args[3], open, high, low, close, volume)?;
                let fast = args.get(4).and_then(period).unwrap_or(3);
                let slow = args.get(5).and_then(period).unwrap_or(10);
                crate::indicators::adosc(h, l, c, v, fast, slow).ok()
            }
            "MACD" if args.len() >= 2 => {
                let input = series(&args[0], open, high, low, close, volume)?;
                let fast = period(&args[1])?;
                let slow = args.get(2).and_then(period).unwrap_or(26);
                let signal = args.get(3).and_then(period).unwrap_or(9);
                crate::indicators::macd(input, fast, slow, signal)
                    .ok()
                    .map(|value| value.macd)
            }
            _ => {
                if args.len() < 2 {
                    return None;
                }
                let input = series(&args[0], open, high, low, close, volume)?;
                if input.iter().any(|value| !value.is_finite()) {
                    return Some(Array1::from_elem(len, f64::NAN));
                }
                let n = period(&args[1])?;
                match name.as_str() {
                    "MA" | "BOLLMID" => {
                        let mut output = Array1::from_elem(len, f64::NAN);
                        crate::math::simd_kernels::sma_simd_into(
                            input,
                            n,
                            output.as_slice_mut().expect("Array1 is contiguous"),
                        );
                        Some(output)
                    }
                    "EMA" => {
                        let mut output = Array1::from_elem(len, f64::NAN);
                        crate::math::simd_kernels::ema_simd_into(
                            input,
                            n,
                            output.as_slice_mut().expect("Array1 is contiguous"),
                        );
                        Some(output)
                    }
                    "RSI" => {
                        let mut output = Array1::from_elem(len, f64::NAN);
                        crate::math::simd_kernels::rsi_simd_into(
                            input,
                            n,
                            output.as_slice_mut().expect("Array1 is contiguous"),
                        );
                        Some(output)
                    }
                    "ROC" => crate::indicators::roc(input, n).ok(),
                    "STD" => crate::indicators::std_dev(input, n, 1.0).ok(),
                    "VAR" => crate::indicators::var(input, n, 1.0).ok(),
                    "REF" => {
                        let mut output = Array1::from_elem(len, f64::NAN);
                        for i in n..len {
                            output[i] = input[i - n];
                        }
                        Some(output)
                    }
                    "BOLL" | "BOLLUP" | "BOLLDN" => {
                        let nbdev = args
                            .get(2)
                            .and_then(|node| match node {
                                AstNode::Number(value) if value.is_finite() => Some(*value),
                                _ => None,
                            })
                            .unwrap_or(2.0);
                        crate::indicators::bbands(input, n, nbdev, nbdev)
                            .ok()
                            .map(|bands| if name == "BOLLDN" { bands.lower } else { bands.upper })
                    }
                    _ => None,
                }
            }
        }
    }'''
    _replace_function(path, "    fn try_execute_simple_formula_slices(", replacement)


def patch_formula_plan() -> None:
    path = ROOT / "ffi/python-binding/src/formula_plan.rs"
    text = path.read_text(encoding="utf-8")
    if "fn eval_range_zero_copy<'py>(" in text:
        return
    marker = "    /// Evaluate the last bar. With no arrays this reuses the context retained\n"
    method = r'''    /// Evaluate a half-open range while borrowing contiguous NumPy OHLCV.
    ///
    /// Unlike `eval_range`, this method intentionally does not retain an owned
    /// streaming context after the call.  Use `eval` before append/eval_last
    /// workflows that require retained history.
    #[pyo3(signature = (open, high, low, close, volume, start, end, amount=None))]
    #[allow(clippy::too_many_arguments)]
    fn eval_range_zero_copy<'py>(
        &mut self,
        py: Python<'py>,
        open: PyReadonlyArray1<'py, f64>,
        high: PyReadonlyArray1<'py, f64>,
        low: PyReadonlyArray1<'py, f64>,
        close: PyReadonlyArray1<'py, f64>,
        volume: PyReadonlyArray1<'py, f64>,
        start: usize,
        end: usize,
        amount: Option<PyReadonlyArray1<'py, f64>>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let open = open.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("open must be contiguous float64: {error}"))
        })?;
        let high = high.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("high must be contiguous float64: {error}"))
        })?;
        let low = low.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("low must be contiguous float64: {error}"))
        })?;
        let close = close.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("close must be contiguous float64: {error}"))
        })?;
        let volume = volume.as_slice().map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("volume must be contiguous float64: {error}"))
        })?;
        let amount = amount
            .as_ref()
            .map(|array| array.as_slice().map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!("amount must be contiguous float64: {error}"))
            }))
            .transpose()?;
        let data_len = validate_lengths(open, high, low, close, volume, amount)?;
        if start > end || end > data_len {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "eval_range_zero_copy expects 0 <= start <= end <= input length",
            ));
        }

        let engine = self.engine.take().ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "compiled formula is already being evaluated",
            )
        })?;
        let execution = engine.eval_range_zero_copy_inputs(
            &self.compiled,
            open,
            high,
            low,
            close,
            volume,
            start,
            end,
            amount,
        );
        self.engine = Some(engine);
        let result = execution.map_err(formula_runtime_error)?;
        let output = PyDict::new(py);
        output.set_item("__result__", PyArray1::from_vec(py, result.into_raw_vec()))?;
        Ok(output)
    }

'''
    if marker not in text:
        raise RuntimeError("formula_plan insertion marker missing")
    path.write_text(text.replace(marker, method + marker, 1), encoding="utf-8")


def patch_python_api() -> None:
    init_path = ROOT / "ffi/python-binding/finkit/__init__.py"
    text = init_path.read_text(encoding="utf-8")
    old = '_PUBLIC_ALIASES = {"stddev": "std_dev", "correl": "correlation"}'
    new = '_PUBLIC_ALIASES = {"stddev": "std_dev", "correl": "correlation", "compute_many": "compute_indicators"}'
    if old in text:
        text = text.replace(old, new, 1)
    elif new not in text:
        raise RuntimeError("public alias block missing; run binding migration first")
    init_path.write_text(text, encoding="utf-8")

    stub = ROOT / "ffi/python-binding/finkit/__init__.pyi"
    st = stub.read_text(encoding="utf-8")
    formula_marker = "# ============================================================================\n# Formula Engine\n# ============================================================================\n"
    batch_stubs = '''# ============================================================================
# Batch Computation
# ============================================================================

def compute_indicators(
    close: Array1D,
    requests: List[Tuple[str, List[float]]],
    open: Optional[Array1D] = ...,
    high: Optional[Array1D] = ...,
    low: Optional[Array1D] = ...,
    volume: Optional[Array1D] = ...,
    secondary: Optional[Array1D] = ...,
) -> Dict[str, Union[Array1D, str]]:
    """Compute multiple indicators with one native boundary crossing."""
    ...

def compute_many(
    close: Array1D,
    requests: List[Tuple[str, List[float]]],
    open: Optional[Array1D] = ...,
    high: Optional[Array1D] = ...,
    low: Optional[Array1D] = ...,
    volume: Optional[Array1D] = ...,
    secondary: Optional[Array1D] = ...,
) -> Dict[str, Union[Array1D, str]]:
    """Alias for compute_indicators optimized for factor/feature batches."""
    ...

'''
    if "def compute_many(" not in st:
        if formula_marker not in st:
            raise RuntimeError("stub formula marker missing")
        st = st.replace(formula_marker, batch_stubs + formula_marker, 1)

    eval_signature = '''    def eval(
        self,
        open: ArrayLike,
        high: ArrayLike,
        low: ArrayLike,
        close: ArrayLike,
        volume: ArrayLike,
        amount: Optional[ArrayLike] = ...,
    ) -> Dict[str, Array1D]:
        """Evaluate the compiled formula and return NumPy arrays."""
        ...
'''
    expanded = eval_signature + '''
    def eval_zero_copy(
        self,
        open: Array1D,
        high: Array1D,
        low: Array1D,
        close: Array1D,
        volume: Array1D,
        amount: Optional[Array1D] = ...,
    ) -> Dict[str, Array1D]:
        """Borrow contiguous float64 NumPy inputs for synchronous evaluation."""
        ...

    def eval_range(
        self,
        open: ArrayLike,
        high: ArrayLike,
        low: ArrayLike,
        close: ArrayLike,
        volume: ArrayLike,
        start: int,
        end: int,
        amount: Optional[ArrayLike] = ...,
    ) -> Dict[str, Array1D]:
        ...

    def eval_range_zero_copy(
        self,
        open: Array1D,
        high: Array1D,
        low: Array1D,
        close: Array1D,
        volume: Array1D,
        start: int,
        end: int,
        amount: Optional[Array1D] = ...,
    ) -> Dict[str, Array1D]:
        ...

    def eval_last(self) -> float:
        ...

    def append_bar(self, open: float, high: float, low: float, close: float, volume: float) -> None:
        ...

    def reserve_bars(self, additional: int) -> None:
        ...

    def reset(self) -> None:
        ...
'''
    if "def eval_range_zero_copy(" not in st:
        if eval_signature not in st:
            raise RuntimeError("CompiledFormula eval stub block missing")
        st = st.replace(eval_signature, expanded, 1)

    if '"compute_many"' not in st:
        st = st.replace(
            '    # Formula Engine\n    "CompiledFormula", "formula_eval", "formula_eval_dialect",',
            '    # Batch Computation\n    "compute_indicators", "compute_many",\n    # Formula Engine\n    "CompiledFormula", "formula_eval", "formula_eval_dialect",',
            1,
        )
    stub.write_text(st, encoding="utf-8")


def main() -> int:
    patch_engine()
    patch_formula_plan()
    patch_python_api()
    print("formula runtime performance fixes applied")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
