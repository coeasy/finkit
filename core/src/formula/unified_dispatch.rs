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
}
