//! Canonical operation contracts shared by indicators, formulas, factors,
//! composites, and drawing adapters.
//!
//! The registry is the stable discovery and planning layer. The unified façade
//! below routes the currently executable Formula, Factor, and Composite paths
//! through one request/result/error contract; each new operation still needs a
//! verified dispatcher and golden vectors before it is marked fully complete.

use crate::composite::{CompositeDefinition, CompositeEngine};
use crate::data_contract::{CrossSectionView, DataContractError, MarketPanel};
use crate::factors::{BorrowedFactorContext, FactorEngine, FactorRegistry};
use crate::formula::{
    parse_formula_with_dialect, AstNode, DrawResult, FormulaContext, FormulaDialect, FormulaEngine,
    FormulaError,
};
use crate::registry::{
    builtin_function_registry, FunctionCategory, FunctionSpec, InputKind, LookbackSpec, ParamSpec,
};
use ndarray::Array1;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fmt;

/// Reserved name used for the primary value returned by a formula.
pub const PRIMARY_OUTPUT_NAME: &str = "__PRIMARY__";

/// Top-level kind of a public operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationKind {
    /// TA-Lib-style or native technical indicator.
    Indicator,
    /// A callable formula-language primitive.
    FormulaFunction,
    /// A reusable one-output or multi-output factor transform.
    Factor,
    /// A composition of operations and/or factors.
    Composite,
    /// A rendering instruction or derived chart series.
    Draw,
}

/// Shape of an operation's value result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValueShape {
    /// One value per row.
    Series,
    /// Multiple aligned series per row, such as MACD or Bollinger Bands.
    MultiSeries,
    /// One value per symbol and timestamp.
    CrossSection,
    /// Discrete events aligned to rows.
    Event,
    /// A non-row result, such as a report or summary.
    Report,
}

/// Explicit execution and semantic capabilities.
///
/// A capability is only set when the underlying implementation has a public
/// contract for it. In particular, `parallel`, `multi_symbol`, and
/// `multi_timeframe` are not inferred from a batch function accepting slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationCapabilities {
    /// Supports finite batch evaluation.
    pub batch: bool,
    /// Supports incremental/stateful evaluation.
    pub streaming: bool,
    /// Supports a cross-sectional execution context.
    pub cross_sectional: bool,
    /// Accepts multiple symbols as one logical request.
    pub multi_symbol: bool,
    /// Accepts multiple explicit timeframes without implicit resampling.
    pub multi_timeframe: bool,
    /// Can be partitioned by the planner without changing results.
    pub parallel: bool,
    /// Repeated evaluation is deterministic for identical inputs/configuration.
    pub deterministic: bool,
    /// Does not read future rows.
    pub causal: bool,
    /// May intentionally read future rows.
    pub lookahead: bool,
    /// May revise prior values after new input arrives.
    pub repaint: bool,
    /// Requires mutable state between evaluations.
    pub stateful: bool,
    /// Produces or mutates a drawing scene.
    pub drawable: bool,
}

impl OperationCapabilities {
    /// Conservative capabilities for a regular causal indicator.
    pub const fn indicator(streaming: bool, deterministic: bool) -> Self {
        Self {
            batch: true,
            streaming,
            cross_sectional: false,
            multi_symbol: false,
            multi_timeframe: false,
            parallel: false,
            deterministic,
            causal: true,
            lookahead: false,
            repaint: false,
            stateful: streaming,
            drawable: false,
        }
    }
}

/// Stable identifier derived from the canonical normalized operation name.
///
/// The identifier is not a cryptographic hash. The registry checks collisions
/// at registration time, and the canonical name remains the human-readable
/// source of truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(pub u32);

impl OperationId {
    /// Build the stable id for a canonical operation name.
    pub fn from_name(name: &str) -> Self {
        let normalized = normalize_name(name);
        let mut hash = 0x811c_9dc5_u32;
        for byte in normalized.bytes() {
            hash ^= u32::from(byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
        Self(hash)
    }
}

/// Canonical metadata consumed by planners, bindings, docs, and adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSpec {
    /// Canonical uppercase name.
    pub name: String,
    /// Accepted aliases, normalized at registration time.
    pub aliases: Vec<String>,
    /// Stable operation kind.
    pub kind: OperationKind,
    /// Result shape.
    pub value_shape: ValueShape,
    /// Legacy discovery category where one exists.
    pub category: Option<FunctionCategory>,
    /// Input contract inherited from the function registry when available.
    pub input: Option<InputKind>,
    /// Parameter declarations.
    pub params: Vec<ParamSpec>,
    /// Number of aligned output series.
    pub outputs: usize,
    /// Warm-up/lookback behavior.
    pub lookback: LookbackSpec,
    /// Execution and semantic capabilities.
    pub capabilities: OperationCapabilities,
    /// Version of this metadata contract.
    pub schema_version: u16,
}

impl OperationSpec {
    /// Project an existing public function description into the canonical
    /// operation contract without claiming capabilities it did not declare.
    pub fn from_function(spec: &FunctionSpec) -> Self {
        let kind = match spec.category {
            FunctionCategory::Formula => OperationKind::FormulaFunction,
            FunctionCategory::Factor => OperationKind::Factor,
            _ => OperationKind::Indicator,
        };
        let value_shape = if spec.outputs > 1 {
            ValueShape::MultiSeries
        } else {
            ValueShape::Series
        };
        Self {
            name: normalize_name(spec.name),
            aliases: spec
                .aliases
                .iter()
                .map(|alias| normalize_name(alias))
                .collect(),
            kind,
            value_shape,
            category: Some(spec.category),
            input: Some(spec.input),
            params: spec.params.to_vec(),
            outputs: spec.outputs,
            lookback: spec.lookback,
            capabilities: OperationCapabilities::indicator(spec.streaming, spec.deterministic),
            schema_version: 1,
        }
    }
}

/// Errors raised while registering a canonical operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationRegistryError {
    /// The canonical name or an alias is empty.
    EmptyName,
    /// The operation name is already registered.
    DuplicateName(String),
    /// An alias conflicts with an operation name or another alias.
    DuplicateAlias(String),
    /// Two canonical names produced the same stable id.
    IdCollision {
        id: OperationId,
        existing: String,
        incoming: String,
    },
}

impl fmt::Display for OperationRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("operation name must not be empty"),
            Self::DuplicateName(name) => write!(formatter, "operation already registered: {name}"),
            Self::DuplicateAlias(alias) => {
                write!(formatter, "operation alias already registered: {alias}")
            }
            Self::IdCollision {
                id,
                existing,
                incoming,
            } => write!(
                formatter,
                "operation id collision {}: {existing} vs {incoming}",
                id.0
            ),
        }
    }
}

impl std::error::Error for OperationRegistryError {}

/// Deterministic registry for the unified public operation catalog.
#[derive(Debug, Clone, Default)]
pub struct OperationRegistry {
    specs: BTreeMap<String, OperationSpec>,
    aliases: BTreeMap<String, String>,
    ids: BTreeMap<OperationId, String>,
}

impl OperationRegistry {
    /// Create an empty operation registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an operation atomically.
    pub fn register(&mut self, mut spec: OperationSpec) -> Result<(), OperationRegistryError> {
        let canonical = normalize_name(&spec.name);
        if canonical.is_empty() {
            return Err(OperationRegistryError::EmptyName);
        }
        if self.specs.contains_key(&canonical) || self.aliases.contains_key(&canonical) {
            return Err(OperationRegistryError::DuplicateName(canonical));
        }

        let id = OperationId::from_name(&canonical);
        if let Some(existing) = self.ids.get(&id) {
            return Err(OperationRegistryError::IdCollision {
                id,
                existing: existing.clone(),
                incoming: canonical,
            });
        }

        let mut normalized_aliases = BTreeMap::new();
        for alias in &spec.aliases {
            let normalized = normalize_name(alias);
            if normalized.is_empty()
                || normalized == canonical
                || self.specs.contains_key(&normalized)
                || self.aliases.contains_key(&normalized)
                || normalized_aliases.insert(normalized.clone(), ()).is_some()
            {
                return Err(OperationRegistryError::DuplicateAlias(normalized));
            }
        }

        spec.name = canonical.clone();
        spec.aliases = normalized_aliases.keys().cloned().collect();
        self.ids.insert(id, canonical.clone());
        for alias in spec.aliases.iter() {
            self.aliases.insert(alias.clone(), canonical.clone());
        }
        self.specs.insert(canonical, spec);
        Ok(())
    }

    /// Register all entries from the existing function metadata registry.
    pub fn from_function_registry(
        functions: &crate::registry::FunctionRegistry,
    ) -> Result<Self, OperationRegistryError> {
        let mut registry = Self::new();
        for function in functions.iter() {
            registry.register(OperationSpec::from_function(function))?;
        }
        Ok(registry)
    }

    /// Resolve a canonical name or alias case-insensitively.
    pub fn get(&self, name: &str) -> Option<&OperationSpec> {
        let normalized = normalize_name(name);
        self.specs.get(&normalized).or_else(|| {
            self.aliases
                .get(&normalized)
                .and_then(|canonical| self.specs.get(canonical))
        })
    }

    /// Resolve a stable id.
    pub fn get_by_id(&self, id: OperationId) -> Option<&OperationSpec> {
        self.ids.get(&id).and_then(|name| self.specs.get(name))
    }

    /// Iterate in stable canonical-name order.
    pub fn iter(&self) -> impl Iterator<Item = &OperationSpec> {
        self.specs.values()
    }

    /// Number of canonical operations.
    pub fn len(&self) -> usize {
        self.specs.len()
    }

    /// Whether no operations are registered.
    pub fn is_empty(&self) -> bool {
        self.specs.is_empty()
    }
}

/// Build the canonical catalog from all currently registered public
/// functions. Formula-language, factor, composite, and drawing entries will
/// be added through the same registry as their verified dispatchers land.
pub fn builtin_operation_registry() -> OperationRegistry {
    OperationRegistry::from_function_registry(&builtin_function_registry())
        .expect("built-in operation names and aliases are unique")
}

/// A request routed through the unified Formula/Factor/Composite façade.
///
/// The request borrows caller-owned contexts. No raw market data is copied at
/// dispatch time; ownership is transferred only in the returned result.
pub enum OperationRequest<'a> {
    /// Invoke one registered indicator directly without parsing a source string.
    Indicator {
        /// Canonical name or registered alias.
        name: &'a str,
        /// Named input series resolved by the formula context.
        inputs: &'a [&'a str],
        /// Numeric parameters in the operation's declared order.
        params: &'a [f64],
        /// Mutable input context.
        context: &'a mut FormulaContext,
    },
    /// Compile and execute a formula against a mutable formula context.
    Formula {
        /// Formula source in the selected dialect's canonical syntax.
        source: &'a str,
        /// Parser/semantic dialect used for the source.
        dialect: FormulaDialect,
        /// Mutable context, because formulas may assign variables or emit draw commands.
        context: &'a mut FormulaContext,
    },
    /// Evaluate one registered factor from a borrowed named-series context.
    Factor {
        /// Registered factor name.
        name: &'a str,
        /// Borrowed factor input context.
        context: &'a BorrowedFactorContext<'a>,
    },
    /// Evaluate selected composite outputs from a borrowed named-series context.
    Composite {
        /// Named composite definitions.
        definitions: &'a [CompositeDefinition],
        /// Requested output names.
        outputs: &'a [&'a str],
        /// Borrowed composite input context.
        context: &'a BorrowedFactorContext<'a>,
        /// Optional caller-owned revision used by the composite cache.
        data_revision: Option<u64>,
    },
    /// Evaluate one cross-sectional factor at every timestamp row.
    CrossSectionalFactor {
        /// Registered factor name.
        name: &'a str,
        /// Named cross-sectional input views with identical dimensions.
        inputs: &'a [(&'a str, &'a CrossSectionView<'a>)],
    },
}

/// Unified named-series result returned by Formula, Factor, and Composite execution.
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Named aligned output series. Keys are deterministic and operation-defined.
    pub values: BTreeMap<String, Vec<f64>>,
    /// Shape of the returned values.
    pub shape: ValueShape,
    /// Name of the primary output, if the operation has one.
    pub primary: Option<String>,
    /// Formula drawing commands, when the execution emitted any.
    pub draw: Option<DrawResult>,
}

/// Results keyed by the explicit symbol/timeframe frame that produced them.
#[derive(Debug, Clone)]
pub struct PanelOperationResult {
    /// Deterministically ordered results keyed by `FrameKey`.
    pub values: BTreeMap<crate::data_contract::FrameKey, OperationResult>,
}

impl PanelOperationResult {
    /// Number of symbol/timeframe frames evaluated.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the panel produced no frame results.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Return one frame result without copying it.
    pub fn get(&self, key: &crate::data_contract::FrameKey) -> Option<&OperationResult> {
        self.values.get(key)
    }
}

impl OperationResult {
    /// Return the primary output without copying it.
    pub fn primary_values(&self) -> Option<&[f64]> {
        self.primary
            .as_deref()
            .and_then(|name| self.values.get(name).map(Vec::as_slice))
    }

    /// Return a named output without copying it.
    pub fn get(&self, name: &str) -> Option<&[f64]> {
        self.values.get(name).map(Vec::as_slice)
    }

    /// Number of named output series.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the result contains no output series.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Errors surfaced by the unified operation façade.
#[derive(Debug, Clone, PartialEq)]
pub enum OperationExecutionError {
    /// The requested operation is absent from the canonical catalog.
    UnknownOperation(String),
    /// The request shape or numeric arguments are invalid before execution.
    InvalidRequest(String),
    /// Formula parsing, planning, or evaluation failed.
    Formula(FormulaError),
    /// Factor registration, dependency resolution, or evaluation failed.
    Factor(crate::factors::FactorError),
    /// Composite graph validation or evaluation failed.
    Composite(crate::factors::FactorError),
    /// Market-panel data violated the canonical dimension contract.
    DataContract(DataContractError),
}

impl fmt::Display for OperationExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOperation(name) => write!(formatter, "unknown operation: {name}"),
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid operation request: {message}")
            }
            Self::Formula(error) => write!(formatter, "formula operation failed: {error}"),
            Self::Factor(error) => write!(formatter, "factor operation failed: {error}"),
            Self::Composite(error) => write!(formatter, "composite operation failed: {error}"),
            Self::DataContract(error) => {
                write!(formatter, "operation data contract failed: {error}")
            }
        }
    }
}

impl std::error::Error for OperationExecutionError {}

impl From<FormulaError> for OperationExecutionError {
    fn from(value: FormulaError) -> Self {
        Self::Formula(value)
    }
}

impl From<DataContractError> for OperationExecutionError {
    fn from(value: DataContractError) -> Self {
        Self::DataContract(value)
    }
}

/// Runtime façade that routes the three currently executable high-level
/// operation families through one request/result/error contract.
pub struct UnifiedOperationEngine {
    catalog: OperationRegistry,
    formula: FormulaEngine,
    factor: FactorEngine,
    composite: CompositeEngine,
}

impl UnifiedOperationEngine {
    /// Create an engine with the built-in operation catalog and caller factors.
    pub fn new(factors: FactorRegistry) -> Self {
        Self {
            catalog: builtin_operation_registry(),
            formula: FormulaEngine::new(),
            factor: FactorEngine::new(factors),
            composite: CompositeEngine::new(),
        }
    }

    /// Read the canonical metadata catalog used by this engine.
    pub fn catalog(&self) -> &OperationRegistry {
        &self.catalog
    }

    /// Access the factor engine for registration and precompiled workflows.
    pub fn factor_engine(&self) -> &FactorEngine {
        &self.factor
    }

    /// Access the composite engine for custom function registration and cache control.
    pub fn composite_engine(&self) -> &CompositeEngine {
        &self.composite
    }

    /// Mutably access the composite engine for custom function registration and cache control.
    pub fn composite_engine_mut(&mut self) -> &mut CompositeEngine {
        &mut self.composite
    }

    /// Execute a Formula, Factor, or Composite request using one result contract.
    pub fn execute<'a>(
        &mut self,
        request: OperationRequest<'a>,
    ) -> Result<OperationResult, OperationExecutionError> {
        match request {
            OperationRequest::Indicator {
                name,
                inputs,
                params,
                context,
            } => self.execute_indicator(name, inputs, params, context),
            OperationRequest::Formula {
                source,
                dialect,
                context,
            } => {
                let (values, draw) = self.execute_formula(source, dialect, context)?;
                let shape = if values.len() > 1 {
                    ValueShape::MultiSeries
                } else {
                    ValueShape::Series
                };
                Ok(OperationResult {
                    values,
                    shape,
                    primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
                    draw,
                })
            }
            OperationRequest::Factor { name, context } => {
                let result = self
                    .factor
                    .evaluate_borrowed(name, context)
                    .map_err(OperationExecutionError::Factor)?;
                let mut values = BTreeMap::new();
                values.insert(name.to_string(), result);
                Ok(OperationResult {
                    values,
                    shape: ValueShape::Series,
                    primary: Some(name.to_string()),
                    draw: None,
                })
            }
            OperationRequest::Composite {
                definitions,
                outputs,
                context,
                data_revision,
            } => {
                let values = match data_revision {
                    Some(revision) => {
                        self.composite
                            .evaluate_cached(definitions, outputs, context, revision)
                    }
                    None => self
                        .composite
                        .evaluate_borrowed(definitions, outputs, context),
                }
                .map_err(OperationExecutionError::Composite)?;
                let primary = (outputs.len() == 1).then(|| outputs[0].to_string());
                Ok(OperationResult {
                    values,
                    shape: if outputs.len() > 1 {
                        ValueShape::MultiSeries
                    } else {
                        ValueShape::Series
                    },
                    primary,
                    draw: None,
                })
            }
            OperationRequest::CrossSectionalFactor { name, inputs } => {
                let result = self
                    .factor
                    .evaluate_cross_sectional(name, inputs)
                    .map_err(OperationExecutionError::Factor)?;
                let values = std::iter::once((name.to_string(), result.values)).collect();
                Ok(OperationResult {
                    values,
                    shape: ValueShape::CrossSection,
                    primary: Some(name.to_string()),
                    draw: None,
                })
            }
        }
    }

    fn execute_indicator(
        &mut self,
        name: &str,
        inputs: &[&str],
        params: &[f64],
        context: &mut FormulaContext,
    ) -> Result<OperationResult, OperationExecutionError> {
        let spec = self
            .catalog
            .get(name)
            .ok_or_else(|| OperationExecutionError::UnknownOperation(name.to_string()))?;
        if inputs.iter().any(|input| input.trim().is_empty()) {
            return Err(OperationExecutionError::InvalidRequest(
                "indicator input names must not be empty".to_string(),
            ));
        }
        if params.iter().any(|value| !value.is_finite()) {
            return Err(OperationExecutionError::InvalidRequest(
                "indicator parameters must be finite".to_string(),
            ));
        }

        let mut args = Vec::with_capacity(inputs.len() + params.len());
        args.extend(
            inputs
                .iter()
                .map(|input| AstNode::Variable((*input).to_string())),
        );
        args.extend(params.iter().copied().map(AstNode::Number));
        let ast = AstNode::FunctionCall {
            name: spec.name.clone(),
            args,
        };
        let result = self.formula.eval_ast(&ast, context)?;
        let mut values = BTreeMap::new();
        values.insert(PRIMARY_OUTPUT_NAME.to_string(), result.to_vec());
        Ok(OperationResult {
            values,
            shape: if spec.outputs > 1 {
                ValueShape::MultiSeries
            } else {
                ValueShape::Series
            },
            primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
            draw: None,
        })
    }

    /// Execute one formula independently for every explicit symbol/timeframe frame.
    ///
    /// This method intentionally returns a panel result rather than forcing
    /// multi-dimensional data into the single-operation result map.
    pub fn execute_panel_formula(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        panel: &MarketPanel<'_>,
    ) -> Result<PanelOperationResult, OperationExecutionError> {
        let mut values = BTreeMap::new();
        for (key, frame) in panel.iter() {
            frame
                .validate()
                .map_err(|error| DataContractError::InvalidMarketFrame(error))?;
            let amount = frame.amount.map(|series| Array1::from_vec(series.to_vec()));
            let mut context = FormulaContext::from_borrowed_ohlcv(
                frame.open,
                frame.high,
                frame.low,
                frame.close,
                frame.volume,
                amount,
            );
            context.datetime = frame
                .timestamp
                .map(|timestamps| Array1::from_vec(timestamps.to_vec()));
            let (frame_values, draw) = self.execute_formula(source, dialect, &mut context)?;
            let shape = if frame_values.len() > 1 {
                ValueShape::MultiSeries
            } else {
                ValueShape::Series
            };
            values.insert(
                key.clone(),
                OperationResult {
                    values: frame_values,
                    shape,
                    primary: Some(PRIMARY_OUTPUT_NAME.to_string()),
                    draw,
                },
            );
        }
        Ok(PanelOperationResult { values })
    }

    fn execute_formula(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        context: &mut FormulaContext,
    ) -> Result<(BTreeMap<String, Vec<f64>>, Option<DrawResult>), FormulaError> {
        let mut values = BTreeMap::new();
        match dialect {
            FormulaDialect::AlphaTA => {
                let result = self.formula.eval_multi(source, context)?;
                for (name, value) in result.outputs {
                    values.insert(name, value.to_vec());
                }
                values.insert(PRIMARY_OUTPUT_NAME.to_string(), result.final_value.to_vec());
            }
            FormulaDialect::Pine => {
                let ast = parse_formula_with_dialect(source, dialect)
                    .map_err(FormulaError::ParseError)?;
                let variables_before: HashSet<String> =
                    context.variables.keys().map(ToString::to_string).collect();
                let final_value = self.formula.eval_ast(&ast, context)?;
                for (name, value) in &context.variables {
                    let name = name.to_string();
                    if !variables_before.contains(&name) {
                        values.insert(name, value.to_vec());
                    }
                }
                values.insert(PRIMARY_OUTPUT_NAME.to_string(), final_value.to_vec());
            }
        }
        let draw = {
            let draw = context.draw_commands.borrow();
            (!draw.commands.is_empty()).then(|| draw.clone())
        };
        Ok((values, draw))
    }
}

fn normalize_name(name: &str) -> String {
    name.trim().to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite::{CompositeExpr, CompositeOp};
    use crate::data_contract::FrameKey;
    use crate::factors::{FactorDefinition, FactorDirection, FactorKind};
    use crate::runtime::MarketFrame;
    use ndarray::Array1;
    use std::sync::Arc;

    #[test]
    fn builtins_project_without_losing_function_contracts() {
        let functions = builtin_function_registry();
        let operations = OperationRegistry::from_function_registry(&functions).unwrap();

        assert_eq!(operations.len(), functions.len());
        let ema = operations.get(" ema ").unwrap();
        assert_eq!(ema.kind, OperationKind::Indicator);
        assert_eq!(ema.value_shape, ValueShape::Series);
        assert!(ema.capabilities.batch);
        assert!(ema.capabilities.streaming);
        assert!(ema.capabilities.causal);
        assert!(!ema.capabilities.lookahead);
    }

    #[test]
    fn ids_are_case_insensitive_and_resolvable() {
        let operations = builtin_operation_registry();
        let ema = operations.get("EMA").unwrap();
        let id = OperationId::from_name("ema");

        assert_eq!(operations.get_by_id(id), Some(ema));
        assert_eq!(id, OperationId::from_name(" EMA "));
    }

    #[test]
    fn registration_rejects_alias_conflicts_atomically() {
        let mut operations = OperationRegistry::new();
        let first = OperationSpec {
            name: "FIRST".to_string(),
            aliases: vec!["one".to_string()],
            kind: OperationKind::Indicator,
            value_shape: ValueShape::Series,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 1,
            lookback: LookbackSpec::None,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };
        operations.register(first).unwrap();

        let conflicting = OperationSpec {
            name: "SECOND".to_string(),
            aliases: vec!["ONE".to_string(), "two".to_string()],
            kind: OperationKind::Composite,
            value_shape: ValueShape::MultiSeries,
            category: None,
            input: None,
            params: Vec::new(),
            outputs: 2,
            lookback: LookbackSpec::Dynamic,
            capabilities: OperationCapabilities::indicator(false, true),
            schema_version: 1,
        };

        assert!(matches!(
            operations.register(conflicting),
            Err(OperationRegistryError::DuplicateAlias(_))
        ));
        assert_eq!(operations.len(), 1);
        assert!(operations.get("SECOND").is_none());
        assert!(operations.get("TWO").is_none());
    }

    fn formula_context() -> FormulaContext {
        let values = |start| Array1::from_vec((0..6).map(|index| start + index as f64).collect());
        FormulaContext::new(
            values(1.0),
            values(2.0),
            values(0.0),
            values(10.0),
            values(100.0),
            None,
        )
    }

    #[test]
    fn unified_engine_executes_formula_and_preserves_primary_result() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let result = engine
            .execute(OperationRequest::Formula {
                source: "SMA(CLOSE, 3)",
                dialect: FormulaDialect::AlphaTA,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.primary.as_deref(), Some(PRIMARY_OUTPUT_NAME));
        assert_eq!(result.primary_values().unwrap().len(), 6);
        assert!(result.draw.is_none());
    }

    #[test]
    fn unified_engine_dispatches_registered_indicator_without_source_parsing() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let inputs = ["CLOSE"];
        let params = [2.0];
        let result = engine
            .execute(OperationRequest::Indicator {
                name: "ema",
                inputs: &inputs,
                params: &params,
                context: &mut context,
            })
            .unwrap();

        assert_eq!(result.shape, ValueShape::Series);
        assert_eq!(result.primary_values().unwrap().len(), 6);
        assert!(result
            .primary_values()
            .unwrap()
            .iter()
            .any(|value| value.is_finite()));
        let direct = result.primary_values().unwrap().to_vec();
        let mut formula_context = formula_context();
        let formula = engine
            .execute(OperationRequest::Formula {
                source: "EMA(CLOSE, 2)",
                dialect: FormulaDialect::AlphaTA,
                context: &mut formula_context,
            })
            .unwrap();
        for (actual, expected) in direct.iter().zip(formula.primary_values().unwrap()) {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() <= 1e-12,
                "direct and formula dispatch differ: {actual} vs {expected}"
            );
        }
    }

    #[test]
    fn unified_engine_routes_pine_subset_to_the_same_result_contract() {
        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let mut context = formula_context();
        let result = engine
            .execute(OperationRequest::Formula {
                source: "//@version=5\nindicator(\"M\")\nr = ta.sma(close, 3)\nplot(r)\n",
                dialect: FormulaDialect::Pine,
                context: &mut context,
            })
            .unwrap();

        assert!(result.primary_values().is_some());
        assert!(result.get("PLOT").is_some());
        assert!(result.draw.is_none());
    }

    #[test]
    fn panel_formula_keeps_symbol_and_timeframe_results_separate() {
        let open_a = [1.0, 2.0, 3.0];
        let close_a = [2.0, 3.0, 4.0];
        let open_b = [10.0, 12.0, 14.0];
        let close_b = [11.0, 13.0, 15.0];
        let timestamps = [100, 200, 300];
        let frame_a = MarketFrame::new(&open_a, &close_a, &open_a, &close_a, &open_a)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let frame_b = MarketFrame::new(&open_b, &close_b, &open_b, &close_b, &open_b)
            .unwrap()
            .with_timestamp(&timestamps)
            .unwrap();
        let mut panel = MarketPanel::new();
        let key_a = FrameKey::new("AAA", "1d").unwrap();
        let key_b = FrameKey::new("BBB", "5m").unwrap();
        panel.insert(key_a.clone(), frame_a).unwrap();
        panel.insert(key_b.clone(), frame_b).unwrap();

        let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
        let result = engine
            .execute_panel_formula("SMA(CLOSE, 2)", FormulaDialect::AlphaTA, &panel)
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result.get(&key_a).unwrap().shape, ValueShape::Series);
        assert_eq!(result.get(&key_b).unwrap().shape, ValueShape::Series);
        let values_a = result.get(&key_a).unwrap().primary_values().unwrap();
        let values_b = result.get(&key_b).unwrap().primary_values().unwrap();
        assert_eq!(values_a, &[2.0, 2.5, 3.25]);
        assert_eq!(values_b, &[11.0, 12.0, 13.5]);
    }

    #[test]
    fn unified_engine_routes_factor_and_composite_through_existing_engines() {
        let mut factors = FactorRegistry::new();
        factors
            .register(FactorDefinition::new(
                "DOUBLE_CLOSE",
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("close")?
                        .iter()
                        .map(|value| value * 2.0)
                        .collect())
                }),
            ))
            .unwrap();
        factors
            .register(FactorDefinition::new(
                "ROW_SHIFT",
                ["score"],
                FactorKind::CrossSectional,
                FactorDirection::HigherBetter,
                Arc::new(|inputs| {
                    Ok(inputs
                        .get("score")?
                        .iter()
                        .map(|value| value + 1.0)
                        .collect())
                }),
            ))
            .unwrap();
        let mut engine = UnifiedOperationEngine::new(factors);
        let close = [1.0, 2.0, 3.0];
        let context = BorrowedFactorContext::new()
            .with_series("close", &close)
            .unwrap();

        let factor = engine
            .execute(OperationRequest::Factor {
                name: "DOUBLE_CLOSE",
                context: &context,
            })
            .unwrap();
        assert_eq!(factor.primary_values().unwrap(), &[2.0, 4.0, 6.0]);

        let timestamps = [10, 20];
        let symbols = ["AAA", "BBB", "CCC"];
        let scores = [1.0, 3.0, 2.0, 5.0, 4.0, 6.0];
        let view = CrossSectionView::new(&timestamps, &symbols, &scores).unwrap();
        let cross_inputs = [("score", &view)];
        let cross = engine
            .execute(OperationRequest::CrossSectionalFactor {
                name: "ROW_SHIFT",
                inputs: &cross_inputs,
            })
            .unwrap();
        assert_eq!(cross.shape, ValueShape::CrossSection);
        assert_eq!(
            cross.primary_values().unwrap(),
            &[2.0, 4.0, 3.0, 6.0, 5.0, 7.0]
        );

        let definitions = [CompositeDefinition::new(
            "SUM_CLOSE",
            CompositeExpr::Op {
                op: CompositeOp::Add,
                inputs: vec![CompositeExpr::series("close"), CompositeExpr::Constant(1.0)],
            },
        )];
        let outputs = ["SUM_CLOSE"];
        let composite = engine
            .execute(OperationRequest::Composite {
                definitions: &definitions,
                outputs: &outputs,
                context: &context,
                data_revision: Some(1),
            })
            .unwrap();
        assert_eq!(composite.primary_values().unwrap(), &[2.0, 3.0, 4.0]);
    }
}
