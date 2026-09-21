//! Formula kernel dispatcher for the Architecture v3 [`UnifiedExecutor`].
//!
//! This is the first real frontend migration onto the numeric executor. Formula
//! parsing and semantic lowering still happen once at compile time; execution of
//! constants and core numeric unary/binary operators uses only [`KernelId`],
//! physical buffer slots and prebound parameter values.

use crate::error::TaError;
use crate::execution_plan::KernelId;
use crate::formula::types::HostContext;
use crate::state_arena::StateArena;
use crate::unified_executor::{KernelCall, KernelDispatchError, KernelDispatcher, UnifiedExecutor};
use ndarray::Array1;

/// Numeric dispatcher for formula constants and core arithmetic/logical operators.
///
/// The dispatcher is no longer a unit struct: it carries the [`HostContext`]
/// that host-dependent kernels (`WINNER`, `COST`, `PERIODTYPE`) need and that
/// the numeric input slots cannot express.
#[derive(Clone, Default)]
pub struct FormulaKernelDispatcher {
    /// Host-side data (chip distribution, chart period) that the numeric input
    /// slots cannot carry. Empty unless the caller supplies it.
    host: HostContext,
}

impl FormulaKernelDispatcher {
    const ERR_UNSUPPORTED_KERNEL: u32 = 1;
    const ERR_ARITY: u32 = 2;
    const ERR_PARAMETER: u32 = 3;

    /// Create a dispatcher with no host context: host-dependent kernels such as
    /// `WINNER` evaluate to `NaN`, matching the tree path without chip data.
    pub const fn new() -> Self {
        Self {
            host: HostContext {
                chip: None,
                period_type: 0,
            },
        }
    }

    /// Create a dispatcher carrying host data for the host-dependent kernels.
    pub fn with_host(host: HostContext) -> Self {
        Self { host }
    }
}

impl KernelDispatcher for FormulaKernelDispatcher {
    fn dispatch(
        &mut self,
        call: KernelCall<'_>,
        buffers: &mut [Vec<f64>],
        _states: &mut StateArena,
    ) -> Result<(), KernelDispatchError> {
        if call.kernel == KernelId::from_static("NUMBER") {
            if !call.inputs.is_empty() {
                return Err(KernelDispatchError::new(Self::ERR_ARITY));
            }
            let value = call
                .parameters
                .first()
                .and_then(|parameter| parameter.as_f64())
                .ok_or_else(|| KernelDispatchError::new(Self::ERR_PARAMETER))?;
            buffers[call.output.0].fill(value);
            return Ok(());
        }

        if call.kernel == KernelId::from_static("INDEX") {
            return dispatch_index_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ADD") {
            return dispatch_arith_call(call, buffers, ArithKernel::Add);
        }
        if call.kernel == KernelId::from_static("CALL:SUB") {
            return dispatch_arith_call(call, buffers, ArithKernel::Sub);
        }
        if call.kernel == KernelId::from_static("CALL:MULT") {
            return dispatch_arith_call(call, buffers, ArithKernel::Mul);
        }
        if call.kernel == KernelId::from_static("CALL:DIV") {
            return dispatch_arith_call(call, buffers, ArithKernel::Div);
        }

        if call.kernel == KernelId::from_static("CALL:SQRT") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Sqrt);
        }
        if call.kernel == KernelId::from_static("CALL:SINH") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Sinh);
        }
        if call.kernel == KernelId::from_static("CALL:COSH") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Cosh);
        }
        if call.kernel == KernelId::from_static("CALL:TANH") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Tanh);
        }

        if call.kernel == KernelId::from_static("CALL:MINUS") {
            return dispatch_window_call(call, buffers, WindowKernel::Minus);
        }
        if call.kernel == KernelId::from_static("CALL:MAXINDEX") {
            return dispatch_window_call(call, buffers, WindowKernel::MaxIndex);
        }
        if call.kernel == KernelId::from_static("CALL:MININDEX") {
            return dispatch_window_call(call, buffers, WindowKernel::MinIndex);
        }
        if call.kernel == KernelId::from_static("CALL:HHVBARS") {
            return dispatch_window_call(call, buffers, WindowKernel::HhvBars);
        }
        if call.kernel == KernelId::from_static("CALL:LLVBARS") {
            return dispatch_window_call(call, buffers, WindowKernel::LlvBars);
        }

        if call.kernel == KernelId::from_static("CALL:MA")
            || call.kernel == KernelId::from_static("CALL:SMA")
            || call.kernel == KernelId::from_static("CALL:EMA")
            || call.kernel == KernelId::from_static("CALL:WMA")
            || call.kernel == KernelId::from_static("CALL:KAMA")
            || call.kernel == KernelId::from_static("CALL:RSI")
            || call.kernel == KernelId::from_static("CALL:MOM")
            || call.kernel == KernelId::from_static("CALL:ROC")
            || call.kernel == KernelId::from_static("CALL:TRIX")
            || call.kernel == KernelId::from_static("CALL:TRIMA")
            || call.kernel == KernelId::from_static("CALL:STD")
            || call.kernel == KernelId::from_static("CALL:HHV")
            || call.kernel == KernelId::from_static("CALL:LLV")
            || call.kernel == KernelId::from_static("CALL:SUM")
            || call.kernel == KernelId::from_static("CALL:REF")
            || call.kernel == KernelId::from_static("CALL:ROCP")
            || call.kernel == KernelId::from_static("CALL:ROCR")
            || call.kernel == KernelId::from_static("CALL:ROCR100")
        {
            return dispatch_periodic_call(call, buffers);
        }

        // `CALL:CCI` has two shapes: the domestic four-operand HLC form and the
        // Pine two-operand `ta.cci(source, length)` form, where the source is
        // passed through instead of being folded into a typical price.
        if call.kernel == KernelId::from_static("CALL:CCI") && call.inputs.len() == 2 {
            return dispatch_cci_source_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ATR")
            || call.kernel == KernelId::from_static("CALL:NATR")
            || call.kernel == KernelId::from_static("CALL:CCI")
        {
            return dispatch_hlc_periodic_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:BBANDS") {
            return dispatch_bbands_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:BOLLUP")
            || call.kernel == KernelId::from_static("CALL:BOLLMID")
            || call.kernel == KernelId::from_static("CALL:BOLLDN")
        {
            return dispatch_boll_band_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:MACD") {
            return dispatch_macd_line_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:DEA") {
            return dispatch_dea_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:OBV") {
            return dispatch_obv(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:AROON_UP")
            || call.kernel == KernelId::from_static("CALL:AROON_DN")
        {
            return dispatch_aroon_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:PLUS_DI")
            || call.kernel == KernelId::from_static("CALL:MINUS_DI")
            || call.kernel == KernelId::from_static("CALL:ADX")
        {
            return dispatch_dmi_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:WILLR") {
            return dispatch_willr_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:SAR") {
            return dispatch_sar_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:STOCHF") {
            return dispatch_stochf_call(call, buffers);
        }

        // `IF(COND, A, B)` and Pine's `cond ? A : B` both lower to `CALL:IF`.
        if call.kernel == KernelId::from_static("CALL:IF") {
            return dispatch_if_call(call, buffers);
        }

        // Host-context kernels. These are the only kernels that read data the
        // numeric input slots cannot carry; see `HostContext`.
        if call.kernel == KernelId::from_static("CALL:WINNER")
            || call.kernel == KernelId::from_static("CALL:COST")
        {
            return dispatch_chip_call(&self.host, call, buffers);
        }
        if call.kernel == KernelId::from_static("CALL:PERIODTYPE") {
            return dispatch_periodtype_call(&self.host, call, buffers);
        }
        // `REFDATE` needs no host data — it reads a scalar index out of its
        // second operand — but it must exist as a kernel or the plan path cannot
        // run the cross-period corpus at all.
        if call.kernel == KernelId::from_static("CALL:REFDATE") {
            return dispatch_refdate_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("DRAW:FILL")
            || call.kernel == KernelId::from_static("STICK_LINE")
            || call.kernel == KernelId::from_static("DRAW_TEXT")
            || call.kernel == KernelId::from_static("DRAW_ICON")
        {
            return dispatch_draw_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:AD")
            || call.kernel == KernelId::from_static("CALL:ADOSC")
            || call.kernel == KernelId::from_static("CALL:MFI")
        {
            return dispatch_volume_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ZSCORE")
            || call.kernel == KernelId::from_static("CALL:VWMA")
            || call.kernel == KernelId::from_static("CALL:CMF")
            || call.kernel == KernelId::from_static("CALL:FISHER")
            || call.kernel == KernelId::from_static("CALL:FISHER_SIGNAL")
            || call.kernel == KernelId::from_static("CALL:TSI")
            || call.kernel == KernelId::from_static("CALL:CHOP")
            || call.kernel == KernelId::from_static("CALL:KDJ")
            || call.kernel == KernelId::from_static("CALL:KDJ_D")
            || call.kernel == KernelId::from_static("CALL:KDJ_J")
            || call.kernel == KernelId::from_static("CALL:ICHIMOKU_TENKAN")
            || call.kernel == KernelId::from_static("CALL:ICHIMOKU_KIJUN")
            || call.kernel == KernelId::from_static("CALL:SUPERTREND")
            || call.kernel == KernelId::from_static("CALL:VWAP")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_UPPER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_LOWER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_MIDDLE")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_WIDTH")
        {
            return dispatch_modern_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("UNARY:Neg") {
            return unary(call, buffers, |value| -value);
        }
        if call.kernel == KernelId::from_static("UNARY:Not") {
            return unary(call, buffers, |value| if value <= 0.0 { 1.0 } else { 0.0 });
        }
        if call.kernel == KernelId::from_static("CALL:ABS") {
            return unary(call, buffers, f64::abs);
        }
        if call.kernel == KernelId::from_static("CALL:MATH_AVG") {
            return mean_formula_into(call, buffers);
        }

        let op = if call.kernel == KernelId::from_static("BINARY:Add") {
            BinaryKernel::Add
        } else if call.kernel == KernelId::from_static("BINARY:Sub") {
            BinaryKernel::Sub
        } else if call.kernel == KernelId::from_static("BINARY:Mul") {
            BinaryKernel::Mul
        } else if call.kernel == KernelId::from_static("BINARY:Div") {
            BinaryKernel::Div
        } else if call.kernel == KernelId::from_static("BINARY:Mod") {
            BinaryKernel::Mod
        } else if call.kernel == KernelId::from_static("BINARY:Pow") {
            BinaryKernel::Pow
        } else if call.kernel == KernelId::from_static("BINARY:Gt") {
            BinaryKernel::Gt
        } else if call.kernel == KernelId::from_static("BINARY:Lt") {
            BinaryKernel::Lt
        } else if call.kernel == KernelId::from_static("BINARY:Gte") {
            BinaryKernel::Gte
        } else if call.kernel == KernelId::from_static("BINARY:Lte") {
            BinaryKernel::Lte
        } else if call.kernel == KernelId::from_static("BINARY:Eq") {
            BinaryKernel::Eq
        } else if call.kernel == KernelId::from_static("BINARY:Neq") {
            BinaryKernel::Neq
        } else if call.kernel == KernelId::from_static("BINARY:And") {
            BinaryKernel::And
        } else if call.kernel == KernelId::from_static("BINARY:Or") {
            BinaryKernel::Or
        } else if call.kernel == KernelId::from_static("BINARY:Xor") {
            BinaryKernel::Xor
        } else if call.kernel == KernelId::from_static("CALL:MAX") {
            BinaryKernel::Max
        } else if call.kernel == KernelId::from_static("CALL:MIN") {
            BinaryKernel::Min
        } else {
            return Err(KernelDispatchError::new(Self::ERR_UNSUPPORTED_KERNEL));
        };
        binary(call, buffers, op)
    }
}

/// Execute a one-series formula function directly into the plan-owned output.
///
/// Formula literals are lowered to `NUMBER` buffers, so the period is read from
/// the first element of the second input buffer. The executor guarantees that
/// live dependencies do not alias the output slot; raw pointers let us keep
/// that invariant without allocating temporary `Vec`s or borrowing the whole
/// arena for the duration of the kernel call.
/// Run an allocating rate-of-change kernel into a preallocated buffer.
///
/// Canonical `rocp`/`rocr`/`rocr100` return an owned series, so the result is
/// copied rather than computed in place. A failure fills the output with NaN,
/// matching what the tree path does for an invalid period.
fn rate_of_change_into(
    kernel: fn(&[f64], usize) -> Result<Array1<f64>, TaError>,
    input: &[f64],
    period: usize,
    output: &mut [f64],
) -> Result<(), KernelDispatchError> {
    match kernel(input, period) {
        Ok(values) => {
            let values = values.as_slice().unwrap_or(&[]);
            if values.len() != output.len() {
                return Err(KernelDispatchError::new(
                    FormulaKernelDispatcher::ERR_PARAMETER,
                ));
            }
            output.copy_from_slice(values);
            Ok(())
        }
        Err(_) => {
            output.fill(f64::NAN);
            Ok(())
        }
    }
}

/// Element-wise arithmetic behind the `ADD` / `SUB` / `MULT` / `DIV` functions.
///
/// These deliberately do **not** reuse `BINARY:<op>`. `DIV` guards with
/// `rhs == 0.0`, while `BinaryKernel::Div` guards with `rhs.abs() < 1e-15`;
/// routing one to the other would turn `a / 1e-20` into NaN (or the reverse
/// into an infinity) depending on which path ran. The rule is to reproduce what
/// the tree path's `fn_add` / `fn_sub` / `fn_mult` / `fn_div` do, exactly.
enum ArithKernel {
    Add,
    Sub,
    Mul,
    Div,
}

fn dispatch_arith_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: ArithKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_ARITY,
        ));
    }
    let lhs = call.inputs[0].0;
    let rhs = call.inputs[1].0;
    let out = call.output.0;
    let length = buffers[out].len();
    for index in 0..length {
        // Read both operands before writing: element-wise, so a single index is
        // all that is needed, which keeps this correct even if the allocator
        // aliased the output onto an operand.
        let a = buffers[lhs][index];
        let b = buffers[rhs][index];
        buffers[out][index] = match op {
            ArithKernel::Add => a + b,
            ArithKernel::Sub => a - b,
            ArithKernel::Mul => a * b,
            ArithKernel::Div => {
                if b == 0.0 {
                    f64::NAN
                } else {
                    a / b
                }
            }
        };
    }
    Ok(())
}

/// Windowed functions behind `MINUS` / `MAXINDEX` / `MININDEX` / `HHVBARS` /
/// `LLVBARS`.
///
/// All five reproduce their `fn_*` counterparts bar for bar, including the warm-up
/// rules, which differ between them and are the easiest thing to get wrong:
///
/// - `MINUS` and the `*INDEX` pair leave the first `n - 1` bars NaN.
/// - `HHVBARS` / `LLVBARS` clamp the window with `saturating_sub`, so they
///   produce a value from bar 0 and have no warm-up at all.
///
/// Ties keep the **first** occurrence, because the comparisons are strict
/// (`>` / `<`) rather than `>=` / `<=`.
enum WindowKernel {
    /// `MINUS(X, N)`: `X[i] - X[i-N]`.
    Minus,
    /// `MAXINDEX(X, N)`: offset of the window maximum, counted from its start.
    MaxIndex,
    /// `MININDEX(X, N)`: offset of the window minimum, counted from its start.
    MinIndex,
    /// `HHVBARS(X, N)`: bars since the highest value in the window.
    HhvBars,
    /// `LLVBARS(X, N)`: bars since the lowest value in the window.
    LlvBars,
}

fn dispatch_window_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: WindowKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_ARITY,
        ));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || period_slot == output_slot {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let input_len = buffers[input_slot].len();
    let output_len = buffers[output_slot].len();
    if input_len != output_len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    // Both operands were checked to be distinct slots above, so the aliasing
    // this requires is exactly the aliasing the dispatcher already forbids.
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, input_len),
            std::slice::from_raw_parts_mut(output_ptr, output_len),
        )
    };

    output.fill(f64::NAN);
    match op {
        WindowKernel::Minus => {
            for index in period..input.len() {
                output[index] = input[index] - input[index - period];
            }
        }
        WindowKernel::MaxIndex | WindowKernel::MinIndex => {
            let want_max = matches!(op, WindowKernel::MaxIndex);
            for index in 0..input.len() {
                if index + 1 < period {
                    continue;
                }
                let start = index + 1 - period;
                let mut best = if want_max {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
                let mut best_offset = 0.0;
                for position in start..=index {
                    let value = input[position];
                    if value.is_nan() {
                        continue;
                    }
                    if (want_max && value > best) || (!want_max && value < best) {
                        best = value;
                        best_offset = (position - start) as f64;
                    }
                }
                let found = if want_max {
                    best > f64::NEG_INFINITY
                } else {
                    best < f64::INFINITY
                };
                if found {
                    output[index] = best_offset;
                }
            }
        }
        WindowKernel::HhvBars | WindowKernel::LlvBars => {
            let want_high = matches!(op, WindowKernel::HhvBars);
            for index in 0..input.len() {
                let start = (index + 1).saturating_sub(period);
                let mut best = if want_high {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                };
                let mut best_position: Option<usize> = None;
                for position in start..=index {
                    let value = input[position];
                    if value.is_nan() {
                        continue;
                    }
                    if (want_high && value > best) || (!want_high && value < best) {
                        best = value;
                        best_position = Some(position);
                    }
                }
                if let Some(position) = best_position {
                    output[index] = (index - position) as f64;
                }
            }
        }
    }
    Ok(())
}

/// Single-argument maths behind `SQRT` / `SINH` / `COSH` / `TANH`.
///
/// `SQRT` reproduces `fn_sqrt`'s negative guard rather than calling `sqrt`
/// unconditionally: the tree path maps `v < 0.0` to NaN, and a bare `sqrt` would
/// match that anyway, but the guard is stated so the two stay tied together.
enum UnaryMathKernel {
    Sqrt,
    Sinh,
    Cosh,
    Tanh,
}

fn dispatch_unary_math_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: UnaryMathKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 1 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_ARITY,
        ));
    }
    let input = call.inputs[0].0;
    let out = call.output.0;
    let length = buffers[out].len();
    for index in 0..length {
        let value = buffers[input][index];
        buffers[out][index] = match op {
            UnaryMathKernel::Sqrt => {
                if value < 0.0 {
                    f64::NAN
                } else {
                    value.sqrt()
                }
            }
            UnaryMathKernel::Sinh => value.sinh(),
            UnaryMathKernel::Cosh => value.cosh(),
            UnaryMathKernel::Tanh => value.tanh(),
        };
    }
    Ok(())
}

/// `INDEX(array, index)`: per-bar historical element access, i.e. `array[index]`.
///
/// This is a **gather, not a shift**, and the difference is the whole point.
/// Every output bar receives `array` at the bar that *that bar's* index
/// expression names. When the index is a constant — which is what an unrolled
/// loop produces — the result is one historical element broadcast across the
/// whole series. `REF(X, N)` is a different operation and must never be
/// substituted for this one.
///
/// The semantics mirror the AST interpreter in `executor.rs` exactly, including
/// the edge cases, because the differential gate compares the two paths
/// element by element:
///
/// - The index is converted with Rust's saturating `f64 as usize` cast, so NaN
///   and negative values land on index `0` rather than panicking.
/// - An index past the end of `array` yields NaN.
fn dispatch_index_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let array_slot = call.inputs[0].0;
    let index_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    let length = buffers[output_slot].len();

    // Read both operands before writing: the gather can address any bar of
    // `array`, so writing in place while reading would corrupt the result if
    // the allocator ever gave the output the same buffer as an operand.
    let mut gathered = Vec::with_capacity(length);
    {
        let array = &buffers[array_slot];
        let index = &buffers[index_slot];
        for bar in 0..length {
            let at = if bar < index.len() {
                index[bar]
            } else {
                f64::NAN
            };
            // Saturating by design: this must agree with the interpreter, where
            // `idx[i] as usize` sends NaN and negatives to index 0.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let position = at as usize;
            gathered.push(if position < array.len() {
                array[position]
            } else {
                f64::NAN
            });
        }
    }
    let output = &mut buffers[output_slot];
    output.copy_from_slice(&gathered);
    Ok(())
}

fn dispatch_periodic_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is_sma = call.kernel == KernelId::from_static("CALL:SMA");
    if if is_sma {
        !(2..=3).contains(&call.inputs.len())
    } else {
        call.inputs.len() != 2
    } {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot
        || period_slot == output_slot
        || call.inputs.iter().skip(2).any(|slot| slot.0 == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let input_len = buffers[input_slot].len();
    let output_len = buffers[output_slot].len();
    if input_len != output_len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, input_len),
            std::slice::from_raw_parts_mut(output_ptr, output_len),
        )
    };

    // Rolling extrema return their own error type, so they return early rather
    // than joining the `TaError`-typed chain below.
    if call.kernel == KernelId::from_static("CALL:HHV") {
        return crate::math::kernels::rolling_max_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }
    if call.kernel == KernelId::from_static("CALL:LLV") {
        return crate::math::kernels::rolling_min_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }
    if call.kernel == KernelId::from_static("CALL:SUM") {
        return sum_formula_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }
    if call.kernel == KernelId::from_static("CALL:REF") {
        return ref_formula_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }
    // Rate-of-change ratio variants. These delegate to the *same* canonical
    // functions the tree path calls (`fn_rocp`/`fn_rocr`/`fn_rocr100`), and on
    // error they mirror the tree path by emitting NaN instead of failing, so
    // the two paths cannot diverge on a bad period.
    if call.kernel == KernelId::from_static("CALL:ROCP") {
        return rate_of_change_into(crate::indicators::momentum::rocp, input, period, output);
    }
    if call.kernel == KernelId::from_static("CALL:ROCR") {
        return rate_of_change_into(crate::indicators::momentum::rocr, input, period, output);
    }
    if call.kernel == KernelId::from_static("CALL:ROCR100") {
        return rate_of_change_into(crate::indicators::momentum::rocr100, input, period, output);
    }

    let result = if is_sma {
        let multiplier = call
            .inputs
            .get(2)
            .map(|slot| scalar_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(1.0);
        sma_formula_into(input, period, multiplier, output)
    } else if call.kernel == KernelId::from_static("CALL:MA") {
        crate::math::moving_avg::sma_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:EMA") {
        crate::math::moving_avg::ema_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:WMA") {
        crate::math::moving_avg::wma_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:KAMA") {
        crate::math::moving_avg::kama_into(input, period, 2, 30, output)
    } else if call.kernel == KernelId::from_static("CALL:RSI") {
        crate::indicators::rsi_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:MOM") {
        crate::indicators::mom_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:TRIMA") {
        crate::math::moving_avg::trima_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:TRIX") {
        crate::indicators::momentum::trix_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:STD") {
        crate::math::rolling_stats::stddev_into(input, period, 1.0, output)
    } else {
        crate::indicators::roc_into(input, period, output)
    };

    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute terminal SMA(X, N[, M]) without materialising argument arrays.
///
/// In the formula language SMA is recursive smoothing, distinct from MA's
/// rolling arithmetic mean. Keep its NaN propagation and first-value seed
/// identical to the compatibility implementation.
fn sma_formula_into(
    input: &[f64],
    period: usize,
    multiplier: f64,
    output: &mut [f64],
) -> crate::error::Result<()> {
    if period == 0 || !multiplier.is_finite() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "SMA parameters".to_string(),
            constraint: "period must be > 0 and multiplier must be finite".to_string(),
        });
    }
    output.fill(f64::NAN);
    let denominator = period as f64;
    let retained = denominator - multiplier;
    let mut previous = None;
    for (index, &current) in input.iter().enumerate() {
        if current.is_nan() {
            continue;
        }
        let value = previous
            .map(|old| (multiplier * current + retained * old) / denominator)
            .unwrap_or(current);
        output[index] = value;
        previous = Some(value);
    }
    Ok(())
}

/// Execute terminal SUM(X, N) without materialising argument arrays.
///
/// Mirrors the reference implementation exactly, including its NaN behaviour:
/// once a NaN enters the window it stays in the running sum until it slides out,
/// because the accumulator is never re-seeded.
fn sum_formula_into(input: &[f64], period: usize, output: &mut [f64]) -> crate::error::Result<()> {
    if period == 0 || input.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "SUM parameters".to_string(),
            constraint: "period must be > 0 and input/output lengths must match".to_string(),
        });
    }
    output.fill(f64::NAN);
    let mut running = 0.0;
    for (index, &current) in input.iter().enumerate() {
        running += current;
        if index >= period {
            running -= input[index - period];
            output[index] = running;
        } else if index == period - 1 {
            output[index] = running;
        }
    }
    Ok(())
}

/// Execute terminal REF(X, N) without materialising argument arrays.
fn ref_formula_into(input: &[f64], period: usize, output: &mut [f64]) -> crate::error::Result<()> {
    if period == 0 || input.len() != output.len() {
        return Err(crate::error::TaError::InvalidParameter {
            name: "REF parameters".to_string(),
            constraint: "period must be > 0 and input/output lengths must match".to_string(),
        });
    }
    output.fill(f64::NAN);
    for index in period..input.len() {
        output[index] = input[index - period];
    }
    Ok(())
}

/// Execute Pine `math.avg(a, b, ...)` — the arithmetic mean of every input.
///
/// Variadic on purpose: Pine's `math.avg` accepts two or more series, and the
/// plan carries them as an ordered operand list.
fn mean_formula_into(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.is_empty() {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let output_slot = call.output.0;
    let len = buffers[output_slot].len();
    let divisor = call.inputs.len() as f64;
    let mut accumulator = vec![0.0; len];
    for input in call.inputs {
        let slot = input.0;
        if slot == output_slot || buffers[slot].len() != len {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
        for (total, value) in accumulator.iter_mut().zip(&buffers[slot]) {
            *total += value;
        }
    }
    for (output, total) in buffers[output_slot].iter_mut().zip(&accumulator) {
        *output = total / divisor;
    }
    Ok(())
}

fn period_from_slot(buffers: &[Vec<f64>], slot: usize) -> Result<usize, KernelDispatchError> {
    let value = buffers
        .get(slot)
        .and_then(|values| values.first())
        .copied()
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if !value.is_finite() || value <= 0.0 || value.fract() != 0.0 {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    Ok(value as usize)
}

#[inline]
fn scalar_from_slot(buffers: &[Vec<f64>], slot: usize) -> Result<f64, KernelDispatchError> {
    buffers
        .get(slot)
        .and_then(|values| values.first())
        .copied()
        .filter(|value| value.is_finite())
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute ATR directly from the HLC dependency slots.
/// Execute the two-operand `CCI(source, period)` form into the plan-owned
/// output.
///
/// Delegates to `momentum::cci_source_into`, which is the same function
/// `fn_cci` now calls for its two-operand branch, so the tree path and the
/// compiled-plan kernel cannot drift. The four-operand HLC form keeps routing
/// to [`dispatch_hlc_periodic_call`].
fn dispatch_cci_source_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let source_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if [source_slot, period_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[source_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrow ends before the output is borrowed mutably.
    let result = {
        let source = &buffers[source_slot];
        let mut cci = vec![f64::NAN; len];
        crate::indicators::momentum::cci_source_into(source, period, &mut cci)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        cci
    };

    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != result.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(&result);
    Ok(())
}

fn dispatch_hlc_periodic_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 4 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let period_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot, period_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let high_ptr = buffers[high_slot].as_ptr();
    let low_ptr = buffers[low_slot].as_ptr();
    let close_ptr = buffers[close_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (high, low, close, output) = unsafe {
        (
            std::slice::from_raw_parts(high_ptr, len),
            std::slice::from_raw_parts(low_ptr, len),
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    let result = if call.kernel == KernelId::from_static("CALL:ATR") {
        crate::indicators::volatility::atr_into(high, low, close, period, output)
    } else if call.kernel == KernelId::from_static("CALL:NATR") {
        crate::indicators::volatility::natr_into(high, low, close, period, output)
    } else {
        crate::indicators::momentum::cci_into(high, low, close, period, output)
    };
    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute `IF(COND, A, B)` and Pine's `cond ? A : B`, which both lower to
/// `CALL:IF`.
///
/// Delegated to [`crate::formula::simd::SimdOps::select`] — the same primitive
/// `fn_if` reaches for series it can vectorise — so the two paths agree on
/// truthiness as well as on values.
///
/// Truthiness here is `condition != 0.0`. This is **not** universal across the
/// codebase: `stateful.rs` and `fn_if`'s short-series fallback use `> 0.0`,
/// which differs for negative and NaN conditions. Corpus-length series take the
/// vectorised path in the tree executor, so `!= 0.0` is the convention the plan
/// has to match; the inconsistency itself is a separate pre-existing issue.
/// `WINNER(price)` / `COST(ratio)` over the host-supplied chip distribution.
///
/// Mirrors the tree path exactly (`fn_winner` / `fn_cost` in the compatibility
/// table): with no chip data both yield `NaN`, and `COST` divides its argument
/// by 100 before applying the `0..=1` range check. Delegating to the same
/// `ChipData` methods is what keeps the two paths numerically identical.
fn dispatch_chip_call(
    host: &HostContext,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 1 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input = call.inputs[0].0;
    let output = call.output.0;
    let is_cost = call.kernel == KernelId::from_static("CALL:COST");
    let Some(chip) = host.chip.as_ref() else {
        buffers[output].fill(f64::NAN);
        return Ok(());
    };
    let len = buffers[output].len();
    for index in 0..len {
        let value = buffers[input][index];
        buffers[output][index] = if is_cost {
            let ratio = value / 100.0;
            if (0.0..=1.0).contains(&ratio) {
                chip.cost(ratio)
            } else {
                f64::NAN
            }
        } else {
            chip.winner(value)
        };
    }
    Ok(())
}

/// `PERIODTYPE()`: a constant series carrying the host's chart period.
fn dispatch_periodtype_call(
    host: &HostContext,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !call.inputs.is_empty() {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    buffers[call.output.0].fill(host.period_type as f64);
    Ok(())
}

/// `REFDATE(X, DATE)`: a constant series holding `X` at one bar index.
///
/// The date operand is read from its first element and cast the same way the
/// tree path does, so a negative or non-finite operand saturates to index 0
/// rather than behaving differently between the two paths.
fn dispatch_refdate_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let source = call.inputs[0].0;
    let date = call.inputs[1].0;
    let output = call.output.0;
    let index = buffers[date][0] as usize;
    let value = if index < buffers[source].len() {
        buffers[source][index]
    } else {
        f64::NAN
    };
    buffers[output].fill(value);
    Ok(())
}

fn dispatch_if_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 3 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let cond_slot = call.inputs[0].0;
    let then_slot = call.inputs[1].0;
    let else_slot = call.inputs[2].0;
    let output_slot = call.output.0;
    if [cond_slot, then_slot, else_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let len = buffers[output_slot].len();
    if buffers[cond_slot].len() != len
        || buffers[then_slot].len() != len
        || buffers[else_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let cond_ptr = buffers[cond_slot].as_ptr();
    let then_ptr = buffers[then_slot].as_ptr();
    let else_ptr = buffers[else_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (cond, then_values, else_values, output) = unsafe {
        (
            std::slice::from_raw_parts(cond_ptr, len),
            std::slice::from_raw_parts(then_ptr, len),
            std::slice::from_raw_parts(else_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::formula::simd::SimdOps::select(cond, then_values, else_values, output);
    Ok(())
}

/// Execute formula BOLL/BBANDS as its public upper-band projection.
fn dispatch_bbands_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=3).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || period_slot == output_slot {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let nb_dev = if let Some(parameter) = call.inputs.get(2) {
        let values = &buffers[parameter.0];
        let value = values
            .first()
            .copied()
            .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        if !value.is_finite() {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
        value
    } else {
        2.0
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::math::rolling_stats::bbands_upper_into(input, period, nb_dev, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute the single-band `BBANDS` projections into the plan-owned output.
///
/// `BOLLUP` / `BOLLMID` / `BOLLDN` are the three legs of one
/// `overlap::bbands` call — the same call `canonical_bband_component` makes on
/// the tree path — so the kernel picks a leg instead of recomputing. Note this
/// is a different entry point from `CALL:BBANDS`, which only needs the upper
/// band and uses the cheaper `rolling_stats::bbands_upper_into`; the corpus is
/// what keeps the two honest.
fn dispatch_boll_band_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=3).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let period_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if input_slot == output_slot
        || period_slot == output_slot
        || call.inputs.get(2).is_some_and(|slot| slot.0 == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let nb_dev = match call.inputs.get(2) {
        Some(parameter) => scalar_from_slot(buffers, parameter.0)?,
        None => 2.0,
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrow ends before the output is borrowed mutably.
    let result = {
        let input = &buffers[input_slot];
        crate::indicators::overlap::bbands(input, period, nb_dev, nb_dev)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
    };
    let source = if call.kernel == KernelId::from_static("CALL:BOLLMID") {
        result.middle
    } else if call.kernel == KernelId::from_static("CALL:BOLLDN") {
        result.lower
    } else {
        result.upper
    };
    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != source.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(source.as_slice().unwrap());
    Ok(())
}

/// Execute formula MACD as its historical MACD/DIF-line projection.
fn dispatch_macd_line_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=4).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || call.inputs.iter().skip(1).any(|slot| slot.0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let fast = period_from_slot(buffers, call.inputs[1].0)?;
    let slow = if let Some(slot) = call.inputs.get(2) {
        period_from_slot(buffers, slot.0)?
    } else {
        26
    };
    let signal = if let Some(slot) = call.inputs.get(3) {
        period_from_slot(buffers, slot.0)?
    } else {
        9
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::indicators::macd_line_into(input, fast, slow, signal, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute `DEA` — the MACD signal line — into the plan-owned output.
///
/// Delegates to `momentum::macd` and keeps its `.signal` leg, which is exactly
/// what `fn_dea` does on the tree path. This is deliberately a *different*
/// entry point from `CALL:MACD`, which uses `macd_line_into` for the DIF line
/// only; the differential corpus is what keeps the two honest.
fn dispatch_dea_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(2..=4).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input_slot = call.inputs[0].0;
    let output_slot = call.output.0;
    if input_slot == output_slot || call.inputs.iter().skip(1).any(|slot| slot.0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let fast = period_from_slot(buffers, call.inputs[1].0)?;
    let slow = if let Some(slot) = call.inputs.get(2) {
        period_from_slot(buffers, slot.0)?
    } else {
        26
    };
    let signal = if let Some(slot) = call.inputs.get(3) {
        period_from_slot(buffers, slot.0)?
    } else {
        9
    };
    let len = buffers[output_slot].len();
    if buffers[input_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrow ends before the output is borrowed mutably.
    let result = {
        let input = &buffers[input_slot];
        crate::indicators::momentum::macd(input, fast, slow, signal)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
    };
    let source = result.signal;
    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != source.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(source.as_slice().unwrap());
    Ok(())
}

/// Execute OBV using the canonical caller-owned volume kernel.
fn dispatch_obv(call: KernelCall<'_>, buffers: &mut [Vec<f64>]) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let close_slot = call.inputs[0].0;
    let volume_slot = call.inputs[1].0;
    let output_slot = call.output.0;
    if close_slot == output_slot || volume_slot == output_slot {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let len = buffers[output_slot].len();
    if buffers[close_slot].len() != len || buffers[volume_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let close_ptr = buffers[close_slot].as_ptr();
    let volume_ptr = buffers[volume_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (close, volume, output) = unsafe {
        (
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts(volume_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::math::volume_kernels::obv_into(close, volume, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute AD/ADOSC/MFI from the canonical caller-owned volume kernels.
fn dispatch_volume_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is_ad = call.kernel == KernelId::from_static("CALL:AD");
    let is_adosc = call.kernel == KernelId::from_static("CALL:ADOSC");
    let valid_arity = if is_ad {
        call.inputs.len() == 4
    } else if is_adosc {
        (4..=6).contains(&call.inputs.len())
    } else {
        call.inputs.len() == 5
    };
    if !valid_arity {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let volume_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot, volume_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
        || call.inputs.iter().skip(4).any(|slot| slot.0 == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
        || buffers[volume_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let high_ptr = buffers[high_slot].as_ptr();
    let low_ptr = buffers[low_slot].as_ptr();
    let close_ptr = buffers[close_slot].as_ptr();
    let volume_ptr = buffers[volume_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    let (high, low, close, volume, output) = unsafe {
        (
            std::slice::from_raw_parts(high_ptr, len),
            std::slice::from_raw_parts(low_ptr, len),
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts(volume_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };

    let result = if is_ad {
        crate::math::volume_kernels::ad_into(high, low, close, volume, output)
    } else if is_adosc {
        let fast = call
            .inputs
            .get(4)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let slow = call
            .inputs
            .get(5)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(10);
        crate::math::volume_kernels::adosc_into(high, low, close, volume, fast, slow, output)
    } else {
        let period = period_from_slot(buffers, call.inputs[4].0)?;
        crate::math::mfi::mfi_into(high, low, close, volume, period, output)
    };
    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute `AROON_UP` / `AROON_DN` into the plan-owned output.
///
/// Both legs delegate to `indicators::momentum::aroon`, which is the *same*
/// function the tree path calls from `fn_aroon_up` / `fn_aroon_dn`. That is
/// deliberate: `momentum::aroon_into` is a separately optimised implementation
/// (monotonic-queue rather than a window rescan), and routing this kernel to it
/// would let the two paths disagree the moment the variants drift. Correctness
/// first — the extra allocation can go once an equivalence test pins the fast
/// variant to the reference.
fn dispatch_aroon_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 3 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let period_slot = call.inputs[2].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, period_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len || buffers[low_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrows end before the output is borrowed mutably.
    let result = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        crate::indicators::momentum::aroon(high, low, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
    };
    let source = if call.kernel == KernelId::from_static("CALL:AROON_UP") {
        result.aroon_up
    } else {
        result.aroon_down
    };
    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != source.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(source.as_slice().unwrap());
    Ok(())
}

/// Execute the directional-movement family into the plan-owned output.
///
/// `PLUS_DI` / `MINUS_DI` delegate to `momentum::plus_di` / `momentum::minus_di`
/// — the same functions `fn_plus_di` / `fn_minus_di` call on the tree path. The
/// DX + Wilder/RMA tail of the five-argument `ADX` contract is not reimplemented
/// here either: it lives in `momentum::adx_from_di_into`, which `fn_adx` now
/// calls too, so the two paths cannot drift.
///
/// `ADX` accepts both the domestic four-argument form `(HIGH, LOW, CLOSE, N)`,
/// which smooths DX with the directional-movement length, and Pine's
/// five-argument `ta.dmi` form, which keeps the two lengths distinct.
fn dispatch_dmi_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is_adx = call.kernel == KernelId::from_static("CALL:ADX");
    if call.inputs.len() != 4 && !(is_adx && call.inputs.len() == 5) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let period_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    let used = [high_slot, low_slot, close_slot, period_slot];
    if used.contains(&output_slot) || (call.inputs.len() == 5 && call.inputs[4].0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrows end before the output is borrowed mutably.
    let result = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        let close = &buffers[close_slot];
        let invalid = || KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER);
        if call.kernel == KernelId::from_static("CALL:PLUS_DI") {
            crate::indicators::momentum::plus_di(high, low, close, period)
                .map_err(|_| invalid())?
                .to_vec()
        } else if call.kernel == KernelId::from_static("CALL:MINUS_DI") {
            crate::indicators::momentum::minus_di(high, low, close, period)
                .map_err(|_| invalid())?
                .to_vec()
        } else if call.inputs.len() == 4 {
            crate::indicators::momentum::adx(high, low, close, period)
                .map_err(|_| invalid())?
                .to_vec()
        } else {
            let adx_n = period_from_slot(buffers, call.inputs[4].0)?;
            let plus_di = crate::indicators::momentum::plus_di(high, low, close, period)
                .map_err(|_| invalid())?;
            let minus_di = crate::indicators::momentum::minus_di(high, low, close, period)
                .map_err(|_| invalid())?;
            let mut adx = vec![f64::NAN; len];
            crate::indicators::momentum::adx_from_di_into(
                plus_di.as_slice().unwrap(),
                minus_di.as_slice().unwrap(),
                adx_n,
                &mut adx,
            )
            .map_err(|_| invalid())?;
            adx
        }
    };

    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != result.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(&result);
    Ok(())
}

/// Execute `WILLR` into the plan-owned output.
///
/// Delegates to `momentum::willr_into`, which is what `momentum::willr` calls
/// and therefore what `fn_willr` now resolves to on the tree path. Both paths
/// share the TA-Lib-golden extrema lifecycle instead of keeping a third copy.
fn dispatch_willr_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 4 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let period_slot = call.inputs[3].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot, period_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let period = period_from_slot(buffers, period_slot)?;
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrows end before the output is borrowed mutably.
    let result = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        let close = &buffers[close_slot];
        let mut willr = vec![f64::NAN; len];
        crate::indicators::momentum::willr_into(high, low, close, period, &mut willr)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        willr
    };

    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != result.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(&result);
    Ok(())
}

/// Execute `SAR` into the plan-owned output.
///
/// Accepts the four-argument `(HIGH, LOW, start, max)` form, where the
/// increment equals the start, and the five-argument `(HIGH, LOW, start,
/// increment, max)` form that Pine's `ta.sar` lowers to. Both delegate to
/// `overlap::sar_with_factors_into`, which `fn_sar` now calls too.
///
/// This deliberately does *not* use `overlap::sar`: that entry point only
/// takes `(acceleration, maximum)` and so cannot express an increment that
/// differs from the start.
fn dispatch_sar_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(4..=5).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let start_slot = call.inputs[2].0;
    let output_slot = call.output.0;
    if call.inputs.iter().any(|slot| slot.0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let af_start = scalar_from_slot(buffers, start_slot)?;
    let (af_increment, af_max) = if call.inputs.len() == 5 {
        (
            scalar_from_slot(buffers, call.inputs[3].0)?,
            scalar_from_slot(buffers, call.inputs[4].0)?,
        )
    } else {
        (af_start, scalar_from_slot(buffers, call.inputs[3].0)?)
    };
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len || buffers[low_slot].len() != len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrows end before the output is borrowed mutably.
    let result = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        let mut sar = vec![f64::NAN; len];
        crate::indicators::overlap::sar_with_factors_into(
            high,
            low,
            af_start,
            af_increment,
            af_max,
            &mut sar,
        )
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        sar
    };

    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != result.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(&result);
    Ok(())
}

/// Execute `STOCHF` into the plan-owned output.
///
/// Delegates to `momentum::stochf` and keeps its fast-K leg, which is exactly
/// what `fn_stochf` does on the tree path. Accepts three to five operands:
/// `(HIGH, LOW, CLOSE)` with both periods defaulted, plus optional `fastK` and
/// `fastD`. Pine's `ta.stoch` uses the five-operand form with a fast-D period
/// of 1, which is what makes it the unsmoothed stochastic.
fn dispatch_stochf_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !(3..=5).contains(&call.inputs.len()) {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let output_slot = call.output.0;
    if call.inputs.iter().any(|slot| slot.0 == output_slot) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let fast_k = if let Some(slot) = call.inputs.get(3) {
        period_from_slot(buffers, slot.0)?
    } else {
        5
    };
    let fast_d = if let Some(slot) = call.inputs.get(4) {
        period_from_slot(buffers, slot.0)?
    } else {
        3
    };
    let len = buffers[output_slot].len();
    if buffers[high_slot].len() != len
        || buffers[low_slot].len() != len
        || buffers[close_slot].len() != len
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    // Scoped so the immutable borrows end before the output is borrowed mutably.
    let result = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        let close = &buffers[close_slot];
        crate::indicators::momentum::stochf(high, low, close, fast_k, fast_d)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?
    };
    let source = result.k;
    let output = buffers
        .get_mut(output_slot)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    if output.len() != source.len() {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    output.copy_from_slice(source.as_slice().unwrap());
    Ok(())
}

/// Acknowledge a drawing directive without producing a numeric series.
///
/// Drawing commands are chart side effects, so they have no numeric result.
/// They are nevertheless kept as plan roots **on purpose**: `hot_plan.rs`
/// retains them so that "an unsupported drawing still fails the plan loudly
/// instead of vanishing" under dead-code elimination. The numeric runtime's
/// side of that bargain is to execute them and write no series, leaving
/// rendering to the host. Pruning them instead would contradict that decision.
///
/// The set is enumerated rather than prefix-matched because [`KernelId`] is a
/// hash with no string accessor. The four names are the complete set the IR
/// produces today (`compute_ir.rs`: `STICK_LINE`, `DRAW_TEXT`, `DRAW_ICON`, and
/// `DRAW:<command>` for `DrawGeneric`).
fn dispatch_draw_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.iter().any(|slot| slot.0 == call.output.0) {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let output = buffers
        .get_mut(call.output.0)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    output.fill(f64::NAN);
    Ok(())
}

/// Execute the formula catalogue's popular non-TA-Lib indicators through the
/// same hot-plan ABI as the TA-Lib-compatible kernels.  The two most common
/// rolling transforms (ZSCORE and VWMA) write directly into the plan buffer;
/// composite indicators reuse their canonical public kernels and copy only the
/// selected projection into the caller-owned output.
fn dispatch_modern_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    let is = |name| call.kernel == KernelId::from_static(name);
    let arity_ok = if is("CALL:VWAP") {
        call.inputs.len() == 2 || call.inputs.len() == 4
    } else if is("CALL:VWMA") {
        call.inputs.len() == 3
    } else if is("CALL:ZSCORE") {
        call.inputs.len() == 2
    } else if is("CALL:CMF") {
        call.inputs.len() == 5
    } else if is("CALL:CHOP") {
        call.inputs.len() == 4
    } else if is("CALL:FISHER") || is("CALL:FISHER_SIGNAL") {
        call.inputs.len() == 3
    } else if is("CALL:TSI") {
        (2..=3).contains(&call.inputs.len())
    } else if is("CALL:KDJ") || is("CALL:KDJ_D") || is("CALL:KDJ_J") {
        (3..=6).contains(&call.inputs.len())
    } else if is("CALL:SUPERTREND") {
        (4..=5).contains(&call.inputs.len())
    } else {
        call.inputs.len() == 3
    };
    if !arity_ok {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let output_slot = call.output.0;
    let len = buffers
        .get(output_slot)
        .map(Vec::len)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    for input in call.inputs {
        if input.0 == output_slot
            || buffers
                .get(input.0)
                .is_none_or(|values| values.len() != len)
        {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ));
        }
    }

    if is("CALL:ZSCORE") {
        let period = period_from_slot(buffers, call.inputs[1].0)?;
        let input_ptr = buffers[call.inputs[0].0].as_ptr();
        let output_ptr = buffers[output_slot].as_mut_ptr();
        let (input, output) = unsafe {
            (
                std::slice::from_raw_parts(input_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::indicators::statistics::zscore_into(input, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    if is("CALL:VWMA") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let input_ptr = buffers[call.inputs[0].0].as_ptr();
        let volume_ptr = buffers[call.inputs[1].0].as_ptr();
        let output_ptr = buffers[output_slot].as_mut_ptr();
        let (input, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(input_ptr, len),
                std::slice::from_raw_parts(volume_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::math::moving_avg::vwma_into(input, volume, period, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    if is("CALL:VWAP") {
        let output_ptr = buffers[output_slot].as_mut_ptr();
        if call.inputs.len() == 2 {
            let price_ptr = buffers[call.inputs[0].0].as_ptr();
            let volume_ptr = buffers[call.inputs[1].0].as_ptr();
            let (price, volume, output) = unsafe {
                (
                    std::slice::from_raw_parts(price_ptr, len),
                    std::slice::from_raw_parts(volume_ptr, len),
                    std::slice::from_raw_parts_mut(output_ptr, len),
                )
            };
            let mut price_volume = 0.0;
            let mut total_volume = 0.0;
            for index in 0..len {
                price_volume = price[index].mul_add(volume[index], price_volume);
                total_volume += volume[index];
                output[index] = if total_volume.abs() > 1e-15 {
                    price_volume / total_volume
                } else {
                    f64::NAN
                };
            }
            return Ok(());
        }
        let high_ptr = buffers[call.inputs[0].0].as_ptr();
        let low_ptr = buffers[call.inputs[1].0].as_ptr();
        let close_ptr = buffers[call.inputs[2].0].as_ptr();
        let volume_ptr = buffers[call.inputs[3].0].as_ptr();
        let (high, low, close, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(high_ptr, len),
                std::slice::from_raw_parts(low_ptr, len),
                std::slice::from_raw_parts(close_ptr, len),
                std::slice::from_raw_parts(volume_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return crate::math::volume_kernels::vwap_into(high, low, close, volume, output)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER));
    }

    let copy_result = |output: &mut [f64], result: &[f64]| {
        if result.len() == output.len() {
            output.copy_from_slice(result);
            Ok(())
        } else {
            Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ))
        }
    };

    if is("CALL:CMF") {
        let period = period_from_slot(buffers, call.inputs[4].0)?;
        let (high, low, close, volume, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[3].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::volume_ext::cmf(high, low, close, volume, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:CHOP") {
        let period = period_from_slot(buffers, call.inputs[3].0)?;
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::chop(high, low, close, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:FISHER") || is("CALL:FISHER_SIGNAL") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let (high, low, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::fisher(high, low, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let selected = if is("CALL:FISHER") {
            result.fisher.as_slice().unwrap()
        } else {
            result.signal.as_slice().unwrap()
        };
        return copy_result(output, selected);
    }

    if is("CALL:TSI") {
        let long_period = period_from_slot(buffers, call.inputs[1].0)?;
        let short_period = call
            .inputs
            .get(2)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(13);
        let (input, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::momentum_ext::tsi(input, long_period, short_period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.as_slice().unwrap());
    }

    if is("CALL:KDJ") || is("CALL:KDJ_D") || is("CALL:KDJ_J") {
        let n = call
            .inputs
            .get(3)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(9);
        let m1 = call
            .inputs
            .get(4)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let m2 = call
            .inputs
            .get(5)
            .map(|slot| period_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3);
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result = crate::indicators::china::kdj(high, low, close, n, m1, m2)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let selected = if is("CALL:KDJ_D") {
            result.d.as_slice().unwrap()
        } else if is("CALL:KDJ_J") {
            result.j.as_slice().unwrap()
        } else {
            result.k.as_slice().unwrap()
        };
        return copy_result(output, selected);
    }

    if is("CALL:ICHIMOKU_TENKAN") || is("CALL:ICHIMOKU_KIJUN") {
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let (high, low, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let upper = crate::math::statistics::rolling_max(high, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        let lower = crate::math::statistics::rolling_min(low, period)
            .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        for index in 0..len {
            output[index] = if upper[index].is_finite() && lower[index].is_finite() {
                (upper[index] + lower[index]) * 0.5
            } else {
                f64::NAN
            };
        }
        return Ok(());
    }

    if is("CALL:SUPERTREND") {
        let period = period_from_slot(buffers, call.inputs[3].0)?;
        let multiplier = call
            .inputs
            .get(4)
            .map(|slot| scalar_from_slot(buffers, slot.0))
            .transpose()?
            .unwrap_or(3.0);
        let (high, low, close, output) = unsafe {
            (
                std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
                std::slice::from_raw_parts(buffers[call.inputs[2].0].as_ptr(), len),
                std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
            )
        };
        let result =
            crate::indicators::supertrend::supertrend(high, low, close, period, multiplier)
                .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
        return copy_result(output, result.trend_line.as_slice().unwrap());
    }

    let period = period_from_slot(buffers, call.inputs[2].0)?;
    let (high, low, output) = unsafe {
        (
            std::slice::from_raw_parts(buffers[call.inputs[0].0].as_ptr(), len),
            std::slice::from_raw_parts(buffers[call.inputs[1].0].as_ptr(), len),
            std::slice::from_raw_parts_mut(buffers[output_slot].as_mut_ptr(), len),
        )
    };
    let result = crate::indicators::donchian::donchian(high, low, period)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    let selected = if is("CALL:DONCHIAN_UPPER") {
        result.upper.as_slice().unwrap()
    } else if is("CALL:DONCHIAN_LOWER") {
        result.lower.as_slice().unwrap()
    } else if is("CALL:DONCHIAN_WIDTH") {
        result.width.as_slice().unwrap()
    } else {
        result.middle.as_slice().unwrap()
    };
    copy_result(output, selected)
}

#[derive(Debug, Clone, Copy)]
enum BinaryKernel {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Max,
    Min,
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Neq,
    And,
    Or,
    Xor,
}

fn unary(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: impl Fn(f64) -> f64,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 1 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input = call.inputs[0].0;
    let output = call.output.0;
    let len = buffers[output].len();
    for index in 0..len {
        let value = buffers[input][index];
        buffers[output][index] = op(value);
    }
    Ok(())
}

fn binary(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: BinaryKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let left = call.inputs[0].0;
    let right = call.inputs[1].0;
    let output = call.output.0;
    let len = buffers[output].len();
    for index in 0..len {
        let lhs = buffers[left][index];
        let rhs = buffers[right][index];
        buffers[output][index] = apply_binary(op, lhs, rhs);
    }
    Ok(())
}

#[inline]
fn apply_binary(op: BinaryKernel, lhs: f64, rhs: f64) -> f64 {
    match op {
        BinaryKernel::Add => lhs + rhs,
        BinaryKernel::Sub => lhs - rhs,
        BinaryKernel::Mul => lhs * rhs,
        BinaryKernel::Div => {
            if rhs.abs() < 1e-15 {
                f64::NAN
            } else {
                lhs / rhs
            }
        }
        BinaryKernel::Mod => {
            if rhs.abs() < 1e-15 {
                f64::NAN
            } else {
                lhs - (lhs / rhs).floor() * rhs
            }
        }
        BinaryKernel::Pow => lhs.powf(rhs),
        // `f64::max`/`f64::min` return the non-NaN operand when exactly one side
        // is NaN, matching the reference `MAX`/`MIN` implementations.
        BinaryKernel::Max => lhs.max(rhs),
        BinaryKernel::Min => lhs.min(rhs),
        BinaryKernel::Gt => bool_value(lhs > rhs),
        BinaryKernel::Lt => bool_value(lhs < rhs),
        BinaryKernel::Gte => bool_value(lhs >= rhs),
        BinaryKernel::Lte => bool_value(lhs <= rhs),
        BinaryKernel::Eq => bool_value((lhs - rhs).abs() < 1e-10),
        BinaryKernel::Neq => bool_value((lhs - rhs).abs() >= 1e-10),
        BinaryKernel::And => bool_value(lhs > 0.0 && rhs > 0.0),
        BinaryKernel::Or => bool_value(lhs > 0.0 || rhs > 0.0),
        BinaryKernel::Xor => bool_value((lhs > 0.0) != (rhs > 0.0)),
    }
}

#[inline]
const fn bool_value(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// Construct a reusable unified executor for one compiled formula hot plan.
pub fn unified_formula_executor(
    plan: &super::hot_plan::FormulaHotPlan,
) -> UnifiedExecutor<FormulaKernelDispatcher> {
    UnifiedExecutor::new(plan.hot().clone(), FormulaKernelDispatcher::new())
}

/// [`unified_formula_executor`] carrying host data.
///
/// Callers that evaluate formulas against a `FormulaContext` should use this so
/// host-dependent functions (`WINNER`, `COST`, `PERIODTYPE`) agree with the tree
/// path instead of silently degrading to `NaN`.
pub fn unified_formula_executor_with_host(
    plan: &super::hot_plan::FormulaHotPlan,
    host: HostContext,
) -> UnifiedExecutor<FormulaKernelDispatcher> {
    UnifiedExecutor::new(plan.hot().clone(), FormulaKernelDispatcher::with_host(host))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{parse_formula, FormulaHotPlan};

    #[test]
    fn real_formula_frontend_executes_constant_arithmetic_on_unified_executor() {
        let ast = parse_formula("CLOSE * 2 + 1").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.5, -3.0, 0.0];

        let result = executor.execute(&[&close]).unwrap();
        assert_eq!(result.values, vec![vec![3.0, 6.0, -5.0, 1.0]]);
    }

    #[test]
    fn numeric_semantics_match_legacy_formula_contract() {
        let ast = parse_formula("(CLOSE / 0) + (CLOSE > 1)").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.0];

        let result = executor.execute(&[&close]).unwrap();
        assert!(result.values[0][0].is_nan());
        assert!(result.values[0][1].is_nan());
    }

    #[test]
    fn range_and_last_use_the_same_formula_dispatcher() {
        let ast = parse_formula("-CLOSE + 10").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [2.0, 4.0, 8.0];

        let range = executor.execute_range(&[&close], 0..2).unwrap();
        assert_eq!(range.values, vec![vec![8.0, 6.0]]);
        let last = executor.execute_last(&[&close]).unwrap();
        assert_eq!(last.values, vec![vec![2.0]]);
    }

    #[test]
    fn canonical_indicator_calls_run_inside_the_unified_formula_executor() {
        let ast = parse_formula("EMA(CLOSE, 3) + ROC(CLOSE, 2)").unwrap();
        let plan = FormulaHotPlan::compile(&ast).unwrap();
        let mut executor = unified_formula_executor(&plan);
        let close = [1.0, 2.0, 4.0, 8.0, 16.0];

        let result = executor.execute(&[&close]).unwrap();
        assert!(result.values[0][0].is_nan());
        assert!(result.values[0][1].is_nan());
        assert!(result.values[0][2].is_finite());
        assert!(result.values[0][4].is_finite());
    }

    #[test]
    fn extended_indicator_calls_use_canonical_zero_copy_kernels() {
        let high = [10.0, 10.5, 11.0, 10.8, 11.4, 11.8, 12.1, 11.9, 12.4, 12.7];
        let low = [9.0, 9.5, 10.0, 9.7, 10.3, 10.8, 11.0, 10.9, 11.3, 11.6];
        let close = [9.5, 10.2, 10.7, 10.1, 11.0, 11.5, 11.7, 11.2, 12.0, 12.4];
        let volume = [
            100.0, 110.0, 120.0, 105.0, 130.0, 125.0, 140.0, 115.0, 150.0, 145.0,
        ];

        fn execute(
            source: &str,
            high: &[f64],
            low: &[f64],
            close: &[f64],
            volume: &[f64],
        ) -> Vec<f64> {
            let ast = parse_formula(source).unwrap();
            let plan = FormulaHotPlan::compile(&ast).unwrap();
            let mut inputs = vec![close; plan.hot().input_layout().len()];
            for (name, values) in [
                ("VARIABLE:HIGH", high),
                ("VARIABLE:LOW", low),
                ("VARIABLE:CLOSE", close),
                ("VARIABLE:VOLUME", volume),
            ] {
                if let Some(slot) = plan.hot().input_layout().slot_for_operation(name) {
                    inputs[slot.0] = values;
                }
            }
            let mut executor = unified_formula_executor(&plan);
            executor.execute(&inputs).unwrap().values[0].clone()
        }

        let cases = [
            (
                "CCI(HIGH,LOW,CLOSE,3)",
                crate::indicators::momentum::cci(&high, &low, &close, 3).unwrap(),
            ),
            (
                "NATR(HIGH,LOW,CLOSE,3)",
                crate::indicators::volatility::natr(&high, &low, &close, 3).unwrap(),
            ),
            (
                "TRIMA(CLOSE,3)",
                crate::math::moving_avg::trima(&close, 3).unwrap(),
            ),
            (
                "AD(HIGH,LOW,CLOSE,VOLUME)",
                crate::math::volume_kernels::ad(&high, &low, &close, &volume).unwrap(),
            ),
            (
                "ADOSC(HIGH,LOW,CLOSE,VOLUME,3,5)",
                crate::math::volume_kernels::adosc(&high, &low, &close, &volume, 3, 5).unwrap(),
            ),
            (
                "MFI(HIGH,LOW,CLOSE,VOLUME,3)",
                crate::math::mfi::mfi(&high, &low, &close, &volume, 3).unwrap(),
            ),
        ];

        for (source, expected) in cases {
            let actual = execute(source, &high, &low, &close, &volume);
            assert_eq!(actual.len(), expected.len(), "{source}");
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (*actual - *expected).abs() < 1e-10,
                    "{source}: actual={actual:?} expected={expected:?}"
                );
            }
        }

        let modern_cases = [
            (
                "ZSCORE(CLOSE,3)",
                crate::indicators::statistics::zscore(&close, 3).unwrap(),
            ),
            (
                "VWMA(CLOSE,VOLUME,3)",
                crate::math::moving_avg::vwma(&close, &volume, 3).unwrap(),
            ),
            (
                "CMF(HIGH,LOW,CLOSE,VOLUME,3)",
                crate::indicators::volume_ext::cmf(&high, &low, &close, &volume, 3).unwrap(),
            ),
            (
                "FISHER(HIGH,LOW,3)",
                crate::indicators::momentum_ext::fisher(&high, &low, 3)
                    .unwrap()
                    .fisher,
            ),
            (
                "FISHER_SIGNAL(HIGH,LOW,3)",
                crate::indicators::momentum_ext::fisher(&high, &low, 3)
                    .unwrap()
                    .signal,
            ),
            (
                "TSI(CLOSE,4,2)",
                crate::indicators::momentum_ext::tsi(&close, 4, 2).unwrap(),
            ),
            (
                "CHOP(HIGH,LOW,CLOSE,3)",
                crate::indicators::momentum_ext::chop(&high, &low, &close, 3).unwrap(),
            ),
            (
                "KDJ_D(HIGH,LOW,CLOSE,3,2,2)",
                crate::indicators::china::kdj(&high, &low, &close, 3, 2, 2)
                    .unwrap()
                    .d,
            ),
            (
                "ICHIMOKU_TENKAN(HIGH,LOW,3)",
                crate::math::statistics::rolling_max(&high, 3)
                    .unwrap()
                    .iter()
                    .zip(
                        crate::math::statistics::rolling_min(&low, 3)
                            .unwrap()
                            .iter(),
                    )
                    .map(|(upper, lower)| {
                        if upper.is_finite() && lower.is_finite() {
                            (upper + lower) * 0.5
                        } else {
                            f64::NAN
                        }
                    })
                    .collect(),
            ),
            (
                "SUPERTREND(HIGH,LOW,CLOSE,3,2)",
                crate::indicators::supertrend::supertrend(&high, &low, &close, 3, 2.0)
                    .unwrap()
                    .trend_line,
            ),
            (
                "DONCHIAN_WIDTH(HIGH,LOW,3)",
                crate::indicators::donchian::donchian(&high, &low, 3)
                    .unwrap()
                    .width,
            ),
        ];

        for (source, expected) in modern_cases {
            let actual = execute(source, &high, &low, &close, &volume);
            assert_eq!(actual.len(), expected.len(), "{source}");
            for (actual, expected) in actual.iter().zip(expected.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (*actual - *expected).abs() < 1e-10,
                    "{source}: actual={actual:?} expected={expected:?}"
                );
            }
        }

        let actual = execute("SMA(CLOSE,3,1)", &high, &low, &close, &volume);
        let mut previous = None;
        for (index, &value) in close.iter().enumerate() {
            let expected = previous
                .map(|old| (value + 2.0 * old) / 3.0)
                .unwrap_or(value);
            assert!((actual[index] - expected).abs() < 1e-12);
            previous = Some(expected);
        }
    }
}
