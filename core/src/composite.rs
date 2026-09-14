//! Declarative composite-indicator graphs.
//!
//! This layer complements the low-level indicator functions and the factor
//! registry.  A graph is made from named series, built-in indicators, simple
//! vector operators, and user-registered functions.  Definitions are
//! evaluated with dependency memoization and cycle detection, so a request
//! can reuse an intermediate series without recomputing it.

use crate::factors::{zscore, BorrowedFactorContext, FactorContext, FactorError, FactorResult};
use crate::indicators::{self, momentum, volatility};
use crate::math::{moving_avg, statistics};
use std::collections::BTreeMap;
use std::sync::Arc;

/// A user-defined vector function used by [`CompositeExpr::Call`].
pub type CompositeFn = Arc<dyn Fn(&[&[f64]], &[f64]) -> FactorResult<Vec<f64>> + Send + Sync>;

/// An expression in a composite-indicator graph.
#[derive(Debug, Clone)]
pub enum CompositeExpr {
    /// Read one raw or previously defined named series.
    Series(String),
    /// A scalar broadcast to the current row count.
    Constant(f64),
    /// Invoke a registered indicator/function.
    Call {
        /// Registered function name, case-insensitive for built-ins.
        name: String,
        /// Input expressions passed to the function.
        inputs: Vec<Self>,
        /// Numeric parameters such as lookback windows.
        params: Vec<f64>,
    },
    /// A named graph definition.
    Ref(String),
    /// Element-wise vector operation.
    Op {
        /// Operation to apply.
        op: CompositeOp,
        /// One or more operands.
        inputs: Vec<Self>,
    },
}

impl CompositeExpr {
    /// Shorthand for a named series.
    pub fn series(name: impl Into<String>) -> Self {
        Self::Series(name.into())
    }

    /// Shorthand for a named graph reference.
    pub fn reference(name: impl Into<String>) -> Self {
        Self::Ref(name.into())
    }

    /// Shorthand for a function call.
    pub fn call(name: impl Into<String>, inputs: Vec<Self>, params: Vec<f64>) -> Self {
        Self::Call {
            name: name.into(),
            inputs,
            params,
        }
    }

    /// Shorthand for a weighted composite blend. Empty weights mean equal
    /// weights; otherwise one weight is required for each input.
    pub fn weighted_average(inputs: Vec<Self>, weights: Vec<f64>) -> Self {
        Self::call("weighted_average", inputs, weights)
    }
}

/// Element-wise operations available in a composite graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositeOp {
    /// Sum all operands.
    Add,
    /// Subtract subsequent operands from the first.
    Sub,
    /// Multiply all operands.
    Mul,
    /// Divide the first operand by subsequent operands.
    Div,
    /// Element-wise minimum.
    Min,
    /// Element-wise maximum.
    Max,
    /// Weighted average with equal weights for the operands. Use
    /// [`CompositeExpr::weighted_average`] when explicit weights are needed.
    WeightedAverage,
}

/// One named output in a composite graph.
#[derive(Debug, Clone)]
pub struct CompositeDefinition {
    /// Stable output name.
    pub name: String,
    /// Expression producing the output.
    pub expression: CompositeExpr,
}

impl CompositeDefinition {
    /// Create a named definition.
    pub fn new(name: impl Into<String>, expression: CompositeExpr) -> Self {
        Self {
            name: name.into(),
            expression,
        }
    }
}

/// Dependency-aware composite-indicator evaluator.
#[derive(Clone)]
pub struct CompositeEngine {
    functions: BTreeMap<String, CompositeFn>,
    cache: BTreeMap<(u64, u64), BTreeMap<String, Vec<f64>>>,
    cache_capacity: usize,
}

/// A value produced while walking a composite expression.
///
/// Raw context series are borrowed for the duration of one expression. Only
/// operators, indicator calls, constants and named graph results allocate an
/// owned vector. This is the important distinction between the public
/// `evaluate` convenience API and `evaluate_borrowed`, which is used by
/// high-frequency chart/strategy paths.
enum CompositeValue<'a> {
    Borrowed(&'a [f64]),
    Owned(Vec<f64>),
}

impl<'a> CompositeValue<'a> {
    fn as_slice(&self) -> &[f64] {
        match self {
            Self::Borrowed(values) => values,
            Self::Owned(values) => values,
        }
    }

    fn into_owned(self) -> Vec<f64> {
        match self {
            Self::Borrowed(values) => values.to_vec(),
            Self::Owned(values) => values,
        }
    }
}

impl CompositeEngine {
    /// Create an engine with the standard built-in functions registered.
    pub fn new() -> Self {
        let mut engine = Self::default();
        engine.register_builtins();
        engine
    }

    /// Register or replace a custom vector function.
    pub fn register(&mut self, name: impl Into<String>, function: CompositeFn) -> FactorResult<()> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(FactorError::InvalidParameter(
                "composite function name must not be empty".to_string(),
            ));
        }
        self.functions.insert(name.to_ascii_lowercase(), function);
        self.cache.clear();
        Ok(())
    }

    /// Set the maximum number of cached graph snapshots retained by this engine.
    ///
    /// A zero value is treated as one entry so the cache remains deterministic
    /// without requiring a special case in the evaluation path.
    pub fn with_cache_capacity(mut self, capacity: usize) -> Self {
        self.cache_capacity = capacity.max(1);
        self.cache.clear();
        self
    }

    /// Change the cache limit after construction and invalidate old snapshots.
    pub fn set_cache_capacity(&mut self, capacity: usize) {
        self.cache_capacity = capacity.max(1);
        self.cache.clear();
    }

    /// Remove all cached graph snapshots.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Evaluate and cache a graph snapshot for an explicit data revision.
    ///
    /// The caller owns revision management. Reusing a revision for changed
    /// numeric data is invalid; advancing it gives deterministic invalidation
    /// across repeated chart/strategy requests.
    pub fn evaluate_cached(
        &mut self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
        data_revision: u64,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let key = (data_revision, graph_signature(definitions, outputs));
        if let Some(result) = self.cache.get(&key) {
            return Ok(result.clone());
        }
        let result = self.evaluate_borrowed(definitions, outputs, context)?;
        if self.cache.len() >= self.cache_capacity.max(1) {
            if let Some(oldest) = self.cache.keys().next().copied() {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(key, result.clone());
        Ok(result)
    }

    /// Evaluate selected outputs from an owned context.
    pub fn evaluate(
        &self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let borrowed = context.as_borrowed();
        self.evaluate_borrowed(definitions, outputs, &borrowed)
    }

    /// Evaluate selected outputs without copying raw input series.
    pub fn evaluate_borrowed(
        &self,
        definitions: &[CompositeDefinition],
        outputs: &[&str],
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let mut definitions_by_name = BTreeMap::new();
        for definition in definitions {
            if definition.name.trim().is_empty() {
                return Err(FactorError::InvalidParameter(
                    "composite definition name must not be empty".to_string(),
                ));
            }
            if definitions_by_name
                .insert(definition.name.as_str(), definition)
                .is_some()
            {
                return Err(FactorError::InvalidParameter(format!(
                    "duplicate composite definition: {}",
                    definition.name
                )));
            }
        }
        let definitions = definitions_by_name;
        let mut cache = BTreeMap::new();
        let mut visiting = Vec::new();
        for &output in outputs {
            self.eval_named(output, &definitions, context, &mut cache, &mut visiting)?;
        }
        Ok(outputs
            .iter()
            .filter_map(|name| {
                cache
                    .get(*name)
                    .map(|values| ((*name).to_string(), values.clone()))
            })
            .collect())
    }

    /// Register the standard functions used by daily indicator composition.
    fn register_builtins(&mut self) {
        let _ = self.register(
            "sma",
            Arc::new(|inputs, params| {
                unary_input(inputs, "sma")
                    .and_then(|input| Ok(rolling_sma(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "ema",
            Arc::new(|inputs, params| {
                unary_input(inputs, "ema")
                    .and_then(|input| Ok(rolling_ema(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "wma",
            Arc::new(|inputs, params| {
                unary_input(inputs, "wma")
                    .and_then(|input| Ok(rolling_wma(input, period(params, 14)?)))
            }),
        );
        let _ = self.register(
            "rsi",
            Arc::new(|inputs, params| {
                unary_input(inputs, "rsi").and_then(|input| {
                    indicators::momentum::rsi(input, period(params, 14)?)
                        .map(|value| value.into_raw_vec())
                        .map_err(ta_error)
                })
            }),
        );
        let _ = self.register(
            "atr",
            Arc::new(|inputs, params| {
                if inputs.len() != 3 {
                    return Err(FactorError::InvalidParameter(
                        "atr requires high, low and close".to_string(),
                    ));
                }
                volatility::atr(inputs[0], inputs[1], inputs[2], period(params, 14)?)
                    .map(|value| value.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "return",
            Arc::new(|inputs, params| {
                unary_input(inputs, "return")
                    .and_then(|input| crate::factors::time_series_return(input, period(params, 1)?))
            }),
        );
        let _ = self.register(
            "zscore",
            Arc::new(|inputs, _| unary_input(inputs, "zscore").map(zscore)),
        );
        let _ = self.register(
            "vwma",
            Arc::new(|inputs, params| {
                if inputs.len() != 2 {
                    return Err(FactorError::InvalidParameter(
                        "vwma requires input and volume".to_string(),
                    ));
                }
                moving_avg::vwma(inputs[0], inputs[1], period(params, 14)?)
                    .map(|value| value.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "macd",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "macd")?;
                let fast = period_at(params, 0, 12)?;
                let slow = period_at(params, 1, 26)?;
                let signal = period_at(params, 2, 9)?;
                momentum::macd(input, fast, slow, signal)
                    .map(|value| value.hist.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_mid",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_mid")?;
                indicators::overlap::bbands(input, period(params, 20)?, 2.0, 2.0)
                    .map(|value| value.middle.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_upper",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_upper")?;
                let deviation = params.get(1).copied().unwrap_or(2.0);
                indicators::overlap::bbands(input, period(params, 20)?, deviation, deviation)
                    .map(|value| value.upper.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "boll_lower",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "boll_lower")?;
                let deviation = params.get(1).copied().unwrap_or(2.0);
                indicators::overlap::bbands(input, period(params, 20)?, deviation, deviation)
                    .map(|value| value.lower.into_raw_vec())
                    .map_err(ta_error)
            }),
        );
        let _ = self.register(
            "threshold",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "threshold")?;
                let level = params.first().copied().unwrap_or(0.0);
                if !level.is_finite() {
                    return Err(FactorError::InvalidParameter(
                        "threshold must be finite".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        if value.is_finite() {
                            (*value >= level) as u8 as f64
                        } else {
                            f64::NAN
                        }
                    })
                    .collect())
            }),
        );
        let _ = self.register(
            "between",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "between")?;
                let lower = params.first().copied().unwrap_or(0.0);
                let upper = params.get(1).copied().unwrap_or(1.0);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "between bounds must be finite and ordered".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        if value.is_finite() {
                            (*value >= lower && *value <= upper) as u8 as f64
                        } else {
                            f64::NAN
                        }
                    })
                    .collect())
            }),
        );
        let _ = self.register(
            "clip",
            Arc::new(|inputs, params| {
                let input = unary_input(inputs, "clip")?;
                let lower = params.first().copied().unwrap_or(-f64::MAX);
                let upper = params.get(1).copied().unwrap_or(f64::MAX);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(FactorError::InvalidParameter(
                        "clip bounds must be finite and ordered".to_string(),
                    ));
                }
                Ok(input
                    .iter()
                    .map(|value| {
                        value
                            .is_finite()
                            .then(|| value.clamp(lower, upper))
                            .unwrap_or(f64::NAN)
                    })
                    .collect())
            }),
        );
        for (name, operation) in [("abs", 0_u8), ("neg", 1_u8), ("sign", 2_u8)] {
            let _ = self.register(
                name,
                Arc::new(move |inputs, _| {
                    let input = unary_input(inputs, name)?;
                    Ok(input
                        .iter()
                        .map(|value| {
                            if !value.is_finite() {
                                f64::NAN
                            } else {
                                match operation {
                                    0 => value.abs(),
                                    1 => -*value,
                                    _ => value.signum(),
                                }
                            }
                        })
                        .collect())
                }),
            );
        }
        for (name, operation) in [
            ("rolling_std", 0_u8),
            ("rolling_min", 1_u8),
            ("rolling_max", 2_u8),
        ] {
            let _ = self.register(
                name,
                Arc::new(move |inputs, params| {
                    let input = unary_input(inputs, name)?;
                    let window = period(params, 20)?;
                    match operation {
                        0 => statistics::rolling_std_dev(input, window),
                        1 => statistics::rolling_min(input, window),
                        _ => statistics::rolling_max(input, window),
                    }
                    .map(|values| values.into_raw_vec())
                    .map_err(ta_error)
                }),
            );
        }
        let _ = self.register(
            "volatility",
            Arc::new(|inputs, params| {
                unary_input(inputs, "volatility").and_then(|input| {
                    crate::factors::rolling_volatility(input, period(params, 20)?)
                })
            }),
        );
        let _ = self.register("cross_up", Arc::new(|inputs, _| cross(inputs, true)));
        let _ = self.register("cross_down", Arc::new(|inputs, _| cross(inputs, false)));
        let _ = self.register(
            "weighted_average",
            Arc::new(|inputs, params| weighted_average(inputs, params)),
        );
    }

    fn eval_named(
        &self,
        name: &str,
        definitions: &BTreeMap<&str, &CompositeDefinition>,
        context: &BorrowedFactorContext<'_>,
        cache: &mut BTreeMap<String, Vec<f64>>,
        visiting: &mut Vec<String>,
    ) -> FactorResult<()> {
        if cache.contains_key(name) {
            return Ok(());
        }
        if let Some(position) = visiting.iter().position(|current| current == name) {
            let mut cycle = visiting[position..].to_vec();
            cycle.push(name.to_string());
            return Err(FactorError::DependencyCycle(cycle));
        }
        let definition = definitions
            .get(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
        visiting.push(name.to_string());
        let values = self
            .eval_expr(
                &definition.expression,
                definitions,
                context,
                cache,
                visiting,
            )?
            .into_owned();
        if values.len() != context.len() {
            return Err(FactorError::LengthMismatch {
                name: name.to_string(),
                expected: context.len(),
                actual: values.len(),
            });
        }
        cache.insert(name.to_string(), values);
        visiting.pop();
        Ok(())
    }

    fn eval_expr<'a>(
        &self,
        expression: &CompositeExpr,
        definitions: &BTreeMap<&str, &CompositeDefinition>,
        context: &BorrowedFactorContext<'a>,
        cache: &mut BTreeMap<String, Vec<f64>>,
        visiting: &mut Vec<String>,
    ) -> FactorResult<CompositeValue<'a>> {
        match expression {
            CompositeExpr::Series(name) => context
                .get(name)
                .map(CompositeValue::Borrowed)
                .ok_or_else(|| FactorError::MissingInput(name.clone())),
            CompositeExpr::Constant(value) => {
                Ok(CompositeValue::Owned(vec![*value; context.len()]))
            }
            CompositeExpr::Ref(name) => {
                self.eval_named(name, definitions, context, cache, visiting)?;
                cache
                    .get(name)
                    .cloned()
                    .map(CompositeValue::Owned)
                    .ok_or_else(|| FactorError::UnknownFactor(name.clone()))
            }
            CompositeExpr::Call {
                name,
                inputs,
                params,
            } => {
                let values: Vec<CompositeValue<'_>> = inputs
                    .iter()
                    .map(|input| self.eval_expr(input, definitions, context, cache, visiting))
                    .collect::<FactorResult<_>>()?;
                let views: Vec<&[f64]> = values.iter().map(CompositeValue::as_slice).collect();
                let function = self
                    .functions
                    .get(&name.to_ascii_lowercase())
                    .ok_or_else(|| FactorError::UnknownFactor(name.clone()))?;
                let result = function(&views, params)?;
                if result.len() != context.len() {
                    return Err(FactorError::LengthMismatch {
                        name: name.clone(),
                        expected: context.len(),
                        actual: result.len(),
                    });
                }
                Ok(CompositeValue::Owned(result))
            }
            CompositeExpr::Op { op, inputs } => {
                let values: Vec<CompositeValue<'_>> = inputs
                    .iter()
                    .map(|input| self.eval_expr(input, definitions, context, cache, visiting))
                    .collect::<FactorResult<_>>()?;
                apply_op(*op, &values, &[])
            }
        }
    }
}

impl Default for CompositeEngine {
    fn default() -> Self {
        Self {
            functions: BTreeMap::new(),
            cache: BTreeMap::new(),
            cache_capacity: 64,
        }
    }
}

fn graph_signature(definitions: &[CompositeDefinition], outputs: &[&str]) -> u64 {
    let signature = format!("{definitions:?}|{outputs:?}");
    let mut hash = 0xcbf29ce484222325u64;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn period(params: &[f64], default: usize) -> FactorResult<usize> {
    period_at(params, 0, default)
}

fn ta_error(error: crate::error::TaError) -> FactorError {
    FactorError::Compute(error.to_string())
}

fn rolling_sma(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    for index in period.saturating_sub(1)..values.len() {
        let window = &values[index + 1 - period..=index];
        if window.iter().all(|value| value.is_finite()) {
            output[index] = window.iter().sum::<f64>() / period as f64;
        }
    }
    output
}

fn rolling_ema(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut previous = f64::NAN;
    for index in 0..values.len() {
        if !values[index].is_finite() {
            previous = f64::NAN;
            continue;
        }
        if previous.is_finite() {
            previous += alpha * (values[index] - previous);
            output[index] = previous;
        } else if index + 1 >= period
            && values[index + 1 - period..=index]
                .iter()
                .all(|value| value.is_finite())
        {
            previous = values[index + 1 - period..=index].iter().sum::<f64>() / period as f64;
            output[index] = previous;
        }
    }
    output
}

fn rolling_wma(values: &[f64], period: usize) -> Vec<f64> {
    let mut output = vec![f64::NAN; values.len()];
    if period == 0 {
        return output;
    }
    let denominator = (period * (period + 1) / 2) as f64;
    for index in period.saturating_sub(1)..values.len() {
        let window = &values[index + 1 - period..=index];
        if window.iter().all(|value| value.is_finite()) {
            output[index] = window
                .iter()
                .enumerate()
                .map(|(offset, value)| *value * (offset + 1) as f64)
                .sum::<f64>()
                / denominator;
        }
    }
    output
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

fn unary_input<'a>(inputs: &'a [&[f64]], name: &str) -> FactorResult<&'a [f64]> {
    if inputs.len() != 1 {
        return Err(FactorError::InvalidParameter(format!(
            "{name} requires one input"
        )));
    }
    Ok(inputs[0])
}

fn cross(inputs: &[&[f64]], upward: bool) -> FactorResult<Vec<f64>> {
    if inputs.len() != 2 || inputs[0].len() != inputs[1].len() {
        return Err(FactorError::InvalidParameter(
            "cross requires two aligned inputs".to_string(),
        ));
    }
    let mut output = vec![0.0; inputs[0].len()];
    for index in 1..output.len() {
        let (previous_a, previous_b) = (inputs[0][index - 1], inputs[1][index - 1]);
        let (current_a, current_b) = (inputs[0][index], inputs[1][index]);
        if [previous_a, previous_b, current_a, current_b]
            .iter()
            .all(|value| value.is_finite())
        {
            output[index] = if upward {
                (previous_a <= previous_b && current_a > current_b) as u8 as f64
            } else {
                (previous_a >= previous_b && current_a < current_b) as u8 as f64
            };
        }
    }
    Ok(output)
}

fn weighted_average(inputs: &[&[f64]], weights: &[f64]) -> FactorResult<Vec<f64>> {
    if inputs.is_empty() {
        return Err(FactorError::InvalidParameter(
            "weighted_average needs inputs".to_string(),
        ));
    }
    let len = inputs[0].len();
    if inputs.iter().any(|input| input.len() != len) {
        return Err(FactorError::InvalidParameter(
            "weighted_average inputs must be aligned".to_string(),
        ));
    }
    if !weights.is_empty() && weights.len() != inputs.len() {
        return Err(FactorError::InvalidParameter(
            "weights must match input count".to_string(),
        ));
    }
    let weights = if weights.is_empty() {
        vec![1.0; inputs.len()]
    } else {
        weights.to_vec()
    };
    if weights.iter().any(|weight| !weight.is_finite()) {
        return Err(FactorError::InvalidParameter(
            "weights must be finite".to_string(),
        ));
    }
    let total: f64 = weights.iter().map(|value| value.abs()).sum();
    let mut output = vec![f64::NAN; len];
    if total == 0.0 {
        return Ok(output);
    }
    for index in 0..len {
        if inputs.iter().any(|input| !input[index].is_finite()) {
            continue;
        }
        output[index] = inputs
            .iter()
            .zip(weights.iter())
            .map(|(input, weight)| input[index] * weight)
            .sum::<f64>()
            / total;
    }
    Ok(output)
}

fn apply_op(
    op: CompositeOp,
    values: &[CompositeValue<'_>],
    weights: &[f64],
) -> FactorResult<CompositeValue<'static>> {
    if values.is_empty() {
        return Err(FactorError::InvalidParameter(
            "composite operation needs inputs".to_string(),
        ));
    }
    let len = values[0].as_slice().len();
    if values.iter().any(|value| value.as_slice().len() != len) {
        return Err(FactorError::InvalidParameter(
            "composite inputs must be aligned".to_string(),
        ));
    }
    let mut output = vec![f64::NAN; len];
    for index in 0..len {
        let row: Vec<f64> = values.iter().map(|value| value.as_slice()[index]).collect();
        if row.iter().any(|value| !value.is_finite()) {
            continue;
        }
        output[index] = match op {
            CompositeOp::Add => row.iter().sum(),
            CompositeOp::Sub => row[1..].iter().fold(row[0], |value, next| value - next),
            CompositeOp::Mul => row.iter().product(),
            CompositeOp::Div => row[1..]
                .iter()
                .try_fold(row[0], |value, next| {
                    if *next == 0.0 {
                        None
                    } else {
                        Some(value / next)
                    }
                })
                .unwrap_or(f64::NAN),
            CompositeOp::Min => row.iter().copied().fold(f64::INFINITY, f64::min),
            CompositeOp::Max => row.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            CompositeOp::WeightedAverage => {
                let weights = if weights.is_empty() {
                    vec![1.0; row.len()]
                } else {
                    weights.to_vec()
                };
                if weights.len() != row.len() {
                    return Err(FactorError::InvalidParameter(
                        "weights must match input count".to_string(),
                    ));
                }
                let total: f64 = weights.iter().map(|value| value.abs()).sum();
                if total == 0.0 {
                    f64::NAN
                } else {
                    row.iter()
                        .zip(weights.iter())
                        .map(|(value, weight)| value * weight)
                        .sum::<f64>()
                        / total
                }
            }
        };
    }
    Ok(CompositeValue::Owned(output))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    #[test]
    fn evaluates_shared_named_dependencies_once_and_composes_them() {
        let close: Vec<f64> = (1..=40).map(|value| value as f64).collect();
        let context = FactorContext::new()
            .with_series("close", close)
            .expect("valid context");
        let definitions = vec![
            CompositeDefinition::new(
                "fast",
                CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![5.0]),
            ),
            CompositeDefinition::new(
                "slow",
                CompositeExpr::call("ema", vec![CompositeExpr::series("close")], vec![10.0]),
            ),
            CompositeDefinition::new(
                "spread",
                CompositeExpr::Op {
                    op: CompositeOp::Sub,
                    inputs: vec![
                        CompositeExpr::reference("fast"),
                        CompositeExpr::reference("slow"),
                    ],
                },
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::call("sma", vec![CompositeExpr::reference("spread")], vec![3.0]),
            ),
        ];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["signal"], &context)
            .expect("graph evaluates");
        assert_eq!(result["signal"].len(), 40);
        assert!(result["signal"][39].is_finite());
    }

    #[test]
    fn detects_definition_cycles() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .expect("context");
        let definitions = vec![
            CompositeDefinition::new("a", CompositeExpr::reference("b")),
            CompositeDefinition::new("b", CompositeExpr::reference("a")),
        ];
        let error = CompositeEngine::new()
            .evaluate(&definitions, &["a"], &context)
            .expect_err("cycle must fail");
        assert!(matches!(error, FactorError::DependencyCycle(_)));
    }

    #[test]
    fn cached_evaluation_reuses_revision_and_invalidates_on_revision_change() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "identity",
                Arc::new(move |inputs, _| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok(inputs[0].to_vec())
                }),
            )
            .expect("register custom function");
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .expect("context");
        let borrowed = context.as_borrowed();
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call("identity", vec![CompositeExpr::series("close")], vec![]),
        )];
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 7)
            .expect("first evaluation");
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 7)
            .expect("cached evaluation");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        engine
            .evaluate_cached(&definitions, &["value"], &borrowed, 8)
            .expect("new revision evaluation");
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn borrowed_evaluation_passes_raw_series_without_copying() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .expect("context");
        let borrowed = context.as_borrowed();
        let original_pointer = borrowed.get("close").expect("close series").as_ptr() as usize;
        let same_storage = Arc::new(AtomicBool::new(false));
        let observed = same_storage.clone();
        let mut engine = CompositeEngine::new();
        engine
            .register(
                "identity",
                Arc::new(move |inputs, _| {
                    observed.store(
                        inputs[0].as_ptr() as usize == original_pointer,
                        Ordering::SeqCst,
                    );
                    Ok(inputs[0].to_vec())
                }),
            )
            .expect("register identity");
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call("identity", vec![CompositeExpr::series("close")], vec![]),
        )];
        engine
            .evaluate_borrowed(&definitions, &["value"], &borrowed)
            .expect("borrowed graph evaluates");
        assert!(same_storage.load(Ordering::SeqCst));
    }

    #[test]
    fn weighted_average_uses_declared_parameters() {
        let context = FactorContext::new()
            .with_series("a", vec![10.0, 20.0])
            .expect("first series")
            .with_series("b", vec![20.0, 40.0])
            .expect("second series");
        let definitions = vec![CompositeDefinition::new(
            "value",
            CompositeExpr::call(
                "weighted_average",
                vec![CompositeExpr::series("a"), CompositeExpr::series("b")],
                vec![1.0, 3.0],
            ),
        )];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["value"], &context)
            .expect("weighted graph evaluates");
        assert_eq!(result["value"], vec![17.5, 35.0]);
    }

    #[test]
    fn threshold_and_rolling_builtins_support_signal_composition() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0, 2.0, 4.0])
            .expect("context");
        let definitions = vec![
            CompositeDefinition::new(
                "above",
                CompositeExpr::call("threshold", vec![CompositeExpr::series("close")], vec![3.0]),
            ),
            CompositeDefinition::new(
                "range",
                CompositeExpr::call(
                    "between",
                    vec![CompositeExpr::series("close")],
                    vec![2.0, 3.0],
                ),
            ),
            CompositeDefinition::new(
                "signal",
                CompositeExpr::call(
                    "cross_up",
                    vec![CompositeExpr::series("close"), CompositeExpr::Constant(2.5)],
                    vec![],
                ),
            ),
            CompositeDefinition::new(
                "vol",
                CompositeExpr::call(
                    "rolling_std",
                    vec![CompositeExpr::series("close")],
                    vec![3.0],
                ),
            ),
        ];
        let result = CompositeEngine::new()
            .evaluate(&definitions, &["above", "range", "signal", "vol"], &context)
            .expect("threshold graph evaluates");
        assert_eq!(result["above"], vec![0.0, 0.0, 1.0, 0.0, 1.0]);
        assert_eq!(result["range"], vec![0.0, 1.0, 1.0, 1.0, 0.0]);
        assert_eq!(result["signal"][2], 1.0);
        assert!(result["vol"][2].is_finite());
    }

    #[test]
    fn rejects_duplicate_or_empty_definition_names() {
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .expect("context");
        let duplicate = vec![
            CompositeDefinition::new("value", CompositeExpr::series("close")),
            CompositeDefinition::new("value", CompositeExpr::series("close")),
        ];
        assert!(matches!(
            CompositeEngine::new().evaluate(&duplicate, &["value"], &context),
            Err(FactorError::InvalidParameter(message)) if message.contains("duplicate")
        ));
        let empty = vec![CompositeDefinition::new(
            " ",
            CompositeExpr::series("close"),
        )];
        assert!(matches!(
            CompositeEngine::new().evaluate(&empty, &[" "][..], &context),
            Err(FactorError::InvalidParameter(message)) if message.contains("must not be empty")
        ));
    }
}
