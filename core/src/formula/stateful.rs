//! Explicit stateful streaming for the portable Formula subset.
//!
//! Formula batch evaluation remains the source of truth for the complete
//! language. This module provides a deliberately narrow, serializable state
//! contract for direct recursive indicators whose row kernel is already
//! verified against the batch implementation. Complex assignments, drawing,
//! control flow, future-data functions and host-dependent calls are rejected
//! instead of being silently approximated.

use super::ast::AstNode;
use super::{normalize_formula_source, parse_formula_with_dialect, FormulaDialect};
use crate::factors::{FactorError, FactorResult};
use crate::formula::types::{classify_builtin_var, BuiltinVar};
use crate::streaming::indicators::{StreamingEma, StreamingRsi, StreamingSma, StreamingWma};
use crate::streaming::momentum::macd::StreamingMacd;
use crate::streaming::traits::StreamingIndicator;
use std::collections::BTreeMap;

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

/// O(1)-per-row stateful Formula executor for the verified direct subset.
#[derive(Clone)]
pub struct FormulaStatefulStream {
    signature: u64,
    required_inputs: Vec<FormulaStateInput>,
    state: FormulaState,
    row_count: usize,
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
    /// Compile a direct stateful Formula using an explicit dialect profile.
    pub fn from_source(source: &str, dialect: FormulaDialect) -> FactorResult<Self> {
        let normalized = normalize_formula_source(source, dialect);
        let ast = parse_formula_with_dialect(&normalized, dialect)
            .map_err(|error| FactorError::InvalidParameter(error.to_string()))?;
        let expression = direct_expression(&ast)?;
        let (state, required_inputs) = compile_state(expression)?;
        Ok(Self {
            signature: formula_signature(&normalized, dialect),
            required_inputs,
            state,
            row_count: 0,
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
        Ok(())
    }

    fn next_row(&mut self, row: [f64; 6]) -> f64 {
        let value = self.state.next(&row);
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
    fn next(&mut self, row: &[f64; 6]) -> f64 {
        let value = match self {
            Self::Sma { input, indicator } => indicator.next(row[input.slot()]),
            Self::Wma { input, indicator } => indicator.next(row[input.slot()]),
            Self::Ema { input, indicator } => indicator.next(row[input.slot()]),
            Self::Rsi { input, indicator } => indicator.next(row[input.slot()]),
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

fn direct_expression(ast: &AstNode) -> FactorResult<&AstNode> {
    match ast {
        AstNode::FunctionCall { .. } => Ok(ast),
        AstNode::Output { expr, .. } => direct_expression(expr),
        AstNode::Statements(nodes) if nodes.len() == 1 => direct_expression(&nodes[0]),
        _ => Err(FactorError::InvalidParameter(
            "stateful Formula requires one direct indicator expression; assignments, drawing, control flow and future-data calls are not supported"
                .to_string(),
        )),
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
    fn stateful_formula_rejects_assignments_and_unknown_inputs() {
        assert!(FormulaStatefulStream::from_source(
            "X:=EMA(CLOSE,5); X",
            FormulaDialect::TongDaXin
        )
        .is_err());
        assert!(
            FormulaStatefulStream::from_source("EMA(MY_SERIES,5)", FormulaDialect::AlphaTA)
                .is_err()
        );
    }
}
