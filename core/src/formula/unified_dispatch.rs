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
/// that host-dependent kernels (`WINNER`, `COST`, `PERIODTYPE`, `TR` and the
/// zero-argument DZH money-flow family) need and that the numeric input slots
/// cannot express.
#[derive(Clone, Default)]
pub struct FormulaKernelDispatcher<'a> {
    /// Host-side data (chip distribution, chart period, money flow, implicit
    /// OHLC) that the numeric input slots cannot carry. Empty unless the caller
    /// supplies it.
    host: HostContext<'a>,
}

impl<'a> FormulaKernelDispatcher<'a> {
    const ERR_UNSUPPORTED_KERNEL: u32 = 1;
    const ERR_ARITY: u32 = 2;
    const ERR_PARAMETER: u32 = 3;

    /// Create a dispatcher with no host context: host-dependent kernels such as
    /// `WINNER` evaluate to `NaN`, matching the tree path without chip data.
    pub const fn new() -> Self {
        Self {
            host: HostContext::new(),
        }
    }

    /// Create a dispatcher carrying host data for the host-dependent kernels.
    pub const fn with_host(host: HostContext<'a>) -> Self {
        Self { host }
    }
}

impl KernelDispatcher for FormulaKernelDispatcher<'_> {
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

        if call.kernel == KernelId::from_static("CALL:BARSCOUNT")
            || call.kernel == KernelId::from_static("CALL:BARPOS")
            || call.kernel == KernelId::from_static("CALL:CAPITAL")
            || call.kernel == KernelId::from_static("CALL:DRAWNULL")
        {
            return dispatch_context_variable_call(call, buffers, &self.host);
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
        // `LN` and `LOG` are the same function on the formula surface (`fn_log`
        // is registered under both names), so both dispatch here. The plan
        // kernel is named after the SSOT entry, `LN`.
        if call.kernel == KernelId::from_static("CALL:LN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Ln);
        }
        if call.kernel == KernelId::from_static("CALL:ACOS") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Acos);
        }
        if call.kernel == KernelId::from_static("CALL:ASIN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Asin);
        }
        if call.kernel == KernelId::from_static("CALL:ATAN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Atan);
        }
        if call.kernel == KernelId::from_static("CALL:CEIL") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Ceil);
        }
        if call.kernel == KernelId::from_static("CALL:COS") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Cos);
        }
        if call.kernel == KernelId::from_static("CALL:EXP") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Exp);
        }
        if call.kernel == KernelId::from_static("CALL:FLOOR") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Floor);
        }
        if call.kernel == KernelId::from_static("CALL:LOG10") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Log10);
        }
        if call.kernel == KernelId::from_static("CALL:SIN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Sin);
        }
        if call.kernel == KernelId::from_static("CALL:TAN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Tan);
        }
        if call.kernel == KernelId::from_static("CALL:SIGN") {
            return dispatch_unary_math_call(call, buffers, UnaryMathKernel::Sign);
        }

        if call.kernel == KernelId::from_static("CALL:CROSS") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::Cross);
        }
        if call.kernel == KernelId::from_static("CALL:CROSSBELOW") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::CrossBelow);
        }
        if call.kernel == KernelId::from_static("CALL:FIXNAN") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::FixNan);
        }
        if call.kernel == KernelId::from_static("CALL:ISNA") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::IsNa);
        }
        if call.kernel == KernelId::from_static("CALL:INTPART") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::IntPart);
        }
        if call.kernel == KernelId::from_static("CALL:FRACPART") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::FracPart);
        }
        if call.kernel == KernelId::from_static("CALL:MOD") {
            return dispatch_elementwise_call(call, buffers, ElementwiseKernel::Mod);
        }

        if call.kernel == KernelId::from_static("CALL:CUMSUM") {
            return dispatch_sequence_call(call, buffers, SequenceKernel::CumSum);
        }
        if call.kernel == KernelId::from_static("CALL:BARSSINCE") {
            return dispatch_sequence_call(call, buffers, SequenceKernel::BarsSince);
        }
        if call.kernel == KernelId::from_static("CALL:REVERSE") {
            return dispatch_sequence_call(call, buffers, SequenceKernel::Reverse);
        }
        if call.kernel == KernelId::from_static("CALL:SUMBARS") {
            return dispatch_sequence_call(call, buffers, SequenceKernel::SumBars);
        }

        // `TRANGE(H, L, C)` has no period operand, so it cannot join the HLC
        // periodic family that `ATR`/`NATR`/`CCI` use.
        if call.kernel == KernelId::from_static("CALL:TRANGE") {
            return dispatch_trange_call(call, buffers);
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

        if call.kernel == KernelId::from_static("CALL:LINEARREG_SLOPE") {
            return dispatch_linear_reg_slope_call(call, buffers);
        }
        if call.kernel == KernelId::from_static("CALL:QUANTILE") {
            return dispatch_reference_rolling_call(
                call,
                buffers,
                ReferenceRollingKernel::Quantile,
            );
        }
        if call.kernel == KernelId::from_static("CALL:RSQUARE") {
            return dispatch_reference_rolling_call(
                call,
                buffers,
                ReferenceRollingKernel::RSquared,
            );
        }
        if call.kernel == KernelId::from_static("CALL:RESI") {
            return dispatch_reference_rolling_call(
                call,
                buffers,
                ReferenceRollingKernel::Residual,
            );
        }
        if call.kernel == KernelId::from_static("CALL:RANK_PCT") {
            return dispatch_reference_rolling_call(call, buffers, ReferenceRollingKernel::RankPct);
        }
        if call.kernel == KernelId::from_static("CALL:STDDEV_SAMPLE") {
            return dispatch_reference_rolling_call(
                call,
                buffers,
                ReferenceRollingKernel::StdSample,
            );
        }

        if call.kernel == KernelId::from_static("CALL:MA")
            || call.kernel == KernelId::from_static("CALL:SMA")
            || call.kernel == KernelId::from_static("CALL:EMA")
            || call.kernel == KernelId::from_static("CALL:WMA")
            || call.kernel == KernelId::from_static("CALL:HMA")
            || call.kernel == KernelId::from_static("CALL:RMA")
            || call.kernel == KernelId::from_static("CALL:MEDIAN")
            || call.kernel == KernelId::from_static("CALL:ROLLING_RANGE")
            || call.kernel == KernelId::from_static("CALL:KAMA")
            || call.kernel == KernelId::from_static("CALL:RSI")
            || call.kernel == KernelId::from_static("CALL:MOM")
            || call.kernel == KernelId::from_static("CALL:ROC")
            || call.kernel == KernelId::from_static("CALL:TRIX")
            || call.kernel == KernelId::from_static("CALL:TRIMA")
            // `STDDEV` is an alias of `STD` on the formula surface (`fn_std` is
            // registered under both names), so it shares the branch rather than
            // getting a kernel of its own.
            || call.kernel == KernelId::from_static("CALL:STD")
            || call.kernel == KernelId::from_static("CALL:STDDEV")
            || call.kernel == KernelId::from_static("CALL:HHV")
            || call.kernel == KernelId::from_static("CALL:LLV")
            || call.kernel == KernelId::from_static("CALL:SUM")
            || call.kernel == KernelId::from_static("CALL:VAR")
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
        // `TR()` reads the implicit OHLC rather than taking it as operands, so
        // it needs the host channel too. It deliberately does *not* share the
        // `TRANGE` kernel: see `dispatch_tr_call`.
        if call.kernel == KernelId::from_static("CALL:TR") {
            return dispatch_tr_call(&self.host, call, buffers);
        }
        if let Some(selector) = money_flow_kernel(call.kernel) {
            return dispatch_money_flow_call(&self.host, call, buffers, selector);
        }
        // `REFDATE` needs no host data — it reads a scalar index out of its
        // second operand — but it must exist as a kernel or the plan path cannot
        // run the cross-period corpus at all.
        if call.kernel == KernelId::from_static("CALL:REFDATE") {
            return dispatch_refdate_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("DRAW_GENERIC")
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
            || call.kernel == KernelId::from_static("CALL:CORREL")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_UPPER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_LOWER")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_MIDDLE")
            || call.kernel == KernelId::from_static("CALL:DONCHIAN_WIDTH")
        {
            return dispatch_modern_call(call, buffers);
        }

        // TA-Lib 0.7/0.8 formula bridge (M0-1 surface). These names are
        // callable on the tree/bytecode/JIT paths but had no `CALL:<NAME>`
        // plan kernel; `dispatch_modern_call` delegates each to the exact
        // `canonical_*` implementation, so the compiled plan path emits
        // byte-identical output (zero divergence by construction).
        if call.kernel == KernelId::from_static("CALL:AC")
            || call.kernel == KernelId::from_static("CALL:ACCBANDS")
            || call.kernel == KernelId::from_static("CALL:ACCBANDS_MID")
            || call.kernel == KernelId::from_static("CALL:ACCBANDS_LOWER")
            || call.kernel == KernelId::from_static("CALL:ADR")
            || call.kernel == KernelId::from_static("CALL:AO")
            || call.kernel == KernelId::from_static("CALL:AROON")
            || call.kernel == KernelId::from_static("CALL:AROON_DOWN")
            || call.kernel == KernelId::from_static("CALL:CMOU")
            || call.kernel == KernelId::from_static("CALL:COPPOCK")
            || call.kernel == KernelId::from_static("CALL:CVI")
            || call.kernel == KernelId::from_static("CALL:EFI")
            || call.kernel == KernelId::from_static("CALL:ER")
            || call.kernel == KernelId::from_static("CALL:ERI")
            || call.kernel == KernelId::from_static("CALL:ERI_BEAR")
            || call.kernel == KernelId::from_static("CALL:FOSC")
            || call.kernel == KernelId::from_static("CALL:FRACTAL")
            || call.kernel == KernelId::from_static("CALL:FRACTAL_LOW")
            || call.kernel == KernelId::from_static("CALL:HA")
            || call.kernel == KernelId::from_static("CALL:HA_OPEN")
            || call.kernel == KernelId::from_static("CALL:HA_HIGH")
            || call.kernel == KernelId::from_static("CALL:HA_LOW")
            || call.kernel == KernelId::from_static("CALL:KC")
            || call.kernel == KernelId::from_static("CALL:KC_MID")
            || call.kernel == KernelId::from_static("CALL:KC_LOWER")
            || call.kernel == KernelId::from_static("CALL:MAMA")
            || call.kernel == KernelId::from_static("CALL:MAMA_FAMA")
            || call.kernel == KernelId::from_static("CALL:MARKETFI")
            || call.kernel == KernelId::from_static("CALL:MASSI")
            || call.kernel == KernelId::from_static("CALL:NVI")
            || call.kernel == KernelId::from_static("CALL:PERCENTRANK")
            || call.kernel == KernelId::from_static("CALL:PVI")
            || call.kernel == KernelId::from_static("CALL:PVO")
            || call.kernel == KernelId::from_static("CALL:PVT")
            || call.kernel == KernelId::from_static("CALL:QSTICK")
            || call.kernel == KernelId::from_static("CALL:RVI")
            || call.kernel == KernelId::from_static("CALL:RVOL")
            || call.kernel == KernelId::from_static("CALL:SMI")
            || call.kernel == KernelId::from_static("CALL:SMI_SIGNAL")
            || call.kernel == KernelId::from_static("CALL:VHF")
            || call.kernel == KernelId::from_static("CALL:VORTEX")
            || call.kernel == KernelId::from_static("CALL:VORTEX_MINUS")
            || call.kernel == KernelId::from_static("CALL:WAD")
            || call.kernel == KernelId::from_static("CALL:ZLEMA")
        {
            return dispatch_modern_call(call, buffers);
        }

        if TALIB_FORMULA_GAP_KERNELS
            .iter()
            .any(|name| call.kernel == KernelId::from_static(name))
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

/// Absorb a caller-owned kernel's failure into an all-NaN output.
///
/// A period that does not fit the data is an **edge case the reference path
/// answers with values, not an error**. Two families on the tree path behave
/// that way, and both are mirrored here:
///
/// - the legacy wrappers — `fn_ma`, `fn_ema`, `fn_wma`, `fn_hma`, `fn_rma`,
///   `fn_median`, `fn_rolling_range`, `fn_rsi`, `fn_mom`, `fn_trima`, `fn_trix`,
///   `fn_hhv`, `fn_llv`, `fn_sum`, `fn_var`, `fn_macd`, `fn_dea` — end with
///   `match lib_x(..) { Ok(result) => Ok(result), Err(_) => Ok(nan_vec(data_len)) }`;
/// - the `canonical_*` overrides that win for the multi-input indicators —
///   `canonical_atr`, `canonical_natr`, `canonical_bband_component` (behind
///   `BOLL`/`BOLLUP`/`BOLLDN`/`BOLLMID`/`BBANDS`), `canonical_adosc`,
///   `canonical_mfi`, `canonical_ad`, `canonical_obv`, `canonical_trange` —
///   answer `Ok(nan_vec(..))` the same way.
///
/// `MA(CLOSE, 100)` over ten bars therefore yields **ten NaN values**, never an
/// error, and a one-bar input behaves the same.
///
/// The compiled-plan path used to propagate the underlying `TaError` as
/// `ERR_PARAMETER` and fail the whole evaluation. That is a cross-path
/// divergence, not a stricter contract — switching
/// `FormulaEngine::set_execution_mode` turned a NaN result into a hard error —
/// so this reproduces the reference series instead. Same policy as
/// [`rate_of_change_into`] and [`sum_formula_into`].
///
/// Only the *kernel's own* failure is absorbed, and only where a **period
/// parameter** exists. Arity, aliasing and buffer-length checks stay errors: the
/// tree path cannot reach them (its arity check runs first and it owns its
/// buffers), so absorbing those would hide caller bugs rather than mirror a
/// documented behaviour. Period-*less* kernels (`TRANGE`, `TR`, `OBV`) are
/// deliberately left propagating for the same reason — their only failure mode
/// is a length mismatch the dispatcher already validates explicitly before the
/// call, so absorbing them would be a no-op that weakens the rule.
///
/// A **zero period is not absorbed**: both paths reject it at their own guard
/// (`extract_n` on the tree path, `period_from_slot` here) *before* any kernel
/// runs, so `absorb_kernel_failure` can never see it.
///
/// `CALL:REF` is deliberately **not** routed through here: `fn_ref` is the one
/// function in this family that does not swallow its error, so the dispatcher
/// must keep failing with it.
fn absorb_kernel_failure<E>(
    result: Result<(), E>,
    output: &mut [f64],
) -> Result<(), KernelDispatchError> {
    match result {
        Ok(()) => Ok(()),
        Err(_) => {
            output.fill(f64::NAN);
            Ok(())
        }
    }
}

/// The same policy for kernels that *return* a freshly built series instead of
/// writing through an output slice (`bbands`, `aroon`, `willr`, `zscore`,
/// `vwma`, `cmf`, `chop`, `fisher`, `tsi`, `kdj`, `supertrend`, `donchian`).
///
/// Two helpers exist because the two kernel shapes need different plumbing for
/// one rule. Returning `None` after filling the output with NaN lets the caller
/// write `let Some(x) = ... else { return Ok(()) };`, which keeps the
/// "degenerate period is data, not an error" decision in one place.
///
/// It is deliberately *not* a general-purpose error swallow: callers must have
/// already validated arity, aliasing and buffer lengths, so the only failure
/// left is the kernel's own `InsufficientData`. See [`absorb_kernel_failure`].
fn absorb_kernel_series<T>(result: crate::error::Result<T>, output: &mut [f64]) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(_) => {
            output.fill(f64::NAN);
            None
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
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
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

/// Non-periodic element-wise operations behind `CROSS` and `FIXNAN`.
///
/// Neither takes a period, so neither can ride the periodic kernel: `CROSS`
/// compares two series and `FIXNAN` transforms one. Both reproduce their `fn_*`
/// counterparts exactly, including the details that look like oversights but are
/// the contract:
///
/// - `FIXNAN` forward-fills and **keeps leading NaN**. It does not seed from the
///   first finite value and does not zero-fill; that is what makes it the Pine
///   `fixnan` contract rather than a generic carry-forward.
/// - `CROSS` yields `0.0` — not NaN — wherever nothing crossed, and a NaN operand
///   compares false, so it never crosses.
/// - `ISNA` yields `1.0`/`0.0`, never NaN: it is a *predicate*, so a NaN input
///   is a `1.0` result rather than a propagated gap. Pine's `na(x)` and the
///   first half of `nz(x, y)` depend on that.
/// - `MOD` is **not** the `%` operator and not `BINARY:Mod`. `fn_mod` uses
///   Rust's truncating `%` and yields NaN when the divisor is near zero, while
///   `BINARY:Mod` uses a floor-based remainder. The two disagree in sign for
///   negative operands (`MOD(-7, 3)` is `-1`, `-7 % 3` as an operator is `2`),
///   so reusing the operator kernel here would silently change results.
enum ElementwiseKernel {
    /// `CROSS(A, B)`: `1.0` on the bar A crosses above B.
    Cross,
    /// `CROSSBELOW(A, B)`: `1.0` on the bar A crosses below B.
    CrossBelow,
    /// `FIXNAN(X)`: forward-fill missing values, preserving leading NaN.
    FixNan,
    /// `ISNA(X)`: `1.0` where `X` is NaN, else `0.0`.
    IsNa,
    /// `INTPART(X)`: truncate toward zero.
    IntPart,
    /// `FRACPART(X)`: the fractional part, `X.fract()`.
    FracPart,
    /// `MOD(A, B)`: truncating remainder, NaN where `|B| <= 1e-15`.
    Mod,
}

fn dispatch_elementwise_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: ElementwiseKernel,
) -> Result<(), KernelDispatchError> {
    let expected = match op {
        ElementwiseKernel::Cross | ElementwiseKernel::CrossBelow | ElementwiseKernel::Mod => 2,
        ElementwiseKernel::FixNan
        | ElementwiseKernel::IsNa
        | ElementwiseKernel::IntPart
        | ElementwiseKernel::FracPart => 1,
    };
    if call.inputs.len() != expected {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let out = call.output.0;
    let length = buffers[out].len();

    match op {
        ElementwiseKernel::IntPart => {
            let input = call.inputs[0].0;
            for index in 0..length {
                let value = buffers[input][index];
                buffers[out][index] = value.trunc();
            }
        }
        ElementwiseKernel::FracPart => {
            let input = call.inputs[0].0;
            for index in 0..length {
                let value = buffers[input][index];
                buffers[out][index] = value.fract();
            }
        }
        ElementwiseKernel::Mod => {
            let lhs = call.inputs[0].0;
            let rhs = call.inputs[1].0;
            for index in 0..length {
                let divisor = buffers[rhs][index];
                let value = buffers[lhs][index];
                buffers[out][index] = if divisor.abs() > 1e-15 {
                    value % divisor
                } else {
                    f64::NAN
                };
            }
        }
        ElementwiseKernel::IsNa => {
            let input = call.inputs[0].0;
            for index in 0..length {
                // Read before writing, for the same aliasing reason as `FixNan`.
                let value = buffers[input][index];
                buffers[out][index] = if value.is_nan() { 1.0 } else { 0.0 };
            }
        }
        ElementwiseKernel::FixNan => {
            let input = call.inputs[0].0;
            let mut previous = f64::NAN;
            for index in 0..length {
                // Read before writing: the allocator may alias the output onto
                // the operand, and this stays correct because the read and the
                // write are at the same index.
                let value = buffers[input][index];
                if !value.is_nan() {
                    previous = value;
                }
                buffers[out][index] = previous;
            }
        }
        ElementwiseKernel::Cross | ElementwiseKernel::CrossBelow => {
            if length == 0 {
                return Ok(());
            }
            let above = matches!(op, ElementwiseKernel::Cross);
            let lhs = call.inputs[0].0;
            let rhs = call.inputs[1].0;
            // Bar 0 has no predecessor to compare against, so it can never be a
            // crossing. `fn_cross`/`fn_crossbelow` initialise the series to zeros
            // for the same reason, which also means a NaN operand yields 0.0
            // rather than NaN.
            let mut previous_lhs = buffers[lhs][0];
            let mut previous_rhs = buffers[rhs][0];
            buffers[out][0] = 0.0;
            for index in 1..length {
                let lhs_now = buffers[lhs][index];
                let rhs_now = buffers[rhs][index];
                let was_below = previous_lhs <= previous_rhs;
                let was_above = previous_lhs >= previous_rhs;
                let crossed = if above {
                    was_below && lhs_now > rhs_now
                } else {
                    was_above && lhs_now < rhs_now
                };
                buffers[out][index] = if crossed { 1.0 } else { 0.0 };
                // Carry the values read above rather than re-reading `index - 1`:
                // that slot may already have been overwritten if the output
                // aliases one of the operands.
                previous_lhs = lhs_now;
                previous_rhs = rhs_now;
            }
        }
    }
    Ok(())
}

/// Whole-series scans behind `CUMSUM` and `BARSSINCE`.
///
/// Both are single-input, windowless accumulations, so they cannot join the
/// periodic family (which demands a period operand) or the elementwise family
/// (which is stateless per bar). Their warm-up rules differ and are the contract:
///
/// - `CUMSUM` writes a value from bar 0 — its accumulator starts at `0.0`, so
///   leading NaN contributes nothing rather than poisoning the prefix.
/// - `BARSSINCE` leaves every bar before the first truthy input as NaN, then
///   counts bars since the most recent truthy one (so the truthy bar itself is
///   `0.0`). A NaN condition is not truthy, matching `fn_barssince`.
/// - `REVERSE` mirrors the whole series, so it is an O(n) scan that needs the
///   *final* length up front rather than a running accumulator.
/// - `SUMBARS` is the one two-operand member: it scans *backwards* from each bar
///   until the running total reaches that bar's target, and reports how many
///   bars that took. A target that is never reached yields the distance to the
///   start of the series, not NaN.
enum SequenceKernel {
    /// `CUMSUM(X)`: running sum, skipping NaN.
    CumSum,
    /// `BARSSINCE(X)`: bars since `X` was last non-zero and non-NaN.
    BarsSince,
    /// `REVERSE(X)`: the series mirrored end to end.
    Reverse,
    /// `SUMBARS(X, T)`: bars back from each bar until the sum reaches `T`.
    SumBars,
}

fn dispatch_sequence_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: SequenceKernel,
) -> Result<(), KernelDispatchError> {
    let expected = match op {
        SequenceKernel::CumSum | SequenceKernel::BarsSince | SequenceKernel::Reverse => 1,
        SequenceKernel::SumBars => 2,
    };
    if call.inputs.len() != expected {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let input = call.inputs[0].0;
    let out = call.output.0;
    let length = buffers[out].len();
    if call
        .inputs
        .iter()
        .any(|slot| slot.0 == out || buffers[slot.0].len() != length)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    match op {
        SequenceKernel::CumSum => {
            let mut sum = 0.0;
            for index in 0..length {
                // Read before writing: the output may alias the input, and the
                // read and write are at the same index, so this stays correct.
                let value = buffers[input][index];
                if !value.is_nan() {
                    sum += value;
                }
                buffers[out][index] = sum;
            }
        }
        SequenceKernel::BarsSince => {
            let mut last_true: Option<usize> = None;
            for index in 0..length {
                let value = buffers[input][index];
                if value != 0.0 && !value.is_nan() {
                    last_true = Some(index);
                }
                buffers[out][index] = match last_true {
                    Some(position) => (index - position) as f64,
                    None => f64::NAN,
                };
            }
        }
        SequenceKernel::Reverse => {
            // Mirrored indices are read and written in opposite order, so an
            // aliased output would clobber values it has not read yet. Snapshot
            // the source instead of relying on the read/write symmetry the
            // single-index kernels get for free.
            let source: Vec<f64> = buffers[input].clone();
            for index in 0..length {
                buffers[out][index] = source[length - 1 - index];
            }
        }
        SequenceKernel::SumBars => {
            let target = call.inputs[1].0;
            let source: Vec<f64> = buffers[input].clone();
            for index in 0..length {
                let threshold = buffers[target][index];
                let mut cumulative = 0.0;
                let mut bars = 0.0;
                for position in (0..=index).rev() {
                    cumulative += source[position];
                    bars += 1.0;
                    if cumulative >= threshold {
                        break;
                    }
                }
                buffers[out][index] = bars;
            }
        }
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

/// Single-argument maths behind `SQRT` / `SINH` / `COSH` / `TANH` / `LN`.
///
/// `SQRT` reproduces `fn_sqrt`'s negative guard rather than calling `sqrt`
/// unconditionally: the tree path maps `v < 0.0` to NaN, and a bare `sqrt` would
/// match that anyway, but the guard is stated so the two stay tied together.
///
/// `Ln` likewise reproduces `fn_log`'s non-positive guard instead of relying on
/// `f64::ln`'s own `-inf`/`NaN` behaviour, so the two paths agree on `0.0` and on
/// negatives rather than merely on positives.
enum UnaryMathKernel {
    Sqrt,
    Sinh,
    Cosh,
    Tanh,
    Ln,
    Acos,
    Asin,
    Atan,
    Ceil,
    Cos,
    Exp,
    Floor,
    Log10,
    Sin,
    Tan,
    /// `sign(x)` — `-1`, `0` or `1`. Needed by the `WorldQuant` alphas, which
    /// use it to re-attach the direction of a differenced series.
    ///
    /// Not `f64::signum`: see `math::three_way_sign`.
    Sign,
}

fn dispatch_unary_math_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: UnaryMathKernel,
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 1 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
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
            UnaryMathKernel::Ln => {
                if value <= 0.0 {
                    f64::NAN
                } else {
                    value.ln()
                }
            }
            UnaryMathKernel::Acos => value.acos(),
            UnaryMathKernel::Asin => value.asin(),
            UnaryMathKernel::Atan => value.atan(),
            UnaryMathKernel::Ceil => value.ceil(),
            UnaryMathKernel::Cos => value.cos(),
            UnaryMathKernel::Exp => value.exp(),
            UnaryMathKernel::Floor => value.floor(),
            UnaryMathKernel::Log10 => {
                if value <= 0.0 {
                    f64::NAN
                } else {
                    value.log10()
                }
            }
            UnaryMathKernel::Sin => value.sin(),
            UnaryMathKernel::Tan => value.tan(),
            // `f64::signum` is not the mathematical sign: it maps `0.0` and
            // `-0.0` to `1.0` and `-1.0` respectively, and propagates `NaN`.
            // The three-way form is what the alphas mean, and it keeps
            // `sign(0) == 0` so a flat window does not acquire a direction.
            // Shared with the formula table and the stateful stream so the
            // three implementations cannot drift; see `math::three_way_sign`.
            UnaryMathKernel::Sign => crate::math::three_way_sign(value),
        };
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)] // a bar index is exact for practical series lengths.
fn dispatch_context_variable_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    host: &HostContext<'_>,
) -> Result<(), KernelDispatchError> {
    if !call.inputs.is_empty() {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let output = buffers
        .get_mut(call.output.0)
        .ok_or_else(|| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))?;
    let len = output.len();
    if call.kernel == KernelId::from_static("CALL:BARSCOUNT") {
        output.fill(len as f64);
    } else if call.kernel == KernelId::from_static("CALL:BARPOS") {
        for (index, value) in output.iter_mut().enumerate() {
            *value = (index + 1) as f64;
        }
    } else if call.kernel == KernelId::from_static("CALL:CAPITAL") {
        output.fill(host.capital.unwrap_or(f64::NAN));
    } else {
        output.fill(f64::NAN);
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
        return absorb_kernel_failure(
            crate::math::kernels::rolling_max_into(input, period, output),
            output,
        );
    }
    if call.kernel == KernelId::from_static("CALL:LLV") {
        return absorb_kernel_failure(
            crate::math::kernels::rolling_min_into(input, period, output),
            output,
        );
    }
    if call.kernel == KernelId::from_static("CALL:SUM") {
        return absorb_kernel_failure(sum_formula_into(input, period, output), output);
    }
    // `VAR` is the *population* variance on every other path -- the formula
    // surface resolves it to `indicators::statistics::var`, which is
    // `math::rolling_stats::variance`, and the streaming engine agrees. It is
    // deliberately not `STD * STD`: squaring `stddev_into`'s output would
    // reintroduce a rounding step between the two paths. (`fn_var` in
    // `functions_legacy.rs` uses the *sample* variance, but it is shadowed by
    // the router and is not what any path executes.)
    if call.kernel == KernelId::from_static("CALL:VAR") {
        return absorb_kernel_failure(
            crate::math::rolling_stats::variance_into(input, period, output),
            output,
        );
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
    } else if call.kernel == KernelId::from_static("CALL:HMA") {
        // No `hma_into` exists: the Hull MA is a three-pass WMA that builds the
        // intermediate `2*WMA(n/2) - WMA(n)` series itself, so the tree path's
        // `moving_avg::hma` is the only implementation to delegate to. Writing
        // through `_into` here would mean re-deriving that algorithm in the
        // dispatcher, which is exactly how the two paths drift apart.
        crate::math::moving_avg::hma(input, period).map(|series| {
            output.copy_from_slice(series.as_slice().expect("Array1 is contiguous"));
        })
    } else if call.kernel == KernelId::from_static("CALL:KAMA") {
        crate::math::moving_avg::kama_into(input, period, 2, 30, output)
    } else if call.kernel == KernelId::from_static("CALL:RMA") {
        // Wilder's smoothing, shared with `fn_rma` via the math layer rather
        // than re-derived here: the NaN-skipping seed is exactly the kind of
        // detail that drifts when it is written twice.
        crate::math::moving_avg::rma_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:MEDIAN") {
        crate::math::statistics::rolling_median_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:ROLLING_RANGE") {
        crate::math::statistics::rolling_range_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:RSI") {
        crate::indicators::rsi_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:MOM") {
        crate::indicators::mom_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:TRIMA") {
        crate::math::moving_avg::trima_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:TRIX") {
        crate::indicators::momentum::trix_into(input, period, output)
    } else if call.kernel == KernelId::from_static("CALL:STD")
        || call.kernel == KernelId::from_static("CALL:STDDEV")
    {
        crate::math::rolling_stats::stddev_into(input, period, 1.0, output)
    } else {
        crate::indicators::roc_into(input, period, output)
    };

    // A degenerate period (longer than the series, or zero) must produce the
    // reference path's all-NaN series rather than failing the evaluation -- see
    // `absorb_kernel_failure`.
    absorb_kernel_failure(result, output)
}

/// Rolling operators whose numeric contract is pinned to an **external
/// reference implementation** rather than to a domestic/TALib spelling.
///
/// They exist because Qlib's Alpha158 factor set needs them and nothing else in
/// the formula surface covers the same quantity:
///
/// | kernel      | reference                    | shared math kernel                                    |
/// | ----------- | ---------------------------- | ----------------------------------------------------- |
/// | `QUANTILE`  | Qlib `Quantile`              | [`crate::math::quantile::rolling_quantile_into`]       |
/// | `RSQUARE`   | Qlib `Rsquare`               | [`crate::math::regression::rolling_rsquare_into`]      |
/// | `RESI`      | Qlib `Resi`                  | [`crate::math::regression::rolling_resi_into`]         |
/// | `RANK_PCT`  | Qlib `Rank`                  | [`crate::math::rank::rolling_rank_pct_into`]           |
/// | `STDDEV_SAMPLE` | pandas `rolling(N).std()` | [`crate::math::rolling_stats::stddev_sample_into`]     |
///
/// Each arm calls the *same* `math::` function the formula-language
/// implementation calls, so the tree and plan paths agree by construction. That
/// is the whole point of routing them through one kernel instead of writing the
/// rolling loop twice.
///
/// `STDDEV_SAMPLE` is the odd one out: the operator is not missing, the
/// *convention* is. finkit's `STD`/`STDDEV` are TA-Lib's population standard
/// deviation (`m2 / n`); pandas and Qlib divide by `n - 1`. The gap is the exact
/// factor `sqrt((n - 1) / n)` — a ~10% scale error at `n = 5` — so a separate
/// name is the only honest way to have both, and Alpha158's `Std` needs the
/// sample one.
enum ReferenceRollingKernel {
    Quantile,
    RSquared,
    Residual,
    RankPct,
    StdSample,
}

/// Execute a [`ReferenceRollingKernel`] into the plan-owned output buffer.
///
/// `KernelCall` is taken by value, matching every other `dispatch_*_call` in
/// this file: the struct is a small bundle of borrows, and taking it by
/// reference would make the ~25 dispatcher arms inconsistent for no gain.
#[allow(clippy::needless_pass_by_value)]
fn dispatch_reference_rolling_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    op: ReferenceRollingKernel,
) -> Result<(), KernelDispatchError> {
    // `QUANTILE` is the only three-operand member: `(series, period, qscore)`.
    let is_quantile = matches!(op, ReferenceRollingKernel::Quantile);
    let expected = if is_quantile { 3 } else { 2 };
    if call.inputs.len() != expected {
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
    // Read every scalar operand before taking raw pointers below, so the
    // borrows used to decode the qscore cannot overlap the aliasing.
    let qscore = if is_quantile {
        scalar_from_slot(buffers, call.inputs[2].0)?
    } else {
        0.5
    };
    let input_len = buffers[input_slot].len();
    let output_len = buffers[output_slot].len();
    if input_len != output_len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let input_ptr = buffers[input_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    // The executor guarantees live dependencies do not alias the output slot,
    // and the slot inequality was checked above, so the two slices address
    // different allocations and cannot overlap.
    let (input, output) = unsafe {
        (
            std::slice::from_raw_parts(input_ptr, input_len),
            std::slice::from_raw_parts_mut(output_ptr, output_len),
        )
    };

    let result = match op {
        ReferenceRollingKernel::Quantile => crate::math::quantile::rolling_quantile_into(
            input,
            period,
            qscore,
            crate::math::quantile::QuantileInterpolation::Linear,
            output,
        ),
        ReferenceRollingKernel::RSquared => {
            crate::math::regression::rolling_rsquare_into(input, period, output)
        }
        ReferenceRollingKernel::Residual => {
            crate::math::regression::rolling_resi_into(input, period, output)
        }
        ReferenceRollingKernel::RankPct => {
            crate::math::rank::rolling_rank_pct_into(input, period, output)
        }
        ReferenceRollingKernel::StdSample => {
            crate::math::rolling_stats::stddev_sample_into(input, period, output)
        }
    };

    // A rejected argument is a hard error, matching `dispatch_periodic_call`'s
    // contract for the period slot; a *valid* period longer than the series is
    // not an error at all -- the kernels leave the output all-NaN, which is what
    // the reference path produces too.
    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
}

/// Execute `LINEARREG_SLOPE(series, period)` into the plan-owned output.
///
/// Delegates to [`crate::math::linear::linreg_slope`], which is already the
/// single implementation behind `fn_linear_reg_slope`, so the tree path and this
/// kernel cannot drift. The function returns a freshly allocated series rather
/// than writing through a slice, so the result is copied in — the same shape as
/// the `HMA` arm of [`dispatch_periodic_call`].
///
/// On a rejected period the reference path emits an all-NaN series instead of
/// failing (`fn_linear_reg_slope` maps every error to `nan_vec`), and this
/// kernel mirrors that rather than surfacing a hard error.
fn dispatch_linear_reg_slope_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 2 {
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
    let output_len = buffers[output_slot].len();
    if buffers[input_slot].len() != output_len {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let Ok(series) = crate::math::linear::linreg_slope(&buffers[input_slot], period) else {
        buffers[output_slot].fill(f64::NAN);
        return Ok(());
    };
    let source = series.as_slice().expect("Array1 is contiguous");
    let output = &mut buffers[output_slot];
    let written = source.len().min(output.len());
    output[..written].copy_from_slice(&source[..written]);
    if written < output.len() {
        output[written..].fill(f64::NAN);
    }
    Ok(())
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
    // See `math::leading_warmup`: a leading non-finite run is the warm-up prefix
    // of an upstream rolling indicator, not data. The running total below is an
    // incremental accumulator, so feeding it that run would make it `NaN` for
    // the whole series -- and `SUM` has no math-layer counterpart, so the rule
    // has to be applied here and in `functions_legacy::fn_sum`.
    let start = crate::math::leading_warmup(input);
    let mut running = 0.0;
    for (index, &current) in input.iter().enumerate().skip(start) {
        running += current;
        if index >= start + period {
            running -= input[index - period];
            output[index] = running;
        } else if index == start + period - 1 {
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
        if crate::indicators::momentum::cci_source_into(source, period, &mut cci).is_err() {
            // Degenerate period: the tree path yields an all-NaN series.
            cci.fill(f64::NAN);
        }
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

/// Execute `TRANGE(H, L, C)` into the plan-owned output.
///
/// Delegates to `indicators::volatility::trange_into`, which is what the
/// formula surface's `canonical_trange` override calls — **not** the legacy
/// `fn_trange` in `functions_legacy.rs`. The two disagree on bar 0: TA-Lib has
/// no previous close there, so the canonical implementation writes NaN, while
/// the legacy copy writes `high[0] - low[0]`. The router shadows the legacy
/// copy, so NaN is what the tree path actually produces, and matching the
/// shadowed copy here would make the two paths diverge on the first bar.
fn dispatch_trange_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if call.inputs.len() != 3 {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let high_slot = call.inputs[0].0;
    let low_slot = call.inputs[1].0;
    let close_slot = call.inputs[2].0;
    let output_slot = call.output.0;
    if [high_slot, low_slot, close_slot]
        .into_iter()
        .any(|slot| slot == output_slot)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }
    let len = buffers[output_slot].len();
    if [high_slot, low_slot, close_slot]
        .into_iter()
        .any(|slot| buffers[slot].len() != len)
    {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_PARAMETER,
        ));
    }

    let high_ptr = buffers[high_slot].as_ptr();
    let low_ptr = buffers[low_slot].as_ptr();
    let close_ptr = buffers[close_slot].as_ptr();
    let output_ptr = buffers[output_slot].as_mut_ptr();
    // All four slots were checked to be distinct above, so the aliasing this
    // requires is exactly the aliasing the dispatcher already forbids.
    let (high, low, close, output) = unsafe {
        (
            std::slice::from_raw_parts(high_ptr, len),
            std::slice::from_raw_parts(low_ptr, len),
            std::slice::from_raw_parts(close_ptr, len),
            std::slice::from_raw_parts_mut(output_ptr, len),
        )
    };
    crate::indicators::volatility::trange_into(high, low, close, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
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
    absorb_kernel_failure(result, output)
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
    host: &HostContext<'_>,
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
    host: &HostContext<'_>,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !call.inputs.is_empty() {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    buffers[call.output.0].fill(host.period_type as f64);
    Ok(())
}

/// Selector for the zero-argument DZH money-flow readers.
///
/// All seven are the same shape — hand back one host array when it is the right
/// length, `NaN` otherwise — so they share one dispatcher body. Keeping them as
/// one enum rather than seven near-identical functions is what makes the shared
/// length guard the single place that can drift.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MoneyFlowKernel {
    /// `MONEYFLOW()`
    MoneyFlow,
    /// `MAININFLOW()`
    MainInflow,
    /// `MAININFLOWPCT()`
    MainInflowPct,
    /// `BIGORDER()`
    BigOrder,
    /// `SMALLORDER()`
    SmallOrder,
    /// `SUPERBIGORDER()`
    SuperBigOrder,
    /// `NETINFLOW([level])`
    NetInflow,
}

/// Resolve a `CALL:<name>` kernel to its money-flow selector.
fn money_flow_kernel(kernel: KernelId) -> Option<MoneyFlowKernel> {
    if kernel == KernelId::from_static("CALL:MONEYFLOW") {
        return Some(MoneyFlowKernel::MoneyFlow);
    }
    if kernel == KernelId::from_static("CALL:MAININFLOW") {
        return Some(MoneyFlowKernel::MainInflow);
    }
    if kernel == KernelId::from_static("CALL:MAININFLOWPCT") {
        return Some(MoneyFlowKernel::MainInflowPct);
    }
    if kernel == KernelId::from_static("CALL:BIGORDER") {
        return Some(MoneyFlowKernel::BigOrder);
    }
    if kernel == KernelId::from_static("CALL:SMALLORDER") {
        return Some(MoneyFlowKernel::SmallOrder);
    }
    if kernel == KernelId::from_static("CALL:SUPERBIGORDER") {
        return Some(MoneyFlowKernel::SuperBigOrder);
    }
    if kernel == KernelId::from_static("CALL:NETINFLOW") {
        return Some(MoneyFlowKernel::NetInflow);
    }
    None
}

/// The zero-argument DZH money-flow family, read out of [`HostContext`].
///
/// These mirror the `fn_*` family in `functions_legacy.rs` exactly, including
/// the length guard: a host series whose length does not match the output is
/// treated as *absent* (all `NaN`) rather than broadcast. Without that guard a
/// shorter series would silently leave stale buffer contents behind, so the
/// guard is part of the contract rather than an optimisation.
///
/// `NETINFLOW` is the only one taking an operand — an optional level selecting
/// the order-size bucket — and it reads that level from the first element of its
/// input, the same way the tree path reads `args[0][0]`.
fn dispatch_money_flow_call(
    host: &HostContext<'_>,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    selector: MoneyFlowKernel,
) -> Result<(), KernelDispatchError> {
    // `NETINFLOW` accepts an omitted level; every other member is strictly
    // zero-argument.
    let arity_ok = if selector == MoneyFlowKernel::NetInflow {
        call.inputs.len() <= 1
    } else {
        call.inputs.is_empty()
    };
    if !arity_ok {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }

    let output = call.output.0;
    let Some(money_flow) = host.money_flow else {
        buffers[output].fill(f64::NAN);
        return Ok(());
    };

    let source = match selector {
        MoneyFlowKernel::MoneyFlow => &money_flow.money_flow,
        MoneyFlowKernel::MainInflow => &money_flow.main_inflow,
        MoneyFlowKernel::MainInflowPct => &money_flow.main_inflow_pct,
        MoneyFlowKernel::BigOrder => &money_flow.big_order_pct,
        MoneyFlowKernel::SmallOrder => &money_flow.small_order_pct,
        MoneyFlowKernel::SuperBigOrder => &money_flow.super_big_inflow,
        MoneyFlowKernel::NetInflow => {
            // An absent, out-of-range or non-finite level falls back to the
            // main-inflow series, exactly as `fn_netinflow` does. `as i32`
            // saturates NaN to 0 on both paths.
            let level = call
                .inputs
                .first()
                .map_or(0, |slot| buffers[slot.0].first().map_or(0, |v| *v as i32));
            match level {
                0 => &money_flow.main_inflow,
                1 => &money_flow.super_big_inflow,
                2 => &money_flow.big_inflow,
                3 => &money_flow.medium_inflow,
                4 => &money_flow.small_inflow,
                _ => &money_flow.main_inflow,
            }
        }
    };

    let Some(source) = source.as_slice() else {
        buffers[output].fill(f64::NAN);
        return Ok(());
    };
    if source.len() != buffers[output].len() {
        buffers[output].fill(f64::NAN);
        return Ok(());
    }
    buffers[output].copy_from_slice(source);
    Ok(())
}

/// `TR()`: the DZH true range over the host's implicit OHLC.
///
/// Bar 0 is `high - low`, **not** `NaN`. That single bar is what separates this
/// from `TRANGE(H, L, C)`, whose bar 0 is `NaN` by the TA-Lib convention, so the
/// two cannot share a kernel even though both are called "true range". Folding
/// `TR()` into the `TRANGE` kernel would have been a one-line change that
/// silently moved bar 0 of every `TR()` result.
fn dispatch_tr_call(
    host: &HostContext<'_>,
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
) -> Result<(), KernelDispatchError> {
    if !call.inputs.is_empty() {
        return Err(KernelDispatchError::new(FormulaKernelDispatcher::ERR_ARITY));
    }
    let output = call.output.0;
    let Some(ohlc) = host.ohlc else {
        buffers[output].fill(f64::NAN);
        return Ok(());
    };
    crate::indicators::volatility::trange_dzh_into(
        ohlc.high,
        ohlc.low,
        ohlc.close,
        &mut buffers[output],
    )
    .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
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
    // `canonical_boll` (the tree path's `BBANDS` entry) returns an all-NaN
    // series on a bad period, so absorb the kernel's failure rather than failing
    // the evaluation (`absorb_kernel_failure`). The period guard above runs
    // first, so a zero period is still an error.
    absorb_kernel_failure(
        crate::math::rolling_stats::bbands_upper_into(input, period, nb_dev, output),
        output,
    )
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
    let computed = {
        let input = &buffers[input_slot];
        crate::indicators::overlap::bbands(input, period, nb_dev, nb_dev)
    };
    // A period longer than the series is a data condition: the tree path's
    // `canonical_bband_component` answers an all-NaN series, so absorb the
    // kernel's own failure rather than failing the evaluation.
    let Some(result) = absorb_kernel_series(computed, &mut buffers[output_slot]) else {
        return Ok(());
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
    absorb_kernel_failure(
        crate::indicators::macd_line_into(input, fast, slow, signal, output),
        output,
    )
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
    };
    // `fn_dea` swallows a bad period into an all-NaN series, so this path must
    // too (`absorb_kernel_failure` explains why).
    let source = match result {
        Ok(result) => result.signal,
        Err(_) => {
            buffers[output_slot].fill(f64::NAN);
            return Ok(());
        }
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
    // `canonical_adosc` / `canonical_mfi` (and `canonical_ad`, which shares this
    // tail) answer an all-NaN series on a bad period, so mirror them
    // (`absorb_kernel_failure`). ADOSC's fast/slow and MFI's period go through
    // `period_from_slot` above, so a zero period is still an error.
    absorb_kernel_failure(result, output)
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
        if crate::indicators::momentum::willr_into(high, low, close, period, &mut willr).is_err() {
            // Degenerate period: the tree path yields an all-NaN series.
            willr.fill(f64::NAN);
        }
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
    let computed = {
        let high = &buffers[high_slot];
        let low = &buffers[low_slot];
        let close = &buffers[close_slot];
        crate::indicators::momentum::stochf(high, low, close, fast_k, fast_d)
    };
    // A degenerate period is a data condition; the tree path's `fn_stochf`
    // yields an all-NaN series, so absorb the kernel's own failure.
    let Some(result) = absorb_kernel_series(computed, &mut buffers[output_slot]) else {
        return Ok(());
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
/// hash with no string accessor — an enumeration is the only option available.
///
/// That constraint is exactly why `compute_ir.rs` collapses every `DrawGeneric`
/// command onto the single name `DRAW_GENERIC` instead of emitting
/// `DRAW:<command>`. The per-command form required this list to mirror the
/// parser's command vocabulary by hand, and it did not: `DRAW:FILL` was listed
/// but never produced, while eleven commands that *are* produced — `DRAWLINE`
/// among them — fell through unhandled, so any formula drawing a line failed on
/// the plan path. A one-name gate cannot rot that way.
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

/// TA-Lib functions that already have a formula implementation but do not yet
/// have a hand-written in-place plan kernel. They are routed through the cached
/// formula function table below. This is an intentional compatibility bridge:
/// it closes plan reachability first, while the specialized kernels can later
/// remove the allocation/copy path one family at a time without changing the
/// numeric contract.
const TALIB_FORMULA_GAP_KERNELS: &[&str] = &[
    "CALL:ADXR",
    "CALL:APO",
    "CALL:AROONOSC",
    "CALL:AVGPRICE",
    "CALL:AVGDEV",
    "CALL:BETA",
    "CALL:BOP",
    "CALL:CDL2CROWS",
    "CALL:CDL3BLACKCROWS",
    "CALL:CDL3INSIDE",
    "CALL:CDL3LINESTRIKE",
    "CALL:CDL3OUTSIDE",
    "CALL:CDL3STARSINSOUTH",
    "CALL:CDL3WHITESOLDIERS",
    "CALL:CDLDOJI",
    "CALL:CDLDOJISTAR",
    "CALL:CDLDRAGONFLYDOJI",
    "CALL:CDLENGULFING",
    "CALL:CDLEVENINGDOJISTAR",
    "CALL:CDLGRAVESTONEDOJI",
    "CALL:CDLHAMMER",
    "CALL:CDLHANGINGMAN",
    "CALL:CDLHARAMI",
    "CALL:CDLMARUBOZU",
    "CALL:CDLPIERCING",
    "CALL:CDLSHOOTINGSTAR",
    "CALL:CDLSPINNINGTOP",
    "CALL:CDLABANDONEDBABY",
    "CALL:CDLADVANCEBLOCK",
    "CALL:CDLBELTHOLD",
    "CALL:CDLBREAKAWAY",
    "CALL:CDLCLOSINGMARUBOZU",
    "CALL:CDLCONCEALBABYSWALL",
    "CALL:CDLCOUNTERATTACK",
    "CALL:CDLDARKCLOUDCOVER",
    "CALL:CDLEVENINGSTAR",
    "CALL:CDLGAPSIDESIDEWHITE",
    "CALL:CDLHARAMICROSS",
    "CALL:CDLHIGHWAVE",
    "CALL:CDLHIKKAKE",
    "CALL:CDLHIKKAKEMOD",
    "CALL:CDLHOMINGPIGEON",
    "CALL:CDLIDENTICAL3CROWS",
    "CALL:CDLINNECK",
    "CALL:CDLINVERTEDHAMMER",
    "CALL:CDLKICKING",
    "CALL:CDLKICKINGBYLENGTH",
    "CALL:CDLLADDERBOTTOM",
    "CALL:CDLLONGLEGGEDDOJI",
    "CALL:CDLLONGLINE",
    "CALL:CDLMATCHINGLOW",
    "CALL:CDLMATHOLD",
    "CALL:CDLMORNINGDOJISTAR",
    "CALL:CDLMORNINGSTAR",
    "CALL:CDLONNECK",
    "CALL:CDLRICKSHAWMAN",
    "CALL:CDLRISEFALL3METHODS",
    "CALL:CDLSEPARATINGLINES",
    "CALL:CDLSHORTLINE",
    "CALL:CDLSTALLEDPATTERN",
    "CALL:CDLSTICKSANDWICH",
    "CALL:CDLTAKURI",
    "CALL:CDLTASUKIGAP",
    "CALL:CDLTHRUSTING",
    "CALL:CDLTRISTAR",
    "CALL:CDLUNIQUE3RIVER",
    "CALL:CDLUPSIDEGAP2CROWS",
    "CALL:CDLXSIDEGAP3METHODS",
    "CALL:CMO",
    "CALL:DEMA",
    "CALL:DPO",
    "CALL:DX",
    "CALL:HT_DCPERIOD",
    "CALL:HT_DCPHASE",
    "CALL:HT_PHASOR",
    "CALL:HT_SINE",
    "CALL:HT_TRENDLINE",
    "CALL:HT_TRENDMODE",
    "CALL:IMI",
    "CALL:LINEARREG",
    "CALL:LINEARREG_ANGLE",
    "CALL:LINEARREG_INTERCEPT",
    "CALL:MACDEXT",
    "CALL:MACDFIX",
    "CALL:MAVP",
    "CALL:MEDPRICE",
    "CALL:MIDPOINT",
    "CALL:MIDPRICE",
    "CALL:MINMAX",
    "CALL:MINMAXINDEX",
    "CALL:MINUS_DM",
    "CALL:PLUS_DM",
    "CALL:PERCENTILE",
    "CALL:PPO",
    "CALL:SAREXT",
    "CALL:STOCH",
    "CALL:STOCHRSI",
    "CALL:T3",
    "CALL:TEMA",
    "CALL:TSF",
    "CALL:TYPPRICE",
    "CALL:ULTOSC",
    "CALL:WCLPRICE",
];

/// Execute one TA-Lib formula function through the plan-owned buffers.
///
/// This fallback deliberately allocates only at the boundary: canonical legacy
/// functions accept `Array1<f64>` arguments and the hot-plan ABI owns `Vec<f64>`
/// buffers. The function table itself is cached, so subsequent calls avoid its
/// construction cost. Families with a proven in-place kernel should stay on
/// their specialized dispatch branch above.
fn dispatch_formula_bridge_call(
    call: KernelCall<'_>,
    buffers: &mut [Vec<f64>],
    name: &str,
) -> Result<(), KernelDispatchError> {
    let output_slot = call.output.0;
    let len = match buffers.get(output_slot) {
        Some(output) if !output.is_empty() => output.len(),
        _ => {
            return Err(KernelDispatchError::new(
                FormulaKernelDispatcher::ERR_PARAMETER,
            ))
        }
    };
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

    let args: Vec<Array1<f64>> = call
        .inputs
        .iter()
        .map(|slot| Array1::from(buffers[slot.0].clone()))
        .collect();
    let context_series = |index: usize| {
        args.get(index)
            .cloned()
            .unwrap_or_else(|| Array1::zeros(len))
    };
    let ctx = crate::formula::types::FormulaContext::new(
        context_series(0),
        context_series(1),
        context_series(2),
        context_series(3),
        context_series(4),
        None,
    );
    let Some(kernel) = crate::formula::functions::lookup_builtin_function(name) else {
        return Err(KernelDispatchError::new(
            FormulaKernelDispatcher::ERR_UNSUPPORTED_KERNEL,
        ));
    };
    match kernel(&ctx, &args) {
        Ok(series) if series.len() == len => {
            buffers[output_slot].copy_from_slice(series.as_slice().unwrap());
        }
        Ok(_) | Err(_) => {
            buffers[output_slot].fill(f64::NAN);
        }
    }
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

    // TA-Lib 0.7/0.8 formula bridge (M0-1 surface). Each `CALL:NAME` is
    // delegated to the exact `canonical_*` implementation the tree/bytecode/JIT
    // paths use. The compiled plan path therefore produces byte-identical
    // output — zero divergence by construction; `formula_plan_differential`
    // remains the independent safety net.
    const TALIB_081_KERNELS: &[(
        &str,
        fn(
            &crate::formula::types::FormulaContext,
            &[Array1<f64>],
        ) -> std::result::Result<Array1<f64>, crate::error::FormulaError>,
    )] = &[
        ("CALL:AC", crate::formula::functions_talib_081::canonical_ac),
        (
            "CALL:ACCBANDS",
            crate::formula::functions_talib_081::canonical_accbands,
        ),
        (
            "CALL:ACCBANDS_MID",
            crate::formula::functions_talib_081::canonical_accbands_mid,
        ),
        (
            "CALL:ACCBANDS_LOWER",
            crate::formula::functions_talib_081::canonical_accbands_lower,
        ),
        (
            "CALL:ADR",
            crate::formula::functions_talib_081::canonical_adr,
        ),
        ("CALL:AO", crate::formula::functions_talib_081::canonical_ao),
        (
            "CALL:AROON",
            crate::formula::functions_talib_081::canonical_aroon,
        ),
        (
            "CALL:AROON_DOWN",
            crate::formula::functions_talib_081::canonical_aroon_down,
        ),
        (
            "CALL:CMOU",
            crate::formula::functions_talib_081::canonical_cmou,
        ),
        (
            "CALL:COPPOCK",
            crate::formula::functions_talib_081::canonical_coppock,
        ),
        (
            "CALL:CVI",
            crate::formula::functions_talib_081::canonical_cvi,
        ),
        (
            "CALL:EFI",
            crate::formula::functions_talib_081::canonical_efi,
        ),
        ("CALL:ER", crate::formula::functions_talib_081::canonical_er),
        (
            "CALL:ERI",
            crate::formula::functions_talib_081::canonical_eri,
        ),
        (
            "CALL:ERI_BEAR",
            crate::formula::functions_talib_081::canonical_eri_bear,
        ),
        (
            "CALL:FOSC",
            crate::formula::functions_talib_081::canonical_fosc,
        ),
        (
            "CALL:FRACTAL",
            crate::formula::functions_talib_081::canonical_fractal,
        ),
        (
            "CALL:FRACTAL_LOW",
            crate::formula::functions_talib_081::canonical_fractal_low,
        ),
        ("CALL:HA", crate::formula::functions_talib_081::canonical_ha),
        (
            "CALL:HA_OPEN",
            crate::formula::functions_talib_081::canonical_ha_open,
        ),
        (
            "CALL:HA_HIGH",
            crate::formula::functions_talib_081::canonical_ha_high,
        ),
        (
            "CALL:HA_LOW",
            crate::formula::functions_talib_081::canonical_ha_low,
        ),
        ("CALL:KC", crate::formula::functions_talib_081::canonical_kc),
        (
            "CALL:KC_MID",
            crate::formula::functions_talib_081::canonical_kc_mid,
        ),
        (
            "CALL:KC_LOWER",
            crate::formula::functions_talib_081::canonical_kc_lower,
        ),
        (
            "CALL:MAMA",
            crate::formula::functions_talib_081::canonical_mama,
        ),
        (
            "CALL:MAMA_FAMA",
            crate::formula::functions_talib_081::canonical_mama_fama,
        ),
        (
            "CALL:MARKETFI",
            crate::formula::functions_talib_081::canonical_marketfi,
        ),
        (
            "CALL:MASSI",
            crate::formula::functions_talib_081::canonical_massi,
        ),
        (
            "CALL:NVI",
            crate::formula::functions_talib_081::canonical_nvi,
        ),
        (
            "CALL:PERCENTRANK",
            crate::formula::functions_talib_081::canonical_percentrank,
        ),
        (
            "CALL:PVI",
            crate::formula::functions_talib_081::canonical_pvi,
        ),
        (
            "CALL:PVO",
            crate::formula::functions_talib_081::canonical_pvo,
        ),
        (
            "CALL:PVT",
            crate::formula::functions_talib_081::canonical_pvt,
        ),
        (
            "CALL:QSTICK",
            crate::formula::functions_talib_081::canonical_qstick,
        ),
        (
            "CALL:RVI",
            crate::formula::functions_talib_081::canonical_rvi,
        ),
        (
            "CALL:RVOL",
            crate::formula::functions_talib_081::canonical_rvol,
        ),
        (
            "CALL:SMI",
            crate::formula::functions_talib_081::canonical_smi,
        ),
        (
            "CALL:SMI_SIGNAL",
            crate::formula::functions_talib_081::canonical_smi_signal,
        ),
        (
            "CALL:VHF",
            crate::formula::functions_talib_081::canonical_vhf,
        ),
        (
            "CALL:VORTEX",
            crate::formula::functions_talib_081::canonical_vortex,
        ),
        (
            "CALL:VORTEX_MINUS",
            crate::formula::functions_talib_081::canonical_vortex_minus,
        ),
        (
            "CALL:WAD",
            crate::formula::functions_talib_081::canonical_wad,
        ),
        (
            "CALL:ZLEMA",
            crate::formula::functions_talib_081::canonical_zlema,
        ),
    ];
    for &(name, kernel) in TALIB_081_KERNELS {
        if is(name) {
            let output_slot = call.output.0;
            let len = match buffers.get(output_slot) {
                Some(buf) if !buf.is_empty() => buf.len(),
                _ => {
                    return Err(KernelDispatchError::new(
                        FormulaKernelDispatcher::ERR_PARAMETER,
                    ))
                }
            };
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
            let args: Vec<Array1<f64>> = call
                .inputs
                .iter()
                .map(|slot| Array1::from(buffers[slot.0].clone()))
                .collect();
            let ctx = crate::formula::types::FormulaContext::new(
                Array1::zeros(len),
                Array1::zeros(len),
                Array1::zeros(len),
                Array1::zeros(len),
                Array1::zeros(len),
                None,
            );
            match kernel(&ctx, &args) {
                Ok(series) => {
                    let out = &mut buffers[output_slot];
                    if series.len() == out.len() {
                        out.copy_from_slice(series.as_slice().unwrap());
                    } else {
                        out.fill(f64::NAN);
                    }
                }
                Err(_) => {
                    buffers[output_slot].fill(f64::NAN);
                }
            }
            return Ok(());
        }
    }

    // Compatibility bridge for the remaining TA-Lib catalogue. The lookup is
    // cached, and the canonical formula function is the same implementation
    // used by the tree/bytecode/JIT frontends. Errors are absorbed to an
    // all-NaN output exactly like the explicit TA-Lib bridge above.
    for &kernel_name in TALIB_FORMULA_GAP_KERNELS {
        if call.kernel == KernelId::from_static(kernel_name) {
            return dispatch_formula_bridge_call(call, buffers, &kernel_name[5..]);
        }
    }

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
        return absorb_kernel_failure(
            crate::indicators::statistics::zscore_into(input, period, output),
            output,
        );
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
        return absorb_kernel_failure(
            crate::math::moving_avg::vwma_into(input, volume, period, output),
            output,
        );
    }

    if is("CALL:CORREL") {
        // `CORREL(x, y, n)` is the rolling Pearson correlation of two series, so
        // it is a two-input kernel with the period last — the same shape as
        // VWMA. It delegates to `rolling_correlation_into`, which is exactly
        // what `fn_correl` calls on the tree path.
        let period = period_from_slot(buffers, call.inputs[2].0)?;
        let left_ptr = buffers[call.inputs[0].0].as_ptr();
        let right_ptr = buffers[call.inputs[1].0].as_ptr();
        let output_ptr = buffers[output_slot].as_mut_ptr();
        let (left, right, output) = unsafe {
            (
                std::slice::from_raw_parts(left_ptr, len),
                std::slice::from_raw_parts(right_ptr, len),
                std::slice::from_raw_parts_mut(output_ptr, len),
            )
        };
        return absorb_kernel_failure(
            crate::math::kernels::rolling_correlation_into(left, right, period, output),
            output,
        );
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::volume_ext::cmf(high, low, close, volume, period),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::momentum_ext::chop(high, low, close, period),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::momentum_ext::fisher(high, low, period),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::momentum_ext::tsi(input, long_period, short_period),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::china::kdj(high, low, close, n, m1, m2),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(upper) = absorb_kernel_series(
            crate::math::statistics::rolling_max(high, period),
            &mut *output,
        ) else {
            return Ok(());
        };
        let Some(lower) = absorb_kernel_series(
            crate::math::statistics::rolling_min(low, period),
            &mut *output,
        ) else {
            return Ok(());
        };
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
        let Some(result) = absorb_kernel_series(
            crate::indicators::supertrend::supertrend(high, low, close, period, multiplier),
            &mut *output,
        ) else {
            return Ok(());
        };
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
    let Some(result) = absorb_kernel_series(
        crate::indicators::donchian::donchian(high, low, period),
        &mut *output,
    ) else {
        return Ok(());
    };
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
) -> UnifiedExecutor<FormulaKernelDispatcher<'static>> {
    UnifiedExecutor::new(plan.hot().clone(), FormulaKernelDispatcher::new())
}

/// [`unified_formula_executor`] carrying host data.
///
/// Callers that evaluate formulas against a `FormulaContext` should use this so
/// host-dependent functions (`WINNER`, `COST`, `PERIODTYPE`, `TR` and the
/// money-flow family) agree with the tree path instead of silently degrading to
/// `NaN`.
pub fn unified_formula_executor_with_host<'a>(
    plan: &super::hot_plan::FormulaHotPlan,
    host: HostContext<'a>,
) -> UnifiedExecutor<FormulaKernelDispatcher<'a>> {
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
