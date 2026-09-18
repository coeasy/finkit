//! Explicit stateful streaming for the portable Formula subset.
//!
//! Formula batch evaluation remains the source of truth for the complete
//! language. This module provides a deliberately narrow, serializable state
//! contract for direct recursive indicators whose row kernel is already
//! verified against the batch implementation. Numeric assignments and
//! multi-statement programs are supported when their dependencies can be
//! represented by this state model; drawing, loops, future-data functions and
//! host-dependent calls are rejected instead of being silently approximated.

use super::ast::{AstNode, BinaryOperator, CompoundAssignOp, UnaryOperator};
use super::{normalize_formula_source, parse_formula_with_dialect, FormulaDialect};
use crate::factors::{FactorError, FactorResult};
use crate::formula::types::{classify_builtin_var, BuiltinVar};
use crate::streaming::indicators::{
    StreamingEma, StreamingMax, StreamingMin, StreamingRsi, StreamingSma, StreamingSum,
    StreamingWma,
};
use crate::streaming::momentum::macd::StreamingMacd;
use crate::streaming::traits::StreamingIndicator;
use std::collections::{BTreeMap, HashSet, VecDeque};

/// Direct raw OHLCV input consumed by a stateful Formula call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum FormulaStateInput {
    Open,
    High,
    Low,
    Close,
    Volume,
    Amount,
}

impl FormulaStateInput {
    /// Canonical contract key used by JSON requests.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
            Self::Amount => "amount",
        }
    }

    fn from_ast(node: &AstNode) -> FactorResult<Self> {
        let AstNode::Variable(name) = node else {
            return Err(FactorError::InvalidParameter(
                "stateful Formula inputs must be direct OHLCV variables".to_string(),
            ));
        };
        match classify_builtin_var(name) {
            Some(BuiltinVar::Open) => Ok(Self::Open),
            Some(BuiltinVar::High) => Ok(Self::High),
            Some(BuiltinVar::Low) => Ok(Self::Low),
            Some(BuiltinVar::Close) => Ok(Self::Close),
            Some(BuiltinVar::Volume) => Ok(Self::Volume),
            Some(BuiltinVar::Amount) => Ok(Self::Amount),
            _ => Err(FactorError::InvalidParameter(format!(
                "stateful Formula input is not a portable OHLCV variable: {name}"
            ))),
        }
    }
}

/// O(1)-per-row stateful Formula executor for the verified portable subset.
#[derive(Clone)]
pub struct FormulaStatefulStream {
    signature: u64,
    required_inputs: Vec<FormulaStateInput>,
    state: FormulaState,
    row_count: usize,
    runtime_variables: BTreeMap<String, f64>,
}

/// Serializable checkpoint for [`FormulaStatefulStream`].
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FormulaStatefulCheckpoint {
    signature: u64,
    required_inputs: Vec<FormulaStateInput>,
    state: FormulaState,
    row_count: usize,
}

impl FormulaStatefulCheckpoint {
    /// Stable formula identity associated with this checkpoint.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Number of rows consumed before the checkpoint.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Serialize a checkpoint for the language-neutral stream contract.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    /// Decode a checkpoint received from a language adapter.
    #[cfg(feature = "serde")]
    pub fn from_json(value: &str) -> Result<Self, String> {
        serde_json::from_str(value).map_err(|error| error.to_string())
    }
}

impl FormulaStatefulStream {
    /// Compile a stateful Formula using an explicit dialect profile.
    pub fn from_source(source: &str, dialect: FormulaDialect) -> FactorResult<Self> {
        let normalized = normalize_formula_source(source, dialect);
        let ast = parse_formula_with_dialect(&normalized, dialect)
            .map_err(|error| FactorError::InvalidParameter(error.to_string()))?;
        let (state, required_inputs) = match direct_expression(&ast) {
            Ok(expression) => match compile_state(expression) {
                Ok((state, inputs)) => (state, inputs),
                Err(_) => compile_expression_or_program(&ast)?,
            },
            Err(_) => compile_expression_or_program(&ast)?,
        };
        Ok(Self {
            signature: formula_signature(&normalized, dialect),
            required_inputs,
            state,
            row_count: 0,
            runtime_variables: BTreeMap::new(),
        })
    }

    /// Stable identity of the compiled formula profile and source.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Raw input names required by the direct state kernel.
    #[must_use]
    pub fn required_inputs(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.required_inputs.iter().map(|input| input.as_str())
    }

    /// Number of rows consumed by this stream.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Advance one row in the canonical `[open, high, low, close, volume,
    /// amount]` slot layout and write the nullable result as `NaN` when the
    /// indicator is warming up.
    pub fn push_values_into(&mut self, values: &[f64], output: &mut f64) -> FactorResult<()> {
        if values.len() != 6 {
            return Err(FactorError::LengthMismatch {
                name: "formula_stateful_row".to_string(),
                expected: 6,
                actual: values.len(),
            });
        }
        let row = [
            values[0], values[1], values[2], values[3], values[4], values[5],
        ];
        *output = self.next_row(row);
        Ok(())
    }

    /// Append aligned named columns into a reusable output vector.
    pub fn push_batch_into(
        &mut self,
        inputs: &BTreeMap<String, Vec<f64>>,
        output: &mut Vec<f64>,
    ) -> FactorResult<()> {
        let rows = self
            .required_inputs
            .first()
            .and_then(|input| inputs.get(input.as_str()))
            .ok_or_else(|| {
                FactorError::MissingInput(
                    self.required_inputs
                        .first()
                        .map_or("formula_stateful_input", |input| input.as_str())
                        .to_string(),
                )
            })?
            .len();
        for input in &self.required_inputs {
            let values = inputs
                .get(input.as_str())
                .ok_or_else(|| FactorError::MissingInput(input.as_str().to_string()))?;
            if values.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: input.as_str().to_string(),
                    expected: rows,
                    actual: values.len(),
                });
            }
        }
        output.clear();
        output.reserve(rows);
        let mut row = [f64::NAN; 6];
        for index in 0..rows {
            row.fill(f64::NAN);
            for input in &self.required_inputs {
                row[input.slot()] = inputs[input.as_str()][index];
            }
            output.push(self.next_row(row));
        }
        Ok(())
    }

    /// Capture state without retaining the raw input history.
    #[must_use]
    pub fn checkpoint(&self) -> FormulaStatefulCheckpoint {
        FormulaStatefulCheckpoint {
            signature: self.signature,
            required_inputs: self.required_inputs.clone(),
            state: self.state.clone(),
            row_count: self.row_count,
        }
    }

    /// Restore a checkpoint created by the same source/profile contract.
    pub fn restore(&mut self, checkpoint: &FormulaStatefulCheckpoint) -> FactorResult<()> {
        if checkpoint.signature != self.signature
            || checkpoint.required_inputs != self.required_inputs
        {
            return Err(FactorError::InvalidParameter(
                "formula stateful checkpoint belongs to a different source or dialect".to_string(),
            ));
        }
        self.state = checkpoint.state.clone();
        self.row_count = checkpoint.row_count;
        self.runtime_variables.clear();
        Ok(())
    }

    fn next_row(&mut self, row: [f64; 6]) -> f64 {
        self.runtime_variables.clear();
        let value = self.state.next(&row, &mut self.runtime_variables);
        self.row_count = self.row_count.saturating_add(1);
        value
    }
}

impl FormulaStateInput {
    fn slot(self) -> usize {
        match self {
            Self::Open => 0,
            Self::High => 1,
            Self::Low => 2,
            Self::Close => 3,
            Self::Volume => 4,
            Self::Amount => 5,
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FormulaState {
    Expression {
        expression: FormulaExpressionState,
    },
    Program(FormulaProgramState),
    Sma {
        input: FormulaStateInput,
        indicator: StreamingSma,
    },
    Wma {
        input: FormulaStateInput,
        indicator: StreamingWma,
    },
    Ema {
        input: FormulaStateInput,
        indicator: StreamingEma,
    },
    Rsi {
        input: FormulaStateInput,
        indicator: StreamingRsi,
    },
    Max {
        input: FormulaStateInput,
        indicator: StreamingMax,
    },
    Min {
        input: FormulaStateInput,
        indicator: StreamingMin,
    },
    Sum {
        input: FormulaStateInput,
        indicator: StreamingSum,
    },
    Reference {
        input: FormulaStateInput,
        state: FormulaReferenceState,
    },
    Cross {
        left: FormulaStateInput,
        right: FormulaStateInput,
        direction: FormulaCrossDirection,
        previous: Option<(f64, f64)>,
    },
    Variance {
        input: FormulaStateInput,
        indicator: FormulaRollingVariance,
        square_root: bool,
    },
    Atr {
        high: FormulaStateInput,
        low: FormulaStateInput,
        close: FormulaStateInput,
        indicator: FormulaAtrState,
    },
    Macd {
        input: FormulaStateInput,
        indicator: StreamingMacd,
    },
}

impl FormulaState {
    fn next(&mut self, row: &[f64; 6], variables: &mut BTreeMap<String, f64>) -> f64 {
        let value = match self {
            Self::Expression { expression } => Some(expression.next(row, variables)),
            Self::Program(program) => Some(program.next(row)),
            Self::Sma { input, indicator } => indicator.next(row[input.slot()]),
            Self::Wma { input, indicator } => indicator.next(row[input.slot()]),
            Self::Ema { input, indicator } => indicator.next(row[input.slot()]),
            Self::Rsi { input, indicator } => indicator.next(row[input.slot()]),
            Self::Max { input, indicator } => indicator.next(row[input.slot()]),
            Self::Min { input, indicator } => indicator.next(row[input.slot()]),
            Self::Sum { input, indicator } => indicator.next(row[input.slot()]),
            Self::Reference { input, state } => state.next(row[input.slot()]),
            Self::Cross {
                left,
                right,
                direction,
                previous,
            } => {
                let current = (row[left.slot()], row[right.slot()]);
                let result = previous.map_or(0.0, |(previous_left, previous_right)| {
                    let crossed = match direction {
                        FormulaCrossDirection::Above => {
                            previous_left <= previous_right && current.0 > current.1
                        }
                        FormulaCrossDirection::Below => {
                            previous_left >= previous_right && current.0 < current.1
                        }
                    };
                    if crossed {
                        1.0
                    } else {
                        0.0
                    }
                });
                *previous = Some(current);
                Some(result)
            }
            Self::Variance {
                input,
                indicator,
                square_root,
            } => indicator.next(row[input.slot()]).map(|value| {
                if *square_root {
                    value.sqrt()
                } else {
                    value
                }
            }),
            Self::Atr {
                high,
                low,
                close,
                indicator,
            } => indicator.next(row[high.slot()], row[low.slot()], row[close.slot()]),
            Self::Macd { input, indicator } => indicator
                .next(row[input.slot()])
                .map(|value| value.histogram),
        };
        value.unwrap_or(f64::NAN)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct FormulaProgramState {
    statements: Vec<FormulaStatementState>,
    variables: BTreeMap<String, f64>,
}

impl FormulaProgramState {
    fn next(&mut self, row: &[f64; 6]) -> f64 {
        self.variables.clear();
        let mut result = f64::NAN;
        for statement in &mut self.statements {
            result = statement.expression.next(row, &mut self.variables);
            if let Some(name) = &statement.target {
                self.variables.insert(name.clone(), result);
            }
        }
        result
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct FormulaStatementState {
    target: Option<String>,
    expression: FormulaExpressionState,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FormulaExpressionState {
    Input(FormulaStateInput),
    Constant(f64),
    Variable(String),
    Direct(Box<FormulaState>),
    Unary {
        op: UnaryOperator,
        expression: Box<FormulaExpressionState>,
    },
    Binary {
        op: BinaryOperator,
        left: Box<FormulaExpressionState>,
        right: Box<FormulaExpressionState>,
    },
    Conditional {
        condition: Box<FormulaExpressionState>,
        then_branch: Box<FormulaExpressionState>,
        else_branch: Box<FormulaExpressionState>,
    },
    UnaryFunction {
        function: FormulaUnaryFunction,
        expression: Box<FormulaExpressionState>,
    },
    Stateful {
        function: FormulaExpressionFunction,
        expression: Box<FormulaExpressionState>,
    },
    Reference {
        state: FormulaReferenceState,
        expression: Box<FormulaExpressionState>,
    },
    Cross {
        direction: FormulaCrossDirection,
        previous: Option<(f64, f64)>,
        left: Box<FormulaExpressionState>,
        right: Box<FormulaExpressionState>,
    },
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FormulaUnaryFunction {
    Abs,
    Sign,
    Sqrt,
    Exp,
    Log,
    Log10,
    Floor,
    Ceil,
    Sin,
    Cos,
    Tan,
    Sinh,
    Cosh,
    Tanh,
    Asin,
    Acos,
    Atan,
    Not,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FormulaExpressionFunction {
    Sma(StreamingSma),
    Wma(StreamingWma),
    Ema(StreamingEma),
    Rsi(StreamingRsi),
    Max(StreamingMax),
    Min(StreamingMin),
    Sum(StreamingSum),
    Variance {
        indicator: FormulaRollingVariance,
        square_root: bool,
    },
}

impl FormulaExpressionFunction {
    fn next(&mut self, input: f64) -> f64 {
        let value = match self {
            Self::Sma(indicator) => indicator.next(input),
            Self::Wma(indicator) => indicator.next(input),
            Self::Ema(indicator) => indicator.next(input),
            Self::Rsi(indicator) => indicator.next(input),
            Self::Max(indicator) => indicator.next(input),
            Self::Min(indicator) => indicator.next(input),
            Self::Sum(indicator) => indicator.next(input),
            Self::Variance {
                indicator,
                square_root,
            } => indicator
                .next(input)
                .map(|value| if *square_root { value.sqrt() } else { value }),
        };
        value.unwrap_or(f64::NAN)
    }
}

impl FormulaExpressionState {
    fn next(&mut self, row: &[f64; 6], variables: &mut BTreeMap<String, f64>) -> f64 {
        match self {
            Self::Input(input) => row[input.slot()],
            Self::Constant(value) => *value,
            Self::Variable(name) => variables.get(name).copied().unwrap_or(f64::NAN),
            Self::Direct(state) => state.next(row, variables),
            Self::Unary { op, expression } => {
                let value = expression.next(row, variables);
                match op {
                    UnaryOperator::Not => {
                        if value > 0.0 {
                            0.0
                        } else {
                            1.0
                        }
                    }
                    UnaryOperator::Neg => -value,
                }
            }
            Self::Binary { op, left, right } => {
                let left = left.next(row, variables);
                let right = right.next(row, variables);
                apply_stateful_binary(op, left, right)
            }
            Self::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = condition.next(row, variables);
                // Both branches advance every row so a stateful indicator in
                // an unselected branch remains aligned with batch Formula
                // evaluation before the result is selected.
                let then_value = then_branch.next(row, variables);
                let else_value = else_branch.next(row, variables);
                if condition > 0.0 {
                    then_value
                } else {
                    else_value
                }
            }
            Self::UnaryFunction {
                function,
                expression,
            } => apply_stateful_unary_function(*function, expression.next(row, variables)),
            Self::Stateful {
                function,
                expression,
            } => function.next(expression.next(row, variables)),
            Self::Reference {
                state, expression, ..
            } => state
                .next(expression.next(row, variables))
                .unwrap_or(f64::NAN),
            Self::Cross {
                direction,
                previous,
                left,
                right,
            } => {
                let current = (left.next(row, variables), right.next(row, variables));
                let result = previous.map_or(0.0, |(previous_left, previous_right)| {
                    let crossed = match direction {
                        FormulaCrossDirection::Above => {
                            previous_left <= previous_right && current.0 > current.1
                        }
                        FormulaCrossDirection::Below => {
                            previous_left >= previous_right && current.0 < current.1
                        }
                    };
                    if crossed {
                        1.0
                    } else {
                        0.0
                    }
                });
                *previous = Some(current);
                result
            }
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct FormulaAtrState {
    period: usize,
    atr_value: f64,
    tr_sum: f64,
    previous_close: f64,
    count: usize,
}

impl FormulaAtrState {
    fn new(period: usize) -> Self {
        Self {
            period,
            atr_value: f64::NAN,
            tr_sum: 0.0,
            previous_close: f64::NAN,
            count: 0,
        }
    }

    fn next(&mut self, high: f64, low: f64, close: f64) -> Option<f64> {
        self.count = self.count.saturating_add(1);
        if !high.is_finite() || !low.is_finite() || !close.is_finite() {
            self.atr_value = f64::NAN;
            self.tr_sum = 0.0;
            self.previous_close = f64::NAN;
            self.count = 0;
            return None;
        }
        if self.count == 1 {
            self.previous_close = close;
            return None;
        }
        let true_range = (high - low)
            .max((high - self.previous_close).abs())
            .max((low - self.previous_close).abs());
        self.previous_close = close;
        if self.count <= self.period {
            self.tr_sum += true_range;
            None
        } else if self.count == self.period + 1 {
            self.tr_sum += true_range;
            self.atr_value = self.tr_sum / self.period as f64;
            Some(self.atr_value)
        } else {
            self.atr_value += (true_range - self.atr_value) / self.period as f64;
            Some(self.atr_value)
        }
    }
}

/// Rolling population variance used by the canonical Formula `STD`/`VAR`
/// semantics.  The public `StreamingVar` indicator intentionally keeps its
/// historical sample-variance contract, while domestic Formula and TA-Lib
/// profiles use population variance (division by `n`).
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct FormulaRollingVariance {
    period: usize,
    buffer: Vec<f64>,
    head: usize,
    len: usize,
    sum: f64,
    sum_sq: f64,
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct FormulaReferenceState {
    period: usize,
    values: VecDeque<f64>,
}

impl FormulaReferenceState {
    fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
        }
    }

    fn next(&mut self, input: f64) -> Option<f64> {
        let output = (self.values.len() >= self.period)
            .then(|| self.values.pop_front().expect("reference buffer is ready"));
        self.values.push_back(input);
        output
    }
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum FormulaCrossDirection {
    Above,
    Below,
}

impl FormulaRollingVariance {
    fn new(period: usize) -> Self {
        Self {
            period,
            buffer: vec![0.0; period],
            head: 0,
            len: 0,
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    fn next(&mut self, input: f64) -> Option<f64> {
        self.sum += input;
        self.sum_sq += input * input;
        if self.len == self.period {
            let old = self.buffer[self.head];
            self.sum -= old;
            self.sum_sq -= old * old;
        } else {
            self.len += 1;
        }
        self.buffer[self.head] = input;
        self.head = (self.head + 1) % self.period;
        if self.len < self.period {
            return None;
        }
        let mean = self.sum / self.period as f64;
        Some(((self.sum_sq - self.sum * mean) / self.period as f64).max(0.0))
    }
}

fn direct_expression(ast: &AstNode) -> FactorResult<&AstNode> {
    match ast {
        AstNode::FunctionCall { .. } => Ok(ast),
        AstNode::Output { expr, .. } => direct_expression(expr),
        AstNode::Statements(nodes) if nodes.len() == 1 => direct_expression(&nodes[0]),
        _ => Err(FactorError::InvalidParameter(
            "stateful Formula requires one direct indicator expression or a supported numeric program"
                .to_string(),
        )),
    }
}

fn compile_expression_or_program(
    ast: &AstNode,
) -> FactorResult<(FormulaState, Vec<FormulaStateInput>)> {
    match compile_expression(ast) {
        Ok((expression, inputs)) => Ok((FormulaState::Expression { expression }, inputs)),
        Err(expression_error) => match compile_program(ast) {
            Ok((program, inputs)) => Ok((FormulaState::Program(program), inputs)),
            Err(_) => Err(expression_error),
        },
    }
}

fn compile_program(ast: &AstNode) -> FactorResult<(FormulaProgramState, Vec<FormulaStateInput>)> {
    let nodes: &[AstNode] = match ast {
        AstNode::Statements(nodes) => nodes,
        node => std::slice::from_ref(node),
    };
    if nodes.is_empty() {
        return Err(FactorError::InvalidParameter(
            "stateful Formula program cannot be empty".to_string(),
        ));
    }

    let mut known_variables = HashSet::new();
    let mut statements = Vec::with_capacity(nodes.len());
    let mut required_inputs = Vec::new();
    for node in nodes {
        let (target, expression_ast) = match node {
            AstNode::Assignment { name, expr } | AstNode::Output { name, expr, .. } => {
                (Some(name.clone()), expr.as_ref())
            }
            AstNode::CompoundAssignment { name, op, expr } => {
                if !known_variables.contains(name) {
                    return Err(FactorError::InvalidParameter(format!(
                        "stateful Formula compound assignment references undeclared variable: {name}"
                    )));
                }
                let binary = AstNode::BinaryOp {
                    op: compound_assignment_operator(op),
                    left: Box::new(AstNode::Variable(name.clone())),
                    right: expr.clone(),
                };
                let (expression, inputs) =
                    compile_expression_with_variables(&binary, Some(&known_variables))?;
                merge_required_inputs(&mut required_inputs, &inputs);
                statements.push(FormulaStatementState {
                    target: Some(name.clone()),
                    expression,
                });
                continue;
            }
            AstNode::ParamDecl { .. } => {
                return Err(FactorError::InvalidParameter(
                    "stateful Formula programs do not support parameter declarations".to_string(),
                ));
            }
            _ => (None, node),
        };

        let (expression, inputs) =
            compile_expression_with_variables(expression_ast, Some(&known_variables))?;
        merge_required_inputs(&mut required_inputs, &inputs);
        if let Some(name) = &target {
            known_variables.insert(name.clone());
        }
        statements.push(FormulaStatementState { target, expression });
    }

    Ok((
        FormulaProgramState {
            statements,
            variables: BTreeMap::new(),
        },
        required_inputs,
    ))
}

fn compound_assignment_operator(op: &CompoundAssignOp) -> BinaryOperator {
    match op {
        CompoundAssignOp::AddAssign => BinaryOperator::Add,
        CompoundAssignOp::SubAssign => BinaryOperator::Sub,
        CompoundAssignOp::MulAssign => BinaryOperator::Mul,
        CompoundAssignOp::DivAssign => BinaryOperator::Div,
    }
}

fn compile_expression(
    ast: &AstNode,
) -> FactorResult<(FormulaExpressionState, Vec<FormulaStateInput>)> {
    compile_expression_with_variables(ast, None)
}

fn compile_expression_with_variables(
    ast: &AstNode,
    known_variables: Option<&HashSet<String>>,
) -> FactorResult<(FormulaExpressionState, Vec<FormulaStateInput>)> {
    match ast {
        AstNode::Number(value) => Ok((FormulaExpressionState::Constant(*value), Vec::new())),
        AstNode::Variable(name) => {
            if let Ok(input) = FormulaStateInput::from_ast(ast) {
                return Ok((FormulaExpressionState::Input(input), vec![input]));
            }
            if known_variables.is_some_and(|variables| variables.contains(name)) {
                return Ok((FormulaExpressionState::Variable(name.clone()), Vec::new()));
            }
            Err(FactorError::InvalidParameter(format!(
                "stateful Formula input is not a declared portable variable: {name}"
            )))
        }
        AstNode::UnaryOp { op, expr } => {
            let (expression, inputs) = compile_expression_with_variables(expr, known_variables)?;
            Ok((
                FormulaExpressionState::Unary {
                    op: op.clone(),
                    expression: Box::new(expression),
                },
                inputs,
            ))
        }
        AstNode::BinaryOp { op, left, right } => {
            if matches!(op, BinaryOperator::StringConcat) {
                return Err(FactorError::InvalidParameter(
                    "stateful Formula does not support string concatenation".to_string(),
                ));
            }
            let (left, mut inputs) = compile_expression_with_variables(left, known_variables)?;
            let (right, right_inputs) =
                compile_expression_with_variables(right, known_variables)?;
            merge_required_inputs(&mut inputs, &right_inputs);
            Ok((
                FormulaExpressionState::Binary {
                    op: op.clone(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
                inputs,
            ))
        }
        AstNode::IfThenElse {
            cond,
            then_branch,
            else_branch,
        } => {
            let (condition, mut inputs) = compile_expression_with_variables(cond, known_variables)?;
            let (then_branch, then_inputs) =
                compile_expression_with_variables(then_branch, known_variables)?;
            let (else_branch, else_inputs) =
                compile_expression_with_variables(else_branch, known_variables)?;
            merge_required_inputs(&mut inputs, &then_inputs);
            merge_required_inputs(&mut inputs, &else_inputs);
            Ok((
                FormulaExpressionState::Conditional {
                    condition: Box::new(condition),
                    then_branch: Box::new(then_branch),
                    else_branch: Box::new(else_branch),
                },
                inputs,
            ))
        }
        AstNode::FunctionCall { name, args } => {
            let normalized = name.to_ascii_uppercase();
            if normalized == "IF" {
                if args.len() < 3 {
                    return Err(FactorError::InvalidParameter(
                        "IF requires condition, then and else expressions".to_string(),
                    ));
                }
                let (condition, mut inputs) =
                    compile_expression_with_variables(&args[0], known_variables)?;
                let (then_branch, then_inputs) =
                    compile_expression_with_variables(&args[1], known_variables)?;
                let (else_branch, else_inputs) =
                    compile_expression_with_variables(&args[2], known_variables)?;
                merge_required_inputs(&mut inputs, &then_inputs);
                merge_required_inputs(&mut inputs, &else_inputs);
                return Ok((
                    FormulaExpressionState::Conditional {
                        condition: Box::new(condition),
                        then_branch: Box::new(then_branch),
                        else_branch: Box::new(else_branch),
                    },
                    inputs,
                ));
            }
            if let Some((expression, inputs)) =
                compile_expression_stateful_function(&normalized, args, known_variables)?
            {
                return Ok((expression, inputs));
            }
            if let Some(function) = FormulaUnaryFunction::from_name(&normalized) {
                if args.len() != 1 {
                    return Err(FactorError::InvalidParameter(format!(
                        "{name} requires exactly one expression"
                    )));
                }
                let (expression, inputs) =
                    compile_expression_with_variables(&args[0], known_variables)?;
                return Ok((
                    FormulaExpressionState::UnaryFunction {
                        function,
                        expression: Box::new(expression),
                    },
                    inputs,
                ));
            }
            let (state, inputs) = compile_state(ast)?;
            Ok((FormulaExpressionState::Direct(Box::new(state)), inputs))
        }
        AstNode::Output { expr, .. } => compile_expression_with_variables(expr, known_variables),
        AstNode::Statements(nodes) if nodes.len() == 1 => {
            compile_expression_with_variables(&nodes[0], known_variables)
        }
        _ => Err(FactorError::InvalidParameter(
            "stateful Formula expression contains drawing, control flow, indexing or unsupported values"
                .to_string(),
        )),
    }
}

fn compile_expression_stateful_function(
    name: &str,
    args: &[AstNode],
    known_variables: Option<&HashSet<String>>,
) -> FactorResult<Option<(FormulaExpressionState, Vec<FormulaStateInput>)>> {
    let period = |minimum: usize| parse_stateful_period(args, 1, 14, name, minimum);
    let unary = |function: FormulaExpressionFunction| -> FactorResult<
        Option<(FormulaExpressionState, Vec<FormulaStateInput>)>,
    > {
        if args.is_empty() {
            return Err(FactorError::InvalidParameter(format!(
                "{name} requires an input"
            )));
        }
        let (expression, inputs) =
            compile_expression_with_variables(&args[0], known_variables)?;
        Ok(Some((
            FormulaExpressionState::Stateful {
                function,
                expression: Box::new(expression),
            },
            inputs,
        )))
    };

    match name {
        "MA" | "SMA" => Ok(unary(FormulaExpressionFunction::Sma(StreamingSma::new(
            period(1)?,
        )))?),
        "WMA" => Ok(unary(FormulaExpressionFunction::Wma(StreamingWma::new(
            period(1)?,
        )))?),
        "EMA" => Ok(unary(FormulaExpressionFunction::Ema(StreamingEma::new(
            period(1)?,
        )))?),
        "RSI" => Ok(unary(FormulaExpressionFunction::Rsi(StreamingRsi::new(
            period(1)?,
        )))?),
        "HHV" => Ok(unary(FormulaExpressionFunction::Max(StreamingMax::new(
            period(1)?,
        )))?),
        "LLV" => Ok(unary(FormulaExpressionFunction::Min(StreamingMin::new(
            period(1)?,
        )))?),
        "SUM" => Ok(unary(FormulaExpressionFunction::Sum(StreamingSum::new(
            period(1)?,
        )))?),
        "STD" | "STDDEV" => {
            let period = period(2)?;
            Ok(unary(FormulaExpressionFunction::Variance {
                indicator: FormulaRollingVariance::new(period),
                square_root: true,
            })?)
        }
        "VAR" => Ok(unary(FormulaExpressionFunction::Variance {
            indicator: FormulaRollingVariance::new(period(1)?),
            square_root: false,
        })?),
        "REF" => {
            if args.len() < 2 {
                return Err(FactorError::InvalidParameter(
                    "REF requires an input and period".to_string(),
                ));
            }
            let period = parse_stateful_period(args, 1, 1, name, 1)?;
            let (expression, inputs) =
                compile_expression_with_variables(&args[0], known_variables)?;
            Ok(Some((
                FormulaExpressionState::Reference {
                    state: FormulaReferenceState::new(period),
                    expression: Box::new(expression),
                },
                inputs,
            )))
        }
        "CROSS" | "CROSSBELOW" => {
            if args.len() < 2 {
                return Err(FactorError::InvalidParameter(format!(
                    "{name} requires two expressions"
                )));
            }
            let (left, mut inputs) = compile_expression_with_variables(&args[0], known_variables)?;
            let (right, right_inputs) =
                compile_expression_with_variables(&args[1], known_variables)?;
            merge_required_inputs(&mut inputs, &right_inputs);
            Ok(Some((
                FormulaExpressionState::Cross {
                    direction: if name == "CROSS" {
                        FormulaCrossDirection::Above
                    } else {
                        FormulaCrossDirection::Below
                    },
                    previous: None,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                inputs,
            )))
        }
        _ => Ok(None),
    }
}

fn parse_stateful_period(
    args: &[AstNode],
    index: usize,
    default: usize,
    name: &str,
    minimum: usize,
) -> FactorResult<usize> {
    match args.get(index) {
        None => Ok(default),
        Some(AstNode::Number(value))
            if value.is_finite() && *value >= minimum as f64 && value.fract() == 0.0 =>
        {
            Ok(*value as usize)
        }
        _ => Err(FactorError::InvalidParameter(format!(
            "{name} period must be an integer greater than or equal to {minimum}"
        ))),
    }
}

impl FormulaUnaryFunction {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "ABS" => Self::Abs,
            "SIGN" => Self::Sign,
            "SQRT" => Self::Sqrt,
            "EXP" => Self::Exp,
            "LOG" | "LN" => Self::Log,
            "LOG10" => Self::Log10,
            "FLOOR" => Self::Floor,
            "CEIL" | "CEILING" => Self::Ceil,
            "SIN" => Self::Sin,
            "COS" => Self::Cos,
            "TAN" => Self::Tan,
            "SINH" => Self::Sinh,
            "COSH" => Self::Cosh,
            "TANH" => Self::Tanh,
            "ASIN" => Self::Asin,
            "ACOS" => Self::Acos,
            "ATAN" => Self::Atan,
            "NOT" => Self::Not,
            _ => return None,
        })
    }
}

fn merge_required_inputs(target: &mut Vec<FormulaStateInput>, inputs: &[FormulaStateInput]) {
    for input in inputs {
        if !target.contains(input) {
            target.push(*input);
        }
    }
}

fn apply_stateful_binary(op: &BinaryOperator, left: f64, right: f64) -> f64 {
    match op {
        BinaryOperator::Add => left + right,
        BinaryOperator::Sub => left - right,
        BinaryOperator::Mul => left * right,
        BinaryOperator::Div => {
            if right.abs() < 1e-15 {
                f64::NAN
            } else {
                left / right
            }
        }
        BinaryOperator::Mod => {
            if right.abs() < 1e-15 {
                f64::NAN
            } else {
                left - (left / right).floor() * right
            }
        }
        BinaryOperator::Pow => left.powf(right),
        BinaryOperator::Gt => (left > right) as u8 as f64,
        BinaryOperator::Lt => (left < right) as u8 as f64,
        BinaryOperator::Gte => (left >= right) as u8 as f64,
        BinaryOperator::Lte => (left <= right) as u8 as f64,
        BinaryOperator::Eq => ((left - right).abs() < 1e-10) as u8 as f64,
        BinaryOperator::Neq => ((left - right).abs() >= 1e-10) as u8 as f64,
        BinaryOperator::And => ((left > 0.0) && (right > 0.0)) as u8 as f64,
        BinaryOperator::Or => ((left > 0.0) || (right > 0.0)) as u8 as f64,
        BinaryOperator::Xor => ((left > 0.0) != (right > 0.0)) as u8 as f64,
        BinaryOperator::StringConcat => f64::NAN,
    }
}

fn apply_stateful_unary_function(function: FormulaUnaryFunction, value: f64) -> f64 {
    match function {
        FormulaUnaryFunction::Abs => value.abs(),
        FormulaUnaryFunction::Sign => value.signum(),
        FormulaUnaryFunction::Sqrt => {
            if value < 0.0 {
                f64::NAN
            } else {
                value.sqrt()
            }
        }
        FormulaUnaryFunction::Exp => value.exp(),
        FormulaUnaryFunction::Log => {
            if value <= 0.0 {
                f64::NAN
            } else {
                value.ln()
            }
        }
        FormulaUnaryFunction::Log10 => {
            if value <= 0.0 {
                f64::NAN
            } else {
                value.log10()
            }
        }
        FormulaUnaryFunction::Floor => value.floor(),
        FormulaUnaryFunction::Ceil => value.ceil(),
        FormulaUnaryFunction::Sin => value.sin(),
        FormulaUnaryFunction::Cos => value.cos(),
        FormulaUnaryFunction::Tan => value.tan(),
        FormulaUnaryFunction::Sinh => value.sinh(),
        FormulaUnaryFunction::Cosh => value.cosh(),
        FormulaUnaryFunction::Tanh => value.tanh(),
        FormulaUnaryFunction::Asin => value.asin(),
        FormulaUnaryFunction::Acos => value.acos(),
        FormulaUnaryFunction::Atan => value.atan(),
        FormulaUnaryFunction::Not => {
            if value > 0.0 {
                0.0
            } else {
                1.0
            }
        }
    }
}

fn compile_state(expression: &AstNode) -> FactorResult<(FormulaState, Vec<FormulaStateInput>)> {
    let AstNode::FunctionCall { name, args } = expression else {
        unreachable!("direct_expression validates function calls")
    };
    let normalized = name.to_ascii_uppercase();
    let input = |index: usize| {
        args.get(index)
            .ok_or_else(|| FactorError::InvalidParameter(format!("{name} is missing input")))
            .and_then(FormulaStateInput::from_ast)
    };
    let period = |index: usize, default: usize| -> FactorResult<usize> {
        match args.get(index) {
            None => Ok(default),
            Some(AstNode::Number(value))
                if value.is_finite() && *value >= 1.0 && value.fract() == 0.0 =>
            {
                Ok(*value as usize)
            }
            _ => Err(FactorError::InvalidParameter(format!(
                "{name} period must be a positive integer"
            ))),
        }
    };
    let (state, inputs) = match normalized.as_str() {
        "MA" | "SMA" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Sma {
                    input: value,
                    indicator: StreamingSma::new(period),
                },
                vec![value],
            )
        }
        "WMA" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Wma {
                    input: value,
                    indicator: StreamingWma::new(period),
                },
                vec![value],
            )
        }
        "EMA" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Ema {
                    input: value,
                    indicator: StreamingEma::new(period),
                },
                vec![value],
            )
        }
        "RSI" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Rsi {
                    input: value,
                    indicator: StreamingRsi::new(period),
                },
                vec![value],
            )
        }
        "HHV" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Max {
                    input: value,
                    indicator: StreamingMax::new(period),
                },
                vec![value],
            )
        }
        "LLV" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Min {
                    input: value,
                    indicator: StreamingMin::new(period),
                },
                vec![value],
            )
        }
        "SUM" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            (
                FormulaState::Sum {
                    input: value,
                    indicator: StreamingSum::new(period),
                },
                vec![value],
            )
        }
        "REF" => {
            let value = input(0)?;
            let period = period(1, 1)?;
            (
                FormulaState::Reference {
                    input: value,
                    state: FormulaReferenceState::new(period),
                },
                vec![value],
            )
        }
        "CROSS" | "CROSSBELOW" => {
            let left = input(0)?;
            let right = input(1)?;
            (
                FormulaState::Cross {
                    left,
                    right,
                    direction: if normalized == "CROSS" {
                        FormulaCrossDirection::Above
                    } else {
                        FormulaCrossDirection::Below
                    },
                    previous: None,
                },
                vec![left, right],
            )
        }
        "STD" | "STDDEV" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            if period < 2 {
                return Err(FactorError::InvalidParameter(format!(
                    "{name} period must be at least 2"
                )));
            }
            (
                FormulaState::Variance {
                    input: value,
                    indicator: FormulaRollingVariance::new(period),
                    square_root: true,
                },
                vec![value],
            )
        }
        "VAR" => {
            let value = input(0)?;
            let period = period(1, 14)?;
            if period < 2 {
                return Err(FactorError::InvalidParameter(format!(
                    "{name} period must be at least 2"
                )));
            }
            (
                FormulaState::Variance {
                    input: value,
                    indicator: FormulaRollingVariance::new(period),
                    square_root: false,
                },
                vec![value],
            )
        }
        "ATR" => {
            let high = input(0)?;
            let low = input(1)?;
            let close = input(2)?;
            let period = period(3, 14)?;
            (
                FormulaState::Atr {
                    high,
                    low,
                    close,
                    indicator: FormulaAtrState::new(period),
                },
                vec![high, low, close],
            )
        }
        "MACD" => {
            let value = input(0)?;
            let fast = period(1, 12)?;
            let slow = period(2, 26)?;
            let signal = period(3, 9)?;
            (
                FormulaState::Macd {
                    input: value,
                    indicator: StreamingMacd::new(fast, slow, signal),
                },
                vec![value],
            )
        }
        _ => {
            return Err(FactorError::InvalidParameter(format!(
                "stateful Formula function is unsupported: {name}"
            )))
        }
    };
    Ok((state, inputs))
}

fn formula_signature(source: &str, dialect: FormulaDialect) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("formula_stateful_v1|{}|{source}", dialect.as_str()).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::FormulaDialect;

    fn assert_same(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        assert!(actual.iter().zip(expected).all(|(left, right)| {
            (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-10
        }));
    }

    #[test]
    fn direct_formula_state_matches_batch_and_checkpoint() {
        let close: Vec<f64> = (0..80)
            .map(|index| 100.0 + index as f64 * 0.2 + (index as f64 * 0.17).sin())
            .collect();
        let mut stream =
            FormulaStatefulStream::from_source("EMA(CLOSE, 5)", FormulaDialect::TongDaXin).unwrap();
        let mut actual = Vec::with_capacity(close.len());
        let mut output = Vec::new();
        let inputs = BTreeMap::from([(String::from("close"), close.clone())]);
        stream.push_batch_into(&inputs, &mut output).unwrap();

        let mut batch_context = crate::formula::FormulaContext::new(
            ndarray::Array1::from_vec(close.clone()),
            ndarray::Array1::from_vec(close.clone()),
            ndarray::Array1::from_vec(close.clone()),
            ndarray::Array1::from_vec(close.clone()),
            ndarray::Array1::from_vec(close.clone()),
            None,
        );
        let mut engine = crate::formula::FormulaEngine::new();
        let expected = engine
            .eval_with_dialect(
                "EMA(CLOSE, 5)",
                FormulaDialect::TongDaXin,
                &mut batch_context,
            )
            .unwrap();
        actual.extend_from_slice(&output);
        assert_same(&actual, expected.as_slice().unwrap());

        let checkpoint = stream.checkpoint();
        let mut next_output = 0.0;
        stream
            .push_values_into(&[0.0; 6], &mut next_output)
            .unwrap();
        stream.restore(&checkpoint).unwrap();
        let mut restored = 0.0;
        stream.push_values_into(&[0.0; 6], &mut restored).unwrap();
        assert_eq!(next_output, restored);
    }

    #[test]
    fn stateful_formula_supports_serializable_programs_and_rejects_unknown_inputs() {
        let close = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        for source in [
            "X:=EMA(CLOSE,5); X",
            "MA3:MA(CLOSE,3); MA3",
            "PREV:=REF(CLOSE,1); PREV",
            "X:=CLOSE; X+=1; X",
        ] {
            let mut stream =
                FormulaStatefulStream::from_source(source, FormulaDialect::TongDaXin).unwrap();
            let mut actual = Vec::new();
            stream
                .push_batch_into(
                    &BTreeMap::from([(String::from("close"), close.clone())]),
                    &mut actual,
                )
                .unwrap();
            let mut context = crate::formula::FormulaContext::new(
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                None,
            );
            let mut engine = crate::formula::FormulaEngine::new();
            let expected = engine
                .eval_with_dialect(source, FormulaDialect::TongDaXin, &mut context)
                .unwrap();
            assert_same(&actual, expected.as_slice().unwrap());
        }
        assert!(
            FormulaStatefulStream::from_source("EMA(MY_SERIES,5)", FormulaDialect::AlphaTA)
                .is_err()
        );
    }

    #[test]
    fn rolling_formula_state_matches_batch_for_common_domestic_functions() {
        let close: Vec<f64> = (0..48)
            .map(|index| 20.0 + (index as f64 * 0.37).sin() * 3.0 + index as f64 * 0.05)
            .collect();
        for source in [
            "HHV(CLOSE, 5)",
            "LLV(CLOSE, 5)",
            "SUM(CLOSE, 5)",
            "STD(CLOSE, 5)",
            "STDDEV(CLOSE, 5)",
            "VAR(CLOSE, 5)",
        ] {
            let mut stream =
                FormulaStatefulStream::from_source(source, FormulaDialect::TongDaXin).unwrap();
            let mut actual = Vec::new();
            stream
                .push_batch_into(
                    &BTreeMap::from([(String::from("close"), close.clone())]),
                    &mut actual,
                )
                .unwrap();

            let mut context = crate::formula::FormulaContext::new(
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                None,
            );
            let mut engine = crate::formula::FormulaEngine::new();
            let expected = engine
                .eval_with_dialect(source, FormulaDialect::TongDaXin, &mut context)
                .unwrap();
            let expected = expected.as_slice().unwrap();
            assert_eq!(actual.len(), expected.len(), "{source} length");
            for (index, (left, right)) in actual.iter().zip(expected).enumerate() {
                assert!(
                    (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-10,
                    "{source} mismatch at {index}: stateful={left:?}, batch={right:?}"
                );
            }
        }
    }

    #[test]
    fn reference_and_cross_formula_state_matches_batch() {
        let close = vec![1.0, 2.0, 1.0, 3.0, 2.0, 4.0];
        let open = vec![1.0, 1.5, 1.5, 2.0, 2.5, 3.0];
        for source in [
            "REF(CLOSE, 2)",
            "CROSS(CLOSE, OPEN)",
            "CROSSBELOW(CLOSE, OPEN)",
        ] {
            let mut stream =
                FormulaStatefulStream::from_source(source, FormulaDialect::TongDaXin).unwrap();
            let mut actual = Vec::new();
            stream
                .push_batch_into(
                    &BTreeMap::from([
                        (String::from("close"), close.clone()),
                        (String::from("open"), open.clone()),
                    ]),
                    &mut actual,
                )
                .unwrap();

            let mut context = crate::formula::FormulaContext::new(
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                None,
            );
            let mut engine = crate::formula::FormulaEngine::new();
            let expected = engine
                .eval_with_dialect(source, FormulaDialect::TongDaXin, &mut context)
                .unwrap();
            assert_same(&actual, expected.as_slice().unwrap());
        }
    }

    #[test]
    fn expression_formula_state_matches_batch_for_common_operators() {
        let close = vec![10.0, 11.0, 9.0, 13.0, 12.0, 15.0];
        let open = vec![10.5, 10.0, 9.5, 11.0, 13.0, 14.0];
        for source in [
            "MA(CLOSE, 3) + 1",
            "IF(CLOSE > OPEN, CLOSE, OPEN)",
            "ABS(CLOSE - OPEN)",
            "IF(CLOSE > MA(CLOSE, 3), 1, 0)",
            "CROSS(MA(CLOSE, 2), MA(CLOSE, 3))",
            "REF(MA(CLOSE, 3), 1)",
            "HHV(MA(CLOSE, 2), 3)",
        ] {
            let mut stream =
                FormulaStatefulStream::from_source(source, FormulaDialect::TongDaXin).unwrap();
            let mut actual = Vec::new();
            stream
                .push_batch_into(
                    &BTreeMap::from([
                        (String::from("close"), close.clone()),
                        (String::from("open"), open.clone()),
                    ]),
                    &mut actual,
                )
                .unwrap();

            let mut context = crate::formula::FormulaContext::new(
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(open.clone()),
                ndarray::Array1::from_vec(close.clone()),
                ndarray::Array1::from_vec(close.clone()),
                None,
            );
            let mut engine = crate::formula::FormulaEngine::new();
            let expected = engine
                .eval_with_dialect(source, FormulaDialect::TongDaXin, &mut context)
                .unwrap();
            assert_same(&actual, expected.as_slice().unwrap());
        }
    }
}
