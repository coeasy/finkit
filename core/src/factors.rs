//! Production factor computation primitives for Finkit.
//!
//! The factor layer is intentionally data-source agnostic: callers provide
//! named numeric series, while the engine resolves factor dependencies,
//! caches intermediate values, detects cycles, and supports time-series and
//! cross-sectional post-processing. Raw inputs can be owned (`FactorContext`)
//! or borrowed (`BorrowedFactorContext`) without changing custom factor
//! callbacks.

use crate::runtime::MarketFrame;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

/// Result type used by the factor engine.
pub type FactorResult<T> = std::result::Result<T, FactorError>;

/// Errors raised while registering or evaluating factors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactorError {
    /// A factor with the same name was already registered.
    DuplicateFactor(String),
    /// The requested factor is not registered.
    UnknownFactor(String),
    /// A required raw input series is missing.
    MissingInput(String),
    /// Input or output series lengths are inconsistent.
    LengthMismatch {
        /// Series or factor name.
        name: String,
        /// Expected number of rows.
        expected: usize,
        /// Actual number of rows.
        actual: usize,
    },
    /// A dependency cycle was detected.
    DependencyCycle(Vec<String>),
    /// An invalid parameter was supplied.
    InvalidParameter(String),
    /// User-provided factor logic failed.
    Compute(String),
}

impl fmt::Display for FactorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateFactor(name) => write!(f, "factor already registered: {name}"),
            Self::UnknownFactor(name) => write!(f, "unknown factor: {name}"),
            Self::MissingInput(name) => write!(f, "missing factor input: {name}"),
            Self::LengthMismatch {
                name,
                expected,
                actual,
            } => write!(
                f,
                "length mismatch for {name}: expected {expected}, got {actual}"
            ),
            Self::DependencyCycle(path) => {
                write!(f, "factor dependency cycle: {}", path.join(" -> "))
            }
            Self::InvalidParameter(message) => write!(f, "invalid factor parameter: {message}"),
            Self::Compute(message) => write!(f, "factor computation failed: {message}"),
        }
    }
}

impl std::error::Error for FactorError {}

/// Whether a factor is evaluated along time or across instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorKind {
    /// Per-instrument time-series factor.
    TimeSeries,
    /// Per-timestamp cross-sectional factor.
    CrossSectional,
}

/// Direction used when turning raw factor values into a score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactorDirection {
    /// Larger values are preferred.
    HigherBetter,
    /// Smaller values are preferred.
    LowerBetter,
    /// The factor has no ranking direction and is used as-is.
    Neutral,
}

/// Internal abstraction shared by owned and borrowed raw factor contexts.
trait RawFactorContext {
    fn raw_get(&self, name: &str) -> Option<&[f64]>;
    fn row_count(&self) -> usize;
}

/// Owned named raw-series input for factor evaluation.
#[derive(Debug, Clone, Default)]
pub struct FactorContext {
    series: BTreeMap<String, Vec<f64>>,
    len: Option<usize>,
}

impl FactorContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a named input series while enforcing row alignment.
    pub fn insert(&mut self, name: impl Into<String>, values: Vec<f64>) -> FactorResult<()> {
        let name = name.into();
        validate_series_name(&name)?;
        validate_series_len(self.len, &name, values.len())?;
        if self.len.is_none() {
            self.len = Some(values.len());
        }
        self.series.insert(name, values);
        Ok(())
    }

    /// Builder-style insertion helper.
    pub fn with_series(mut self, name: impl Into<String>, values: Vec<f64>) -> FactorResult<Self> {
        self.insert(name, values)?;
        Ok(self)
    }

    /// Return a raw input series by name.
    pub fn get(&self, name: &str) -> Option<&[f64]> {
        self.series.get(name).map(Vec::as_slice)
    }

    /// Create a zero-copy borrowed view of all owned series.
    ///
    /// Only the string-key map is rebuilt; numeric buffers remain borrowed
    /// directly from this context.
    pub fn as_borrowed(&self) -> BorrowedFactorContext<'_> {
        BorrowedFactorContext {
            series: self
                .series
                .iter()
                .map(|(name, values)| (name.clone(), values.as_slice()))
                .collect(),
            len: self.len,
        }
    }

    /// Number of aligned rows in this context.
    pub fn len(&self) -> usize {
        self.len.unwrap_or(0)
    }

    /// Whether the context contains no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RawFactorContext for FactorContext {
    fn raw_get(&self, name: &str) -> Option<&[f64]> {
        self.get(name)
    }

    fn row_count(&self) -> usize {
        self.len()
    }
}

/// Zero-copy named raw-series input for factor evaluation.
///
/// This context is the preferred boundary for `MarketFrame`, NumPy/FFI
/// adapters, and other owners that can guarantee input lifetimes for the
/// duration of one factor evaluation.
#[derive(Debug, Clone, Default)]
pub struct BorrowedFactorContext<'a> {
    series: BTreeMap<String, &'a [f64]>,
    len: Option<usize>,
}

impl<'a> BorrowedFactorContext<'a> {
    /// Create an empty borrowed context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a borrowed named series while enforcing row alignment.
    pub fn insert(&mut self, name: impl Into<String>, values: &'a [f64]) -> FactorResult<()> {
        let name = name.into();
        validate_series_name(&name)?;
        validate_series_len(self.len, &name, values.len())?;
        if self.len.is_none() {
            self.len = Some(values.len());
        }
        self.series.insert(name, values);
        Ok(())
    }

    /// Builder-style borrowed insertion helper.
    pub fn with_series(mut self, name: impl Into<String>, values: &'a [f64]) -> FactorResult<Self> {
        self.insert(name, values)?;
        Ok(self)
    }

    /// Build a borrowed factor context directly from an aligned market frame.
    ///
    /// The OHLCV/amount slices are not copied. Timestamps are intentionally not
    /// inserted because their integer representation is not a numeric factor
    /// series of `f64` values.
    pub fn from_market_frame(frame: MarketFrame<'a>) -> FactorResult<Self> {
        frame
            .validate()
            .map_err(|error| FactorError::InvalidParameter(error.to_string()))?;
        let mut context = Self::new()
            .with_series("open", frame.open)?
            .with_series("high", frame.high)?
            .with_series("low", frame.low)?
            .with_series("close", frame.close)?
            .with_series("volume", frame.volume)?;
        if let Some(amount) = frame.amount {
            context.insert("amount", amount)?;
        }
        Ok(context)
    }

    /// Return a borrowed raw input series by name.
    pub fn get(&self, name: &str) -> Option<&'a [f64]> {
        self.series.get(name).copied()
    }

    /// Number of aligned rows in this context.
    pub fn len(&self) -> usize {
        self.len.unwrap_or(0)
    }

    /// Whether the context contains no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl RawFactorContext for BorrowedFactorContext<'_> {
    fn raw_get(&self, name: &str) -> Option<&[f64]> {
        self.series.get(name).copied()
    }

    fn row_count(&self) -> usize {
        self.len()
    }
}

fn validate_series_name(name: &str) -> FactorResult<()> {
    if name.trim().is_empty() {
        Err(FactorError::InvalidParameter(
            "factor input name must not be empty".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn validate_series_len(expected: Option<usize>, name: &str, actual: usize) -> FactorResult<()> {
    if let Some(expected) = expected {
        if actual != expected {
            return Err(FactorError::LengthMismatch {
                name: name.to_string(),
                expected,
                actual,
            });
        }
    }
    Ok(())
}

/// Read-only view available to a factor computation closure.
pub struct FactorInputs<'a> {
    raw: &'a dyn RawFactorContext,
    computed: &'a BTreeMap<String, Vec<f64>>,
}

impl<'a> FactorInputs<'a> {
    /// Resolve either a previously computed factor or a raw input series.
    pub fn get(&self, name: &str) -> FactorResult<&[f64]> {
        if let Some(values) = self.computed.get(name) {
            return Ok(values.as_slice());
        }
        self.raw
            .raw_get(name)
            .ok_or_else(|| FactorError::MissingInput(name.to_string()))
    }
}

/// Function signature for custom factor implementations.
pub type FactorFn = Arc<dyn Fn(&FactorInputs<'_>) -> FactorResult<Vec<f64>> + Send + Sync>;

/// Declarative factor definition stored in [`FactorRegistry`].
#[derive(Clone)]
pub struct FactorDefinition {
    /// Stable factor identifier.
    pub name: String,
    /// Named raw-series or factor dependencies.
    pub dependencies: Vec<String>,
    /// Time-series or cross-sectional execution intent.
    pub kind: FactorKind,
    /// Preferred ranking direction.
    pub direction: FactorDirection,
    /// Computation callback.
    pub compute: FactorFn,
}

impl FactorDefinition {
    /// Create a factor definition.
    pub fn new(
        name: impl Into<String>,
        dependencies: impl IntoIterator<Item = impl Into<String>>,
        kind: FactorKind,
        direction: FactorDirection,
        compute: FactorFn,
    ) -> Self {
        Self {
            name: name.into(),
            dependencies: dependencies.into_iter().map(Into::into).collect(),
            kind,
            direction,
            compute,
        }
    }
}

/// Registry for built-in and user-defined factors.
#[derive(Clone, Default)]
pub struct FactorRegistry {
    factors: BTreeMap<String, FactorDefinition>,
}

impl FactorRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a factor. Duplicate names are rejected.
    pub fn register(&mut self, factor: FactorDefinition) -> FactorResult<()> {
        if factor.name.trim().is_empty() {
            return Err(FactorError::InvalidParameter(
                "factor name must not be empty".to_string(),
            ));
        }
        if factor
            .dependencies
            .iter()
            .any(|dependency| dependency.trim().is_empty())
        {
            return Err(FactorError::InvalidParameter(format!(
                "factor dependency must not be empty: {}",
                factor.name
            )));
        }
        if self.factors.contains_key(&factor.name) {
            return Err(FactorError::DuplicateFactor(factor.name));
        }
        self.factors.insert(factor.name.clone(), factor);
        Ok(())
    }

    /// Get a factor by name.
    pub fn get(&self, name: &str) -> Option<&FactorDefinition> {
        self.factors.get(name)
    }

    /// List registered factor names in deterministic order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.factors.keys().map(String::as_str)
    }

    /// Number of registered factors.
    pub fn len(&self) -> usize {
        self.factors.len()
    }

    /// Whether no factors are registered.
    pub fn is_empty(&self) -> bool {
        self.factors.is_empty()
    }
}

/// Dependency-aware factor evaluator with per-request memoization.
#[derive(Clone, Default)]
pub struct FactorEngine {
    registry: FactorRegistry,
}

impl FactorEngine {
    /// Build an engine from a registry.
    pub fn new(registry: FactorRegistry) -> Self {
        Self { registry }
    }

    /// Access the underlying registry.
    pub fn registry(&self) -> &FactorRegistry {
        &self.registry
    }

    /// Evaluate a single factor and all of its dependencies from owned input.
    pub fn evaluate(&self, name: &str, context: &FactorContext) -> FactorResult<Vec<f64>> {
        self.evaluate_one_raw(name, context)
    }

    /// Evaluate a single factor from zero-copy borrowed input.
    pub fn evaluate_borrowed(
        &self,
        name: &str,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<Vec<f64>> {
        self.evaluate_one_raw(name, context)
    }

    /// Evaluate multiple factors while sharing a dependency cache.
    pub fn evaluate_many(
        &self,
        names: &[&str],
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.evaluate_many_raw(names, context)
    }

    /// Evaluate multiple factors over borrowed raw input while sharing the
    /// same dependency cache.
    pub fn evaluate_many_borrowed(
        &self,
        names: &[&str],
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        self.evaluate_many_raw(names, context)
    }

    /// Build a direction-aware weighted composite score from owned input.
    pub fn composite(
        &self,
        weighted_factors: &[(&str, f64)],
        context: &FactorContext,
    ) -> FactorResult<Vec<f64>> {
        self.composite_raw(weighted_factors, context)
    }

    /// Build a direction-aware weighted composite score from borrowed input.
    pub fn composite_borrowed(
        &self,
        weighted_factors: &[(&str, f64)],
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<Vec<f64>> {
        self.composite_raw(weighted_factors, context)
    }

    fn evaluate_one_raw(
        &self,
        name: &str,
        context: &dyn RawFactorContext,
    ) -> FactorResult<Vec<f64>> {
        let mut cache = BTreeMap::new();
        let mut visiting = Vec::new();
        self.evaluate_inner(name, context, &mut cache, &mut visiting)?;
        cache
            .remove(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))
    }

    fn evaluate_many_raw(
        &self,
        names: &[&str],
        context: &dyn RawFactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let mut cache = BTreeMap::new();
        for &name in names {
            let mut visiting = Vec::new();
            self.evaluate_inner(name, context, &mut cache, &mut visiting)?;
        }
        Ok(cache)
    }

    fn composite_raw(
        &self,
        weighted_factors: &[(&str, f64)],
        context: &dyn RawFactorContext,
    ) -> FactorResult<Vec<f64>> {
        if weighted_factors.is_empty() {
            return Ok(vec![f64::NAN; context.row_count()]);
        }
        let names: Vec<&str> = weighted_factors.iter().map(|(name, _)| *name).collect();
        let values = self.evaluate_many_raw(&names, context)?;
        let mut output = vec![f64::NAN; context.row_count()];

        for row in 0..context.row_count() {
            let mut weighted_sum = 0.0;
            let mut effective_weight = 0.0;
            for &(name, weight) in weighted_factors {
                let factor = self
                    .registry
                    .get(name)
                    .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
                let value = values[name][row];
                if !value.is_finite() || !weight.is_finite() || weight == 0.0 {
                    continue;
                }
                let directed = match factor.direction {
                    FactorDirection::HigherBetter | FactorDirection::Neutral => value,
                    FactorDirection::LowerBetter => -value,
                };
                weighted_sum += directed * weight;
                effective_weight += weight.abs();
            }
            if effective_weight > 0.0 {
                output[row] = weighted_sum / effective_weight;
            }
        }
        Ok(output)
    }

    fn evaluate_inner(
        &self,
        name: &str,
        context: &dyn RawFactorContext,
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

        let factor = self
            .registry
            .get(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
        visiting.push(name.to_string());

        for dependency in &factor.dependencies {
            if self.registry.get(dependency).is_some() {
                self.evaluate_inner(dependency, context, cache, visiting)?;
            } else if context.raw_get(dependency).is_none() {
                return Err(FactorError::MissingInput(dependency.clone()));
            }
        }

        self.compute_factor(factor, context, cache)?;
        visiting.pop();
        Ok(())
    }

    fn compute_factor(
        &self,
        factor: &FactorDefinition,
        context: &dyn RawFactorContext,
        cache: &mut BTreeMap<String, Vec<f64>>,
    ) -> FactorResult<()> {
        let inputs = FactorInputs {
            raw: context,
            computed: cache,
        };
        let result = (factor.compute)(&inputs)?;
        if result.len() != context.row_count() {
            return Err(FactorError::LengthMismatch {
                name: factor.name.clone(),
                expected: context.row_count(),
                actual: result.len(),
            });
        }
        cache.insert(factor.name.clone(), result);
        Ok(())
    }

    fn evaluate_precompiled_raw(
        &self,
        execution_order: &[String],
        required_raw_inputs: &[String],
        context: &dyn RawFactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        let allowed_raw: BTreeSet<&str> = required_raw_inputs.iter().map(String::as_str).collect();
        for input in &allowed_raw {
            if context.raw_get(input).is_none() {
                return Err(FactorError::MissingInput((*input).to_string()));
            }
        }

        let mut cache = BTreeMap::new();
        for name in execution_order {
            let factor = self
                .registry
                .get(name)
                .ok_or_else(|| FactorError::UnknownFactor(name.clone()))?;
            for dependency in &factor.dependencies {
                if self.registry.get(dependency).is_some() {
                    if !cache.contains_key(dependency) {
                        return Err(FactorError::InvalidParameter(format!(
                            "stale factor plan: dependency {dependency} must execute before {name}"
                        )));
                    }
                } else {
                    if !allowed_raw.contains(dependency.as_str()) {
                        return Err(FactorError::InvalidParameter(format!(
                            "stale factor plan: raw dependency changed for {name}: {dependency}"
                        )));
                    }
                    if context.raw_get(dependency).is_none() {
                        return Err(FactorError::MissingInput(dependency.clone()));
                    }
                }
            }
            self.compute_factor(factor, context, &mut cache)?;
        }
        Ok(cache)
    }
}

impl crate::compute::FactorPlan {
    /// Execute this plan in its precompiled topological order over owned input.
    ///
    /// Unlike the compatibility `FactorPlan::execute` entry point introduced
    /// in the first planning phase, this method does not recursively traverse
    /// dependencies on every call. It also rejects a registry whose dependency
    /// graph no longer matches the compiled order/raw-input manifest.
    pub fn execute_precompiled(
        &self,
        engine: &FactorEngine,
        context: &FactorContext,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        engine.evaluate_precompiled_raw(self.execution_order(), self.required_raw_inputs(), context)
    }

    /// Execute this plan in precompiled order over zero-copy borrowed input.
    pub fn execute_borrowed(
        &self,
        engine: &FactorEngine,
        context: &BorrowedFactorContext<'_>,
    ) -> FactorResult<BTreeMap<String, Vec<f64>>> {
        engine.evaluate_precompiled_raw(self.execution_order(), self.required_raw_inputs(), context)
    }
}

/// Simple return over `period` bars with NaN warm-up values.
pub fn time_series_return(values: &[f64], period: usize) -> FactorResult<Vec<f64>> {
    if period == 0 {
        return Err(FactorError::InvalidParameter(
            "period must be greater than zero".to_string(),
        ));
    }
    let mut output = vec![f64::NAN; values.len()];
    for index in period..values.len() {
        let current = values[index];
        let previous = values[index - period];
        if current.is_finite() && previous.is_finite() && previous != 0.0 {
            output[index] = current / previous - 1.0;
        }
    }
    Ok(output)
}

/// Rolling population volatility of one-bar returns.
pub fn rolling_volatility(values: &[f64], period: usize) -> FactorResult<Vec<f64>> {
    if period < 2 {
        return Err(FactorError::InvalidParameter(
            "volatility period must be at least two".to_string(),
        ));
    }
    let returns = time_series_return(values, 1)?;
    let mut output = vec![f64::NAN; values.len()];
    for index in period..values.len() {
        let window = &returns[index + 1 - period..=index];
        if window.iter().any(|value| !value.is_finite()) {
            continue;
        }
        let mean = window.iter().sum::<f64>() / period as f64;
        let variance = window
            .iter()
            .map(|value| {
                let delta = value - mean;
                delta * delta
            })
            .sum::<f64>()
            / period as f64;
        output[index] = variance.sqrt();
    }
    Ok(output)
}

/// Z-score finite values while preserving NaNs and infinities as NaN.
pub fn zscore(values: &[f64]) -> Vec<f64> {
    let finite: Vec<f64> = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if finite.is_empty() {
        return vec![f64::NAN; values.len()];
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    let variance = finite
        .iter()
        .map(|value| {
            let delta = value - mean;
            delta * delta
        })
        .sum::<f64>()
        / finite.len() as f64;
    let std = variance.sqrt();
    if std <= f64::EPSILON {
        return values
            .iter()
            .map(|value| if value.is_finite() { 0.0 } else { f64::NAN })
            .collect();
    }
    values
        .iter()
        .map(|value| {
            if value.is_finite() {
                (value - mean) / std
            } else {
                f64::NAN
            }
        })
        .collect()
}

/// Percentile rank finite values into `[0, 1]`, averaging ties.
pub fn percentile_rank(values: &[f64]) -> Vec<f64> {
    let mut finite: Vec<(usize, f64)> = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .collect();
    finite.sort_by(|left, right| left.1.total_cmp(&right.1));

    let mut output = vec![f64::NAN; values.len()];
    if finite.len() == 1 {
        output[finite[0].0] = 0.5;
        return output;
    }

    let denominator = (finite.len() - 1) as f64;
    let mut start = 0;
    while start < finite.len() {
        let mut end = start + 1;
        while end < finite.len() && finite[end].1 == finite[start].1 {
            end += 1;
        }
        let average_position = (start + end - 1) as f64 / 2.0;
        let rank = average_position / denominator;
        for &(original_index, _) in &finite[start..end] {
            output[original_index] = rank;
        }
        start = end;
    }
    output
}

/// Clamp finite observations to lower and upper empirical quantiles.
pub fn winsorize(values: &[f64], lower: f64, upper: f64) -> FactorResult<Vec<f64>> {
    if !(0.0..=1.0).contains(&lower) || !(0.0..=1.0).contains(&upper) || lower > upper {
        return Err(FactorError::InvalidParameter(
            "winsorize quantiles must satisfy 0 <= lower <= upper <= 1".to_string(),
        ));
    }
    let mut sorted: Vec<f64> = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();
    if sorted.is_empty() {
        return Ok(vec![f64::NAN; values.len()]);
    }
    sorted.sort_by(f64::total_cmp);
    let lower_bound = quantile_sorted(&sorted, lower);
    let upper_bound = quantile_sorted(&sorted, upper);
    Ok(values
        .iter()
        .map(|value| {
            if value.is_finite() {
                value.clamp(lower_bound, upper_bound)
            } else {
                f64::NAN
            }
        })
        .collect())
}

/// Remove a single linear exposure with an intercept using OLS residuals.
pub fn neutralize(values: &[f64], exposure: &[f64]) -> FactorResult<Vec<f64>> {
    if values.len() != exposure.len() {
        return Err(FactorError::LengthMismatch {
            name: "exposure".to_string(),
            expected: values.len(),
            actual: exposure.len(),
        });
    }
    let pairs: Vec<(f64, f64)> = values
        .iter()
        .copied()
        .zip(exposure.iter().copied())
        .filter(|(value, factor)| value.is_finite() && factor.is_finite())
        .collect();
    if pairs.len() < 2 {
        return Ok(vec![f64::NAN; values.len()]);
    }

    let count = pairs.len() as f64;
    let mean_y = pairs.iter().map(|(value, _)| value).sum::<f64>() / count;
    let mean_x = pairs.iter().map(|(_, factor)| factor).sum::<f64>() / count;
    let covariance = pairs
        .iter()
        .map(|(value, factor)| (factor - mean_x) * (value - mean_y))
        .sum::<f64>();
    let variance_x = pairs
        .iter()
        .map(|(_, factor)| {
            let delta = factor - mean_x;
            delta * delta
        })
        .sum::<f64>();
    let beta = if variance_x > f64::EPSILON {
        covariance / variance_x
    } else {
        0.0
    };
    let intercept = mean_y - beta * mean_x;

    Ok(values
        .iter()
        .copied()
        .zip(exposure.iter().copied())
        .map(|(value, factor)| {
            if value.is_finite() && factor.is_finite() {
                value - intercept - beta * factor
            } else {
                f64::NAN
            }
        })
        .collect())
}

/// Build the stable v0.1.2 built-in price-factor registry.
pub fn builtin_factor_registry() -> FactorRegistry {
    let mut registry = FactorRegistry::new();
    for period in [5_usize, 20, 60] {
        let name = format!("momentum_{period}");
        let definition = FactorDefinition::new(
            name,
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::HigherBetter,
            Arc::new(move |inputs| time_series_return(inputs.get("close")?, period)),
        );
        registry
            .register(definition)
            .expect("built-in factor names are unique");
    }
    registry
        .register(FactorDefinition::new(
            "volatility_20",
            ["close"],
            FactorKind::TimeSeries,
            FactorDirection::LowerBetter,
            Arc::new(|inputs| rolling_volatility(inputs.get("close")?, 20)),
        ))
        .expect("built-in factor names are unique");
    registry
        .register(FactorDefinition::new(
            "reversal_5",
            ["momentum_5"],
            FactorKind::TimeSeries,
            FactorDirection::HigherBetter,
            Arc::new(|inputs| {
                Ok(inputs
                    .get("momentum_5")?
                    .iter()
                    .map(|value| if value.is_finite() { -*value } else { f64::NAN })
                    .collect())
            }),
        ))
        .expect("built-in factor names are unique");
    registry
}

fn quantile_sorted(sorted: &[f64], quantile: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = quantile * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
    }
}

/// Return the transitive dependency names for a target factor.
pub fn dependency_set(registry: &FactorRegistry, target: &str) -> FactorResult<BTreeSet<String>> {
    let mut dependencies = BTreeSet::new();
    let mut visiting = Vec::new();
    collect_dependencies(registry, target, &mut dependencies, &mut visiting)?;
    dependencies.remove(target);
    Ok(dependencies)
}

fn collect_dependencies(
    registry: &FactorRegistry,
    name: &str,
    output: &mut BTreeSet<String>,
    visiting: &mut Vec<String>,
) -> FactorResult<()> {
    if let Some(position) = visiting.iter().position(|current| current == name) {
        let mut cycle = visiting[position..].to_vec();
        cycle.push(name.to_string());
        return Err(FactorError::DependencyCycle(cycle));
    }
    let factor = registry
        .get(name)
        .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
    if !output.insert(name.to_string()) {
        return Ok(());
    }
    visiting.push(name.to_string());
    for dependency in &factor.dependencies {
        if registry.get(dependency).is_some() {
            collect_dependencies(registry, dependency, output, visiting)?;
        }
    }
    visiting.pop();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::FactorPlan;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn factor_names_must_not_be_empty() {
        let mut context = FactorContext::new();
        assert!(matches!(
            context.insert("   ", vec![]),
            Err(FactorError::InvalidParameter(_))
        ));

        let mut registry = FactorRegistry::new();
        let error = registry
            .register(FactorDefinition::new(
                "",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|_| Ok(Vec::new())),
            ))
            .unwrap_err();
        assert!(error.to_string().contains("factor name must not be empty"));
    }

    #[test]
    fn dependency_results_are_cached_once() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dep_calls = Arc::clone(&calls);
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "base",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(move |inputs| {
                    dep_calls.fetch_add(1, Ordering::SeqCst);
                    Ok(inputs.get("close")?.to_vec())
                }),
            ))
            .unwrap();
        for name in ["a", "b"] {
            registry
                .register(FactorDefinition::new(
                    name,
                    ["base"],
                    FactorKind::TimeSeries,
                    FactorDirection::HigherBetter,
                    Arc::new(|inputs| Ok(inputs.get("base")?.to_vec())),
                ))
                .unwrap();
        }
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .unwrap();
        FactorEngine::new(registry)
            .evaluate_many(&["a", "b"], &context)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn cycles_are_rejected() {
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "a",
                ["b"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| Ok(inputs.get("b")?.to_vec())),
            ))
            .unwrap();
        registry
            .register(FactorDefinition::new(
                "b",
                ["a"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| Ok(inputs.get("a")?.to_vec())),
            ))
            .unwrap();
        let error = FactorEngine::new(registry)
            .evaluate("a", &FactorContext::new())
            .unwrap_err();
        assert!(matches!(error, FactorError::DependencyCycle(_)));
    }

    #[test]
    fn composite_renormalizes_missing_rows() {
        let mut registry = FactorRegistry::new();
        registry
            .register(FactorDefinition::new(
                "a",
                ["a_raw"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| Ok(inputs.get("a_raw")?.to_vec())),
            ))
            .unwrap();
        registry
            .register(FactorDefinition::new(
                "b",
                ["b_raw"],
                FactorKind::TimeSeries,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| Ok(inputs.get("b_raw")?.to_vec())),
            ))
            .unwrap();
        let mut context = FactorContext::new();
        context.insert("a_raw", vec![2.0, 2.0]).unwrap();
        context.insert("b_raw", vec![2.0, f64::NAN]).unwrap();
        let score = FactorEngine::new(registry)
            .composite(&[("a", 1.0), ("b", 1.0)], &context)
            .unwrap();
        assert_eq!(score, vec![2.0, 2.0]);
    }

    #[test]
    fn percentile_rank_preserves_nan_and_averages_ties() {
        let ranks = percentile_rank(&[3.0, 1.0, 3.0, f64::NAN, 2.0]);
        assert!((ranks[0] - 5.0 / 6.0).abs() < 1e-12);
        assert!((ranks[2] - 5.0 / 6.0).abs() < 1e-12);
        assert_eq!(ranks[1], 0.0);
        assert!(ranks[3].is_nan());
        assert!((ranks[4] - 1.0 / 3.0).abs() < 1e-12);
    }

    #[test]
    fn neutralization_removes_linear_exposure() {
        let exposure = [1.0, 2.0, 3.0, 4.0, 5.0];
        let values = [5.0, 7.0, 9.0, 11.0, 13.0];
        let residual = neutralize(&values, &exposure).unwrap();
        assert!(residual.iter().all(|value| value.abs() < 1e-12));
    }

    #[test]
    fn momentum_has_explicit_warmup() {
        let output = time_series_return(&[10.0, 11.0, 12.0, 13.0], 2).unwrap();
        assert!(output[0].is_nan());
        assert!(output[1].is_nan());
        assert!((output[2] - 0.2).abs() < 1e-12);
    }

    #[test]
    fn builtins_include_dependency_factor() {
        let registry = builtin_factor_registry();
        let dependencies = dependency_set(&registry, "reversal_5").unwrap();
        assert!(dependencies.contains("momentum_5"));
    }

    #[test]
    fn market_frame_is_borrowed_without_copying_numeric_columns() {
        let open = [1.0, 2.0, 3.0];
        let high = [2.0, 3.0, 4.0];
        let low = [0.5, 1.5, 2.5];
        let close = [1.5, 2.5, 3.5];
        let volume = [10.0, 20.0, 30.0];
        let frame = MarketFrame::new(&open, &high, &low, &close, &volume).unwrap();
        let context = BorrowedFactorContext::from_market_frame(frame).unwrap();

        assert_eq!(context.get("close").unwrap().as_ptr(), close.as_ptr());
        assert_eq!(context.get("volume").unwrap().as_ptr(), volume.as_ptr());
    }

    #[test]
    fn borrowed_and_owned_factor_execution_are_equivalent() {
        let close = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0];
        let borrowed = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();
        let owned = FactorContext::new()
            .with_series("close", close.to_vec())
            .unwrap();
        let engine = FactorEngine::new(builtin_factor_registry());

        let borrowed_result = engine.evaluate_borrowed("momentum_5", &borrowed).unwrap();
        let owned_result = engine.evaluate("momentum_5", &owned).unwrap();
        assert_eq!(borrowed_result, owned_result);
    }

    #[test]
    fn factor_plan_executes_precompiled_order_for_borrowed_input() {
        let registry = builtin_factor_registry();
        let plan = FactorPlan::compile(&registry, &["reversal_5"]).unwrap();
        let engine = FactorEngine::new(registry);
        let close = [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();

        let values = plan.execute_borrowed(&engine, &context).unwrap();
        assert!(values.contains_key("momentum_5"));
        assert!(values.contains_key("reversal_5"));
    }

    #[test]
    fn precompiled_plan_rejects_changed_dependency_graph() {
        let mut original = FactorRegistry::new();
        original
            .register(FactorDefinition::new(
                "score",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| Ok(inputs.get("close")?.to_vec())),
            ))
            .unwrap();
        let plan = FactorPlan::compile(&original, &["score"]).unwrap();

        let mut changed = FactorRegistry::new();
        changed
            .register(FactorDefinition::new(
                "score",
                ["volume"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| Ok(inputs.get("volume")?.to_vec())),
            ))
            .unwrap();
        let context = FactorContext::new()
            .with_series("close", vec![1.0, 2.0])
            .unwrap()
            .with_series("volume", vec![10.0, 20.0])
            .unwrap();
        let error = plan
            .execute_precompiled(&FactorEngine::new(changed), &context)
            .unwrap_err();
        assert!(error.to_string().contains("stale factor plan"));
    }
}
