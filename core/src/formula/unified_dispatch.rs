//! Formula kernel dispatcher for the Architecture v3 [`UnifiedExecutor`].
//!
//! This is the first real frontend migration onto the numeric executor. Formula
//! parsing and semantic lowering still happen once at compile time; execution of
//! constants and core numeric unary/binary operators uses only [`KernelId`],
//! physical buffer slots and prebound parameter values.

use crate::execution_plan::KernelId;
use crate::state_arena::StateArena;
use crate::unified_executor::{KernelCall, KernelDispatchError, KernelDispatcher, UnifiedExecutor};

/// Numeric dispatcher for formula constants and core arithmetic/logical operators.
#[derive(Debug, Default, Clone, Copy)]
pub struct FormulaKernelDispatcher;

impl FormulaKernelDispatcher {
    const ERR_UNSUPPORTED_KERNEL: u32 = 1;
    const ERR_ARITY: u32 = 2;
    const ERR_PARAMETER: u32 = 3;

    /// Create the default stateless formula dispatcher.
    pub const fn new() -> Self {
        Self
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

        if call.kernel == KernelId::from_static("CALL:MA")
            || call.kernel == KernelId::from_static("CALL:EMA")
            || call.kernel == KernelId::from_static("CALL:WMA")
            || call.kernel == KernelId::from_static("CALL:KAMA")
            || call.kernel == KernelId::from_static("CALL:RSI")
            || call.kernel == KernelId::from_static("CALL:MOM")
            || call.kernel == KernelId::from_static("CALL:ROC")
            || call.kernel == KernelId::from_static("CALL:STD")
        {
            return dispatch_periodic_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:ATR") {
            return dispatch_hlc_periodic_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:BBANDS") {
            return dispatch_bbands_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:MACD") {
            return dispatch_macd_line_call(call, buffers);
        }

        if call.kernel == KernelId::from_static("CALL:OBV") {
            return dispatch_obv(call, buffers);
        }

        if call.kernel == KernelId::from_static("UNARY:Neg") {
            return unary(call, buffers, |value| -value);
        }
        if call.kernel == KernelId::from_static("UNARY:Not") {
            return unary(call, buffers, |value| if value <= 0.0 { 1.0 } else { 0.0 });
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
fn dispatch_periodic_call(
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

    let result = if call.kernel == KernelId::from_static("CALL:MA") {
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
    } else if call.kernel == KernelId::from_static("CALL:STD") {
        crate::math::rolling_stats::stddev_into(input, period, 1.0, output)
    } else {
        crate::indicators::roc_into(input, period, output)
    };

    result.map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
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

/// Execute ATR directly from the HLC dependency slots.
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
    crate::indicators::volatility::atr_into(high, low, close, period, output)
        .map_err(|_| KernelDispatchError::new(FormulaKernelDispatcher::ERR_PARAMETER))
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

#[derive(Debug, Clone, Copy)]
enum BinaryKernel {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
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
}
