//! Stateful Composite execution for recursive and rolling built-ins.
//!
//! `CompositeStream` is intentionally limited to finite replay windows. This
//! module is the separate stateful path: it compiles a supported Composite
//! graph into a row-oriented state DAG and advances each state exactly once per
//! input row. The hot path uses fixed node ids and caller-owned output buffers;
//! it does not rebuild a string-keyed context or replay historical rows.

use crate::composite::{
    CompiledCompositePlan, CompositeDefinition, CompositeExpr, CompositeOp, StatefulCompositeSpec,
};
use crate::factors::{FactorError, FactorResult};
use crate::returns::{return_between, ReturnKind};
use crate::streaming::indicators::{
    StreamingBoll, StreamingEma, StreamingMacd, StreamingMax, StreamingMin, StreamingRsi,
    StreamingSma, StreamingStdDev, StreamingWma,
};
use crate::streaming::traits::StreamingIndicator;
use std::collections::{BTreeMap, VecDeque};

/// Stateful Composite executor compiled from a validated graph plan.
///
/// The executor owns one state object per recursive/rolling call and advances
/// the graph in dependency order. It is suitable for long-running feeds where
/// retaining the entire input history is not acceptable.
#[derive(Clone)]
pub struct StatefulCompositeStream {
    signature: u64,
    required_raw_inputs: Vec<String>,
    outputs: Vec<(String, usize)>,
    nodes: Vec<StatefulNode>,
    values: Vec<f64>,
    raw_scratch: Vec<f64>,
    input_scratch: Vec<f64>,
    output_scratch: Vec<f64>,
    row_count: usize,
}

/// Persistable state image for [`StatefulCompositeStream`].
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StatefulCompositeCheckpoint {
    signature: u64,
    row_count: usize,
    nodes: Vec<StatefulNode>,
}

impl StatefulCompositeCheckpoint {
    /// Stable graph signature associated with the state image.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Number of rows already consumed by the state image.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Serialize this state image for a language adapter when serde support is
    /// enabled. The serialized form is versioned by the enclosing API contract.
    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|error| error.to_string())
    }

    /// Restore a state image received from a language adapter.
    #[cfg(feature = "serde")]
    pub fn from_json(value: &str) -> Result<Self, String> {
        serde_json::from_str(value).map_err(|error| error.to_string())
    }
}

impl StatefulCompositeStream {
    /// Compile a stateful executor from a validated Composite plan.
    pub fn from_plan(plan: &CompiledCompositePlan) -> FactorResult<Self> {
        if plan.required_raw_inputs.is_empty() {
            return Err(FactorError::InvalidParameter(
                "stateful composite streaming requires at least one raw input series".to_string(),
            ));
        }

        let definitions = plan
            .definitions
            .iter()
            .map(|definition| (definition.name.as_str(), definition))
            .collect::<BTreeMap<_, _>>();
        let raw_indexes = plan
            .required_raw_inputs
            .iter()
            .enumerate()
            .map(|(index, name)| (name.as_str(), index))
            .collect::<BTreeMap<_, _>>();
        let mut builder = StatefulBuilder {
            definitions,
            raw_indexes,
            stateful_specs: &plan.stateful_specs,
            named: BTreeMap::new(),
            visiting: Vec::new(),
            nodes: Vec::new(),
        };
        let outputs = plan
            .outputs
            .iter()
            .map(|name| builder.named_node(name).map(|node| (name.clone(), node)))
            .collect::<FactorResult<Vec<_>>>()?;

        Ok(Self {
            signature: plan.signature,
            required_raw_inputs: plan.required_raw_inputs.clone(),
            outputs,
            values: vec![f64::NAN; builder.nodes.len()],
            raw_scratch: vec![f64::NAN; plan.required_raw_inputs.len()],
            input_scratch: vec![
                f64::NAN;
                builder
                    .nodes
                    .iter()
                    .map(StatefulNode::input_count)
                    .max()
                    .unwrap_or(0)
            ],
            output_scratch: vec![f64::NAN; plan.outputs.len()],
            nodes: builder.nodes,
            row_count: 0,
        })
    }

    /// Stable graph signature for this executor.
    #[must_use]
    pub const fn signature(&self) -> u64 {
        self.signature
    }

    /// Raw input names and their required row order.
    #[must_use]
    pub fn required_raw_inputs(&self) -> &[String] {
        &self.required_raw_inputs
    }

    /// Requested output names in graph order.
    #[must_use]
    pub fn outputs(&self) -> impl Iterator<Item = &str> {
        self.outputs.iter().map(|(name, _)| name.as_str())
    }

    /// Number of rows consumed by this executor.
    #[must_use]
    pub const fn rows(&self) -> usize {
        self.row_count
    }

    /// Advance one row into caller-owned output slots.
    ///
    /// `values` follows [`Self::required_raw_inputs`], and `outputs` follows
    /// [`Self::outputs`]. No heap allocation occurs after the executor is
    /// constructed.
    pub fn push_values_into(&mut self, values: &[f64], outputs: &mut [f64]) -> FactorResult<()> {
        if values.len() != self.raw_scratch.len() {
            return Err(FactorError::LengthMismatch {
                name: "stateful_composite_row".to_string(),
                expected: self.raw_scratch.len(),
                actual: values.len(),
            });
        }
        if outputs.len() != self.output_scratch.len() {
            return Err(FactorError::LengthMismatch {
                name: "stateful_composite_output".to_string(),
                expected: self.output_scratch.len(),
                actual: outputs.len(),
            });
        }
        self.raw_scratch.copy_from_slice(values);
        self.advance_row()?;
        outputs.copy_from_slice(&self.output_scratch);
        Ok(())
    }

    /// Advance one row using a reusable named output map.
    pub fn push_row_into(
        &mut self,
        values: &BTreeMap<String, f64>,
        outputs: &mut BTreeMap<String, f64>,
    ) -> FactorResult<()> {
        for (index, name) in self.required_raw_inputs.iter().enumerate() {
            self.raw_scratch[index] = values
                .get(name)
                .copied()
                .ok_or_else(|| FactorError::MissingInput(name.clone()))?;
        }
        self.advance_row()?;
        for (index, (name, _)) in self.outputs.iter().enumerate() {
            outputs.insert(name.clone(), self.output_scratch[index]);
        }
        Ok(())
    }

    /// Append an aligned batch into reusable output vectors.
    ///
    /// Input validation happens before state advancement. The output vectors
    /// are cleared and reused on success; capacity is retained across calls.
    pub fn push_batch_into(
        &mut self,
        values: &BTreeMap<String, Vec<f64>>,
        emitted: &mut BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<()> {
        let rows = self
            .required_raw_inputs
            .first()
            .and_then(|name| values.get(name))
            .ok_or_else(|| {
                FactorError::MissingInput(
                    self.required_raw_inputs
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "stateful_composite_input".to_string()),
                )
            })?
            .len();
        for name in &self.required_raw_inputs {
            let series = values
                .get(name)
                .ok_or_else(|| FactorError::MissingInput(name.clone()))?;
            if series.len() != rows {
                return Err(FactorError::LengthMismatch {
                    name: name.clone(),
                    expected: rows,
                    actual: series.len(),
                });
            }
        }

        for name in self.outputs() {
            let output = emitted.entry(name.to_string()).or_default();
            output.clear();
            output.reserve(rows);
        }
        for row in 0..rows {
            for (index, name) in self.required_raw_inputs.iter().enumerate() {
                self.raw_scratch[index] = values[name][row];
            }
            self.advance_row()?;
            for (index, (name, _)) in self.outputs.iter().enumerate() {
                emitted
                    .get_mut(name)
                    .expect("stateful output initialized")
                    .push(self.output_scratch[index]);
            }
        }
        Ok(())
    }

    /// Capture a cloneable state checkpoint without retaining raw history.
    #[must_use]
    pub fn checkpoint(&self) -> StatefulCompositeCheckpoint {
        StatefulCompositeCheckpoint {
            signature: self.signature,
            row_count: self.row_count,
            nodes: self.nodes.clone(),
        }
    }

    /// Restore a checkpoint created by the same compiled graph.
    pub fn restore(&mut self, checkpoint: &StatefulCompositeCheckpoint) -> FactorResult<()> {
        if checkpoint.signature != self.signature {
            return Err(FactorError::InvalidParameter(
                "stateful composite checkpoint belongs to a different graph".to_string(),
            ));
        }
        if checkpoint.nodes.len() != self.nodes.len() {
            return Err(FactorError::InvalidParameter(
                "stateful composite checkpoint node layout does not match graph".to_string(),
            ));
        }
        self.nodes.clone_from(&checkpoint.nodes);
        self.values.fill(f64::NAN);
        self.raw_scratch.fill(f64::NAN);
        self.output_scratch.fill(f64::NAN);
        self.row_count = checkpoint.row_count;
        Ok(())
    }

    fn advance_row(&mut self) -> FactorResult<()> {
        for (index, node) in self.nodes.iter_mut().enumerate() {
            self.values[index] =
                node.next(&self.values, &self.raw_scratch, &mut self.input_scratch)?;
        }
        for (index, (_, node)) in self.outputs.iter().enumerate() {
            self.output_scratch[index] = self.values[*node];
        }
        self.row_count = self.row_count.saturating_add(1);
        Ok(())
    }
}

struct StatefulBuilder<'a> {
    definitions: BTreeMap<&'a str, &'a CompositeDefinition>,
    raw_indexes: BTreeMap<&'a str, usize>,
    stateful_specs: &'a BTreeMap<String, StatefulCompositeSpec>,
    named: BTreeMap<String, usize>,
    visiting: Vec<String>,
    nodes: Vec<StatefulNode>,
}

impl StatefulBuilder<'_> {
    fn named_node(&mut self, name: &str) -> FactorResult<usize> {
        if let Some(node) = self.named.get(name).copied() {
            return Ok(node);
        }
        if self.visiting.iter().any(|current| current == name) {
            return Err(FactorError::DependencyCycle(self.visiting.clone()));
        }
        let expression = self
            .definitions
            .get(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?
            .expression
            .clone();
        self.visiting.push(name.to_string());
        let node = self.expression_node(&expression)?;
        self.visiting.pop();
        self.named.insert(name.to_string(), node);
        Ok(node)
    }

    fn expression_node(&mut self, expression: &CompositeExpr) -> FactorResult<usize> {
        let stateful_node = match expression {
            CompositeExpr::Series(name) => StatefulNode::Raw {
                input: *self
                    .raw_indexes
                    .get(name.as_str())
                    .ok_or_else(|| FactorError::MissingInput(name.clone()))?,
            },
            CompositeExpr::Constant(value) => StatefulNode::Constant { value: *value },
            CompositeExpr::Ref(name) => {
                return self.named_node(name);
            }
            CompositeExpr::Op { op, inputs } => StatefulNode::Op {
                op: *op,
                inputs: inputs
                    .iter()
                    .map(|input| self.expression_node(input))
                    .collect::<FactorResult<Vec<_>>>()?,
            },
            CompositeExpr::Call {
                name,
                inputs,
                params,
            } => {
                let input_nodes = inputs
                    .iter()
                    .map(|input| self.expression_node(input))
                    .collect::<FactorResult<Vec<_>>>()?;
                let spec = self.stateful_specs.get(&name.to_ascii_lowercase()).copied();
                let state = StatefulCall::new(name, spec, params)?;
                state.validate_arity(input_nodes.len())?;
                StatefulNode::Call {
                    inputs: input_nodes,
                    state,
                }
            }
        };
        let node = self.nodes.len();
        self.nodes.push(stateful_node);
        Ok(node)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum StatefulNode {
    Raw {
        input: usize,
    },
    Constant {
        value: f64,
    },
    Op {
        op: CompositeOp,
        inputs: Vec<usize>,
    },
    Call {
        inputs: Vec<usize>,
        state: StatefulCall,
    },
}

impl StatefulNode {
    fn input_count(&self) -> usize {
        match self {
            Self::Raw { .. } | Self::Constant { .. } => 0,
            Self::Op { inputs, .. } | Self::Call { inputs, .. } => inputs.len(),
        }
    }

    fn next(&mut self, values: &[f64], raw: &[f64], scratch: &mut [f64]) -> FactorResult<f64> {
        match self {
            Self::Raw { input } => Ok(raw[*input]),
            Self::Constant { value } => Ok(*value),
            Self::Op { op, inputs } => {
                for (index, input) in inputs.iter().copied().enumerate() {
                    scratch[index] = values[input];
                }
                apply_row_op(*op, &scratch[..inputs.len()])
            }
            Self::Call { inputs, state } => {
                for (index, input) in inputs.iter().copied().enumerate() {
                    scratch[index] = values[input];
                }
                state.next(&scratch[..inputs.len()])
            }
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum StatefulCall {
    Sma(StreamingSma),
    Wma(StreamingWma),
    Ema(StreamingEma),
    Rsi(StreamingRsi),
    Atr(AtrState),
    Macd(StreamingMacd),
    Boll {
        indicator: StreamingBoll,
        field: BollField,
    },
    StdDev(StreamingStdDev),
    Min(StreamingMin),
    Max(StreamingMax),
    Return(LaggedState),
    Vwma(VwmaState),
    Volatility(VolatilityState),
    Threshold(f64),
    Between {
        lower: f64,
        upper: f64,
    },
    Clip {
        lower: f64,
        upper: f64,
    },
    Abs,
    Neg,
    Sign,
    WeightedAverage {
        weights: Vec<f64>,
    },
    Cross {
        upward: bool,
        previous: Option<(f64, f64)>,
    },
}

#[derive(Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
enum BollField {
    Middle,
    Upper,
    Lower,
}

impl StatefulCall {
    fn new(name: &str, spec: Option<StatefulCompositeSpec>, params: &[f64]) -> FactorResult<Self> {
        let spec = spec.ok_or_else(|| {
            FactorError::InvalidParameter(format!(
                "stateful composite function has no registered state spec: {name}"
            ))
        })?;
        let state = match spec {
            StatefulCompositeSpec::Sma => Self::Sma(StreamingSma::new(period(params, 14)?)),
            StatefulCompositeSpec::Wma => Self::Wma(StreamingWma::new(period(params, 14)?)),
            StatefulCompositeSpec::Ema => Self::Ema(StreamingEma::new(period(params, 14)?)),
            StatefulCompositeSpec::Rsi => Self::Rsi(StreamingRsi::new(period(params, 14)?)),
            StatefulCompositeSpec::Atr => Self::Atr(AtrState::new(period(params, 14)?)),
            StatefulCompositeSpec::Macd => Self::Macd(StreamingMacd::new(
                period_at(params, 0, 12)?,
                period_at(params, 1, 26)?,
                period_at(params, 2, 9)?,
            )),
            StatefulCompositeSpec::BollMid => Self::Boll {
                indicator: StreamingBoll::new(period(params, 20)?, 2.0, 2.0),
                field: BollField::Middle,
            },
            StatefulCompositeSpec::BollUpper => Self::Boll {
                indicator: StreamingBoll::new(
                    period(params, 20)?,
                    params.get(1).copied().unwrap_or(2.0),
                    params.get(1).copied().unwrap_or(2.0),
                ),
                field: BollField::Upper,
            },
            StatefulCompositeSpec::BollLower => Self::Boll {
                indicator: StreamingBoll::new(
                    period(params, 20)?,
                    params.get(1).copied().unwrap_or(2.0),
                    params.get(1).copied().unwrap_or(2.0),
                ),
                field: BollField::Lower,
            },
            StatefulCompositeSpec::RollingStd => {
                Self::StdDev(StreamingStdDev::new(period(params, 20)?))
            }
            StatefulCompositeSpec::RollingMin => Self::Min(StreamingMin::new(period(params, 20)?)),
            StatefulCompositeSpec::RollingMax => Self::Max(StreamingMax::new(period(params, 20)?)),
            StatefulCompositeSpec::Return => Self::Return(LaggedState::new(period(params, 1)?)),
            StatefulCompositeSpec::Vwma => Self::Vwma(VwmaState::new(period(params, 14)?)),
            StatefulCompositeSpec::Volatility => {
                Self::Volatility(VolatilityState::new(period(params, 20)?))
            }
            StatefulCompositeSpec::Threshold => Self::Threshold(finite_param(
                params.first().copied().unwrap_or(0.0),
                "threshold",
            )?),
            StatefulCompositeSpec::Between => {
                let lower = finite_param(params.first().copied().unwrap_or(0.0), "between lower")?;
                let upper = finite_param(params.get(1).copied().unwrap_or(1.0), "between upper")?;
                if lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "between bounds must be finite and ordered".to_string(),
                    ));
                }
                Self::Between { lower, upper }
            }
            StatefulCompositeSpec::Clip => {
                let lower =
                    finite_param(params.first().copied().unwrap_or(-f64::MAX), "clip lower")?;
                let upper = finite_param(params.get(1).copied().unwrap_or(f64::MAX), "clip upper")?;
                if lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "clip bounds must be finite and ordered".to_string(),
                    ));
                }
                Self::Clip { lower, upper }
            }
            StatefulCompositeSpec::Abs => Self::Abs,
            StatefulCompositeSpec::Neg => Self::Neg,
            StatefulCompositeSpec::Sign => Self::Sign,
            StatefulCompositeSpec::WeightedAverage => {
                if params.iter().any(|weight| !weight.is_finite()) {
                    return Err(FactorError::InvalidParameter(
                        "weights must be finite".to_string(),
                    ));
                }
                Self::WeightedAverage {
                    weights: params.to_vec(),
                }
            }
            StatefulCompositeSpec::CrossUp => Self::Cross {
                upward: true,
                previous: None,
            },
            StatefulCompositeSpec::CrossDown => Self::Cross {
                upward: false,
                previous: None,
            },
        };
        Ok(state)
    }

    fn validate_arity(&self, actual: usize) -> FactorResult<()> {
        let expected = match self {
            Self::Sma(_)
            | Self::Wma(_)
            | Self::Ema(_)
            | Self::Rsi(_)
            | Self::Macd(_)
            | Self::Boll { .. }
            | Self::StdDev(_)
            | Self::Min(_)
            | Self::Max(_)
            | Self::Return(_)
            | Self::Volatility(_)
            | Self::Threshold(_)
            | Self::Between { .. }
            | Self::Clip { .. }
            | Self::Abs
            | Self::Neg
            | Self::Sign => Some(1),
            Self::Atr(_) => Some(3),
            Self::Vwma(_) => Some(2),
            Self::Cross { .. } => Some(2),
            Self::WeightedAverage { .. } => None,
        };
        if expected.is_some_and(|expected| expected != actual)
            || matches!(self, Self::WeightedAverage { weights } if actual == 0 || (!weights.is_empty() && weights.len() != actual))
        {
            return Err(FactorError::InvalidParameter(format!(
                "stateful composite function input count is invalid: expected {:?}, got {actual}",
                expected
            )));
        }
        Ok(())
    }

    fn next(&mut self, inputs: &[f64]) -> FactorResult<f64> {
        let output = match self {
            Self::Cross { upward, previous } => {
                let current = match inputs {
                    [left, right] => (*left, *right),
                    _ => {
                        return Err(FactorError::InvalidParameter(
                            "cross requires two inputs".to_string(),
                        ))
                    }
                };
                let value = previous.map_or(0.0, |(left, right)| {
                    if [left, right, current.0, current.1]
                        .iter()
                        .all(|value| value.is_finite())
                    {
                        if *upward {
                            (left <= right && current.0 > current.1) as u8 as f64
                        } else {
                            (left >= right && current.0 < current.1) as u8 as f64
                        }
                    } else {
                        0.0
                    }
                });
                *previous = Some(current);
                value
            }
            Self::Threshold(level) => {
                unary_finite(inputs, "threshold").map(|value| (value >= *level) as u8 as f64)?
            }
            Self::Between { lower, upper } => unary_finite(inputs, "between")
                .map(|value| (value >= *lower && value <= *upper) as u8 as f64)?,
            Self::Clip { lower, upper } => {
                unary_finite(inputs, "clip").map(|value| value.clamp(*lower, *upper))?
            }
            Self::Abs => unary_finite(inputs, "abs").map(f64::abs)?,
            Self::Neg => unary_finite(inputs, "neg").map(|value| -value)?,
            Self::Sign => unary_finite(inputs, "sign").map(f64::signum)?,
            Self::WeightedAverage { weights } => {
                if inputs.is_empty() {
                    return Err(FactorError::InvalidParameter(
                        "weighted_average needs inputs".to_string(),
                    ));
                }
                if !weights.is_empty() && weights.len() != inputs.len() {
                    return Err(FactorError::InvalidParameter(
                        "weights must match input count".to_string(),
                    ));
                }
                if inputs.iter().any(|value| !value.is_finite()) {
                    f64::NAN
                } else {
                    let total = if weights.is_empty() {
                        inputs.len() as f64
                    } else {
                        weights.iter().map(|value| value.abs()).sum()
                    };
                    if total == 0.0 {
                        f64::NAN
                    } else {
                        let weights = if weights.is_empty() {
                            &[][..]
                        } else {
                            weights.as_slice()
                        };
                        if weights.is_empty() {
                            inputs.iter().sum::<f64>() / total
                        } else {
                            inputs
                                .iter()
                                .zip(weights.iter())
                                .map(|(input, weight)| input * weight)
                                .sum::<f64>()
                                / total
                        }
                    }
                }
            }
            Self::Sma(state) => {
                state.next_checked(inputs, "sma", |state, input| state.next(input))?
            }
            Self::Wma(state) => {
                state.next_checked(inputs, "wma", |state, input| state.next(input))?
            }
            Self::Ema(state) => {
                state.next_checked(inputs, "ema", |state, input| state.next(input))?
            }
            Self::Rsi(state) => {
                state.next_checked(inputs, "rsi", |state, input| state.next(input))?
            }
            Self::StdDev(state) => {
                state.next_checked(inputs, "rolling_std", |state, input| state.next(input))?
            }
            Self::Min(state) => {
                state.next_checked(inputs, "rolling_min", |state, input| state.next(input))?
            }
            Self::Max(state) => {
                state.next_checked(inputs, "rolling_max", |state, input| state.next(input))?
            }
            Self::Return(state) => state.next_checked(inputs)?,
            Self::Volatility(state) => state.next_checked(inputs)?,
            Self::Vwma(state) => state.next_checked(inputs)?,
            Self::Atr(state) => state.next_checked(inputs)?,
            Self::Macd(state) => {
                let value = unary_finite(inputs, "macd")?;
                if !value.is_finite() {
                    state.reset();
                    f64::NAN
                } else {
                    state
                        .next(value)
                        .map_or(f64::NAN, |output| output.histogram)
                }
            }
            Self::Boll { indicator, field } => {
                let value = unary_finite(inputs, "boll")?;
                if !value.is_finite() {
                    indicator.reset();
                    f64::NAN
                } else {
                    indicator
                        .next(value)
                        .map_or(f64::NAN, |output| match field {
                            BollField::Middle => output.middle,
                            BollField::Upper => output.upper,
                            BollField::Lower => output.lower,
                        })
                }
            }
        };
        Ok(output)
    }
}

trait CheckedNext {
    fn next_checked<F>(&mut self, inputs: &[f64], name: &str, next: F) -> FactorResult<f64>
    where
        F: FnOnce(&mut Self, f64) -> Option<f64>;
}

impl<T> CheckedNext for T
where
    T: StreamingIndicator,
{
    fn next_checked<F>(&mut self, inputs: &[f64], name: &str, next: F) -> FactorResult<f64>
    where
        F: FnOnce(&mut Self, f64) -> Option<f64>,
    {
        let value = unary_finite(inputs, name)?;
        if !value.is_finite() {
            self.reset();
            Ok(f64::NAN)
        } else {
            Ok(next(self, value).unwrap_or(f64::NAN))
        }
    }
}

/// TA-Lib-compatible ATR state. The first bar establishes the previous
/// close; the first true range is therefore row 1, and the first ATR is row
/// `period`, matching the batch indicator contract.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct AtrState {
    period: usize,
    previous_close: Option<f64>,
    tr_count: usize,
    tr_sum: f64,
    atr: f64,
}

impl AtrState {
    fn new(period: usize) -> Self {
        Self {
            period,
            previous_close: None,
            tr_count: 0,
            tr_sum: 0.0,
            atr: f64::NAN,
        }
    }

    fn next_checked(&mut self, inputs: &[f64]) -> FactorResult<f64> {
        let [high, low, close] = inputs else {
            return Err(FactorError::InvalidParameter(
                "atr requires high, low and close".to_string(),
            ));
        };
        if ![high, low, close].iter().all(|value| value.is_finite()) {
            self.previous_close = None;
            self.tr_count = 0;
            self.tr_sum = 0.0;
            self.atr = f64::NAN;
            return Ok(f64::NAN);
        }
        let Some(previous_close) = self.previous_close.replace(*close) else {
            return Ok(f64::NAN);
        };
        let true_range = (*high - *low)
            .max((*high - previous_close).abs())
            .max((*low - previous_close).abs());
        self.tr_count += 1;
        if self.tr_count < self.period {
            self.tr_sum += true_range;
            return Ok(f64::NAN);
        }
        if self.tr_count == self.period {
            self.tr_sum += true_range;
            self.atr = self.tr_sum / self.period as f64;
        } else {
            self.atr += (true_range - self.atr) / self.period as f64;
        }
        Ok(self.atr)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct LaggedState {
    period: usize,
    values: VecDeque<f64>,
}

impl LaggedState {
    fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
        }
    }

    fn next_checked(&mut self, inputs: &[f64]) -> FactorResult<f64> {
        let value = unary_finite(inputs, "return")?;
        if !value.is_finite() {
            self.values.clear();
            return Ok(f64::NAN);
        }
        let result = if self.values.len() < self.period {
            f64::NAN
        } else {
            return_between(
                *self.values.front().expect("lagged state is ready"),
                value,
                ReturnKind::Arithmetic,
            )
        };
        self.values.push_back(value);
        if self.values.len() > self.period {
            self.values.pop_front();
        }
        Ok(result)
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct VwmaState {
    period: usize,
    values: VecDeque<(f64, f64)>,
    sum_product: f64,
    sum_volume: f64,
}

impl VwmaState {
    fn new(period: usize) -> Self {
        Self {
            period,
            values: VecDeque::with_capacity(period),
            sum_product: 0.0,
            sum_volume: 0.0,
        }
    }

    fn next_checked(&mut self, inputs: &[f64]) -> FactorResult<f64> {
        let [value, volume] = inputs else {
            return Err(FactorError::InvalidParameter(
                "vwma requires input and volume".to_string(),
            ));
        };
        if !value.is_finite() || !volume.is_finite() {
            self.values.clear();
            self.sum_product = 0.0;
            self.sum_volume = 0.0;
            return Ok(f64::NAN);
        }
        let product = value * volume;
        self.values.push_back((product, *volume));
        self.sum_product += product;
        self.sum_volume += volume;
        if self.values.len() > self.period {
            let (old_product, old_volume) = self.values.pop_front().expect("vwma state");
            self.sum_product -= old_product;
            self.sum_volume -= old_volume;
        }
        if self.values.len() == self.period && self.sum_volume.abs() > 1e-15 {
            Ok(self.sum_product / self.sum_volume)
        } else {
            Ok(f64::NAN)
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct VolatilityState {
    period: usize,
    previous: Option<f64>,
    returns: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

impl VolatilityState {
    fn new(period: usize) -> Self {
        Self {
            period,
            previous: None,
            returns: VecDeque::with_capacity(period),
            sum: 0.0,
            sum_sq: 0.0,
        }
    }

    fn next_checked(&mut self, inputs: &[f64]) -> FactorResult<f64> {
        let value = unary_finite(inputs, "volatility")?;
        if !value.is_finite() {
            self.previous = None;
            self.returns.clear();
            self.sum = 0.0;
            self.sum_sq = 0.0;
            return Ok(f64::NAN);
        }
        let Some(previous) = self.previous.replace(value) else {
            return Ok(f64::NAN);
        };
        let current = return_between(previous, value, ReturnKind::Arithmetic);
        self.returns.push_back(current);
        self.sum += current;
        self.sum_sq += current * current;
        if self.returns.len() > self.period {
            let old = self.returns.pop_front().expect("volatility state");
            self.sum -= old;
            self.sum_sq -= old * old;
        }
        if self.returns.len() == self.period {
            let mean = self.sum / self.period as f64;
            Ok(((self.sum_sq - self.sum * mean) / self.period as f64)
                .max(0.0)
                .sqrt())
        } else {
            Ok(f64::NAN)
        }
    }
}

fn apply_row_op(op: CompositeOp, values: &[f64]) -> FactorResult<f64> {
    if values.is_empty() {
        return Err(FactorError::InvalidParameter(
            "composite operation needs inputs".to_string(),
        ));
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Ok(f64::NAN);
    }
    Ok(match op {
        CompositeOp::Add => values.iter().sum(),
        CompositeOp::Sub => values[1..]
            .iter()
            .fold(values[0], |value, next| value - next),
        CompositeOp::Mul => values.iter().product(),
        CompositeOp::Div => values[1..]
            .iter()
            .try_fold(values[0], |value, next| {
                if *next == 0.0 {
                    None
                } else {
                    Some(value / next)
                }
            })
            .unwrap_or(f64::NAN),
        CompositeOp::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
        CompositeOp::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        CompositeOp::WeightedAverage => {
            let total = values.len() as f64;
            values.iter().sum::<f64>() / total
        }
    })
}

fn unary_finite(inputs: &[f64], name: &str) -> FactorResult<f64> {
    if inputs.len() != 1 {
        return Err(FactorError::InvalidParameter(format!(
            "{name} requires one input"
        )));
    }
    Ok(inputs[0])
}

fn finite_param(value: f64, name: &str) -> FactorResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FactorError::InvalidParameter(format!(
            "{name} must be finite"
        )))
    }
}

fn period_at(params: &[f64], index: usize, default: usize) -> FactorResult<usize> {
    let value = params.get(index).copied().unwrap_or(default as f64);
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(FactorError::InvalidParameter(
            "period must be a positive integer".to_string(),
        ));
    }
    Ok(value as usize)
}

fn period(params: &[f64], default: usize) -> FactorResult<usize> {
    period_at(params, 0, default)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::{CompositeDefinition, CompositeEngine, StatefulCompositeSpec};
    use crate::factors::FactorContext;
    use std::sync::Arc;

    fn assert_same(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (index, (left, right)) in actual.iter().zip(expected).enumerate() {
            assert!(
                (left.is_nan() && right.is_nan()) || (left - right).abs() < 1e-10,
                "mismatch at {index}: stateful={left}, batch={right}"
            );
        }
    }

    #[test]
    fn recursive_composite_stateful_stream_matches_batch() {
        let close: Vec<f64> = (0..160)
            .map(|index| 100.0 + index as f64 * 0.17 + (index as f64 * 0.13).sin())
            .collect();
        for (name, period) in [("ema", 12.0), ("rsi", 14.0)] {
            let definitions = vec![CompositeDefinition::new(
                name,
                CompositeExpr::call(name, vec![CompositeExpr::series("close")], vec![period]),
            )];
            let engine = CompositeEngine::new();
            let plan = engine.compile(&definitions, &[name]).unwrap();
            let context = FactorContext::new()
                .with_series("close", close.clone())
                .unwrap();
            let batch = engine
                .evaluate_compiled(&plan, &context.as_borrowed())
                .unwrap();

            let mut stream = plan.stateful_stream().unwrap();
            let mut actual = Vec::with_capacity(close.len());
            let mut output = [f64::NAN];
            for value in &close {
                stream.push_values_into(&[*value], &mut output).unwrap();
                actual.push(output[0]);
            }
            assert_same(&actual, &batch[name]);

            let checkpoint = stream.checkpoint();
            stream.push_values_into(&[close[0]], &mut output).unwrap();
            stream.restore(&checkpoint).unwrap();
            stream.push_values_into(&[close[0]], &mut output).unwrap();
            assert_eq!(stream.rows(), close.len() + 1);
        }
    }

    #[test]
    fn atr_and_macd_stateful_stream_match_batch() {
        let high: Vec<f64> = (0..180)
            .map(|index| 110.0 + index as f64 * 0.11 + (index as f64 * 0.07).sin())
            .collect();
        let low: Vec<f64> = high.iter().map(|value| value - 2.0).collect();
        let close: Vec<f64> = high
            .iter()
            .zip(&low)
            .map(|(high, low)| (high + low) / 2.0)
            .collect();

        let atr_definitions = vec![CompositeDefinition::new(
            "atr",
            CompositeExpr::call(
                "atr",
                vec![
                    CompositeExpr::series("high"),
                    CompositeExpr::series("low"),
                    CompositeExpr::series("close"),
                ],
                vec![14.0],
            ),
        )];
        let macd_definitions = vec![CompositeDefinition::new(
            "macd",
            CompositeExpr::call(
                "macd",
                vec![CompositeExpr::series("close")],
                vec![12.0, 26.0, 9.0],
            ),
        )];
        for (definitions, output) in [(atr_definitions, "atr"), (macd_definitions, "macd")] {
            let engine = CompositeEngine::new();
            let plan = engine.compile(&definitions, &[output]).unwrap();
            let context = FactorContext::new()
                .with_series("high", high.clone())
                .unwrap()
                .with_series("low", low.clone())
                .unwrap()
                .with_series("close", close.clone())
                .unwrap();
            let batch = engine
                .evaluate_compiled(&plan, &context.as_borrowed())
                .unwrap();
            let mut stream = plan.stateful_stream().unwrap();
            let mut actual = Vec::with_capacity(close.len());
            let mut row = vec![f64::NAN; plan.required_raw_inputs().len()];
            let mut values = BTreeMap::new();
            values.insert("close".to_string(), close.clone());
            values.insert("high".to_string(), high.clone());
            values.insert("low".to_string(), low.clone());
            let mut emitted = BTreeMap::new();
            stream.push_batch_into(&values, &mut emitted).unwrap();
            actual.extend_from_slice(&emitted[output]);
            assert_same(&actual, &batch[output]);
            row.fill(0.0);
            assert_eq!(row.len(), plan.required_raw_inputs().len());
        }
    }

    #[test]
    fn stateful_nested_graph_and_json_checkpoint_round_trip() {
        let definitions = vec![
            CompositeDefinition::new(
                "ema",
                CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![5.0]),
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::Op {
                    op: CompositeOp::Add,
                    inputs: vec![
                        CompositeExpr::reference("ema"),
                        CompositeExpr::Constant(1.0),
                    ],
                },
            ),
        ];
        let engine = CompositeEngine::new();
        let plan = engine.compile(&definitions, &["signal"]).unwrap();
        let mut stream = plan.stateful_stream().unwrap();
        let mut output = [f64::NAN];
        for value in [1.0, 2.0, 3.0, 4.0, 5.0] {
            stream.push_values_into(&[value], &mut output).unwrap();
        }
        let checkpoint = stream.checkpoint();
        let encoded = checkpoint.to_json().unwrap();
        let decoded = StatefulCompositeCheckpoint::from_json(&encoded).unwrap();
        let mut expected = [f64::NAN];
        stream.push_values_into(&[6.0], &mut expected).unwrap();
        stream.restore(&decoded).unwrap();
        let mut restored = [f64::NAN];
        stream.push_values_into(&[6.0], &mut restored).unwrap();
        assert_eq!(expected[0], restored[0]);
    }

    #[test]
    fn custom_composite_uses_registered_state_spec_instead_of_name_dispatch() {
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "custom_ema",
                Arc::new(|inputs, params| {
                    let period = params.first().copied().unwrap_or(3.0) as usize;
                    crate::math::moving_avg::ema(inputs[0], period)
                        .map(|values| values.into_raw_vec())
                        .map_err(|error| FactorError::Compute(error.to_string()))
                }),
            )
            .unwrap();
        engine
            .register_stateful_spec("custom_ema", StatefulCompositeSpec::Ema)
            .unwrap();

        let definitions = vec![CompositeDefinition::new(
            "result",
            CompositeExpr::call(
                "custom_ema",
                vec![CompositeExpr::series("close")],
                vec![3.0],
            ),
        )];
        let close = vec![10.0, 11.0, 12.0, 15.0, 14.0, 16.0];
        let plan = engine.compile(&definitions, &["result"]).unwrap();
        let context = FactorContext::new()
            .with_series("close", close.clone())
            .unwrap();
        let batch = engine
            .evaluate_compiled(&plan, &context.as_borrowed())
            .unwrap();
        let mut stream = plan.stateful_stream().unwrap();
        let mut actual = Vec::with_capacity(close.len());
        let mut output = [f64::NAN];
        for value in close {
            stream.push_values_into(&[value], &mut output).unwrap();
            actual.push(output[0]);
        }
        assert_same(&actual, &batch["result"]);
    }
}
