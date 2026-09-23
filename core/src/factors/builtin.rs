//! Shipped factor libraries, defined as expressions and compiled to plans.
//!
//! # Why expressions
//!
//! A factor library is a *specification*, and the specification that matters is
//! the one a reader can check. Alpha158's definition is 158 expression strings
//! published by Qlib; the module that implements it therefore holds those same
//! 158 strings, in Qlib's order, with Qlib's `+1e-12` guards and Qlib's
//! `Ref($close, 1)` spellings. Each one is parsed into a [`FactorGraph`] and
//! compiled once, so nothing about the arithmetic is re-stated in Rust and
//! nothing can drift from the reference by transcription.
//!
//! # Why the compile happens once
//!
//! [`CompiledFactor`] owns a [`FactorGraphPlan`], built when the library is
//! constructed. Evaluating a factor afterwards runs the plan directly: no
//! lexing, parsing, lowering, topological sort or register allocation is
//! repeated. That is the M0-3 acceptance criterion "第二次求值 0 解析开销", and it
//! is what [`CompiledFactor::plan`] exposes for a benchmark to demonstrate
//! rather than assert.
//!
//! The compiled plan is also what makes the factor runnable at all on the
//! production carrier: `FactorGraphPlan::execute` drives the unified executor,
//! which never silently falls back to the interpreter.
//!
//! # The dependency contract
//!
//! A factor's dependencies are the external series its expression reads. They
//! are resolved case-insensitively against the caller's [`FactorInputs`], and
//! the five OHLCV names bind to the context's own fields; anything else (the
//! `vwap` Alpha158's price block needs) is carried as a formula variable.

pub mod alpha158;
pub mod worldquant101;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use ndarray::Array1;

use crate::factor_graph::{FactorGraph, FactorGraphError, FactorGraphPlan};
use crate::factors::{
    FactorDefinition, FactorDirection, FactorError, FactorInputs, FactorKind, FactorRegistry,
    FactorResult,
};
use crate::formula::FormulaContext;

/// The built-in libraries, by the name [`factor_library`] accepts.
///
/// Kept as a single table so `factor_library`, the name list and the tests all
/// read from one place; a library that is not in here does not exist.
pub const LIBRARY_NAMES: &[&str] = &["alpha158", "worldquant101"];

/// A factor whose arithmetic is a compiled [`FactorGraphPlan`].
///
/// The plan is built once, at library construction. Evaluating the factor only
/// binds inputs and runs it.
pub struct CompiledFactor {
    name: String,
    /// The expression this factor was compiled from, kept verbatim so the
    /// library can be audited against its reference without re-deriving it.
    expression: String,
    /// External series the plan binds, canonical (upper-case) names.
    dependencies: Vec<String>,
    direction: FactorDirection,
    plan: FactorGraphPlan,
}

impl CompiledFactor {
    /// Compile one expression into a factor.
    ///
    /// # Errors
    ///
    /// Propagates [`FactorGraph::from_expression`]'s errors, plus
    /// [`FactorGraphError::UnknownPrimary`] in the degenerate case of an
    /// expression that declares no node at all (a bare variable reference).
    pub fn compile(
        name: impl Into<String>,
        expression: impl Into<String>,
        direction: FactorDirection,
    ) -> Result<Self, FactorGraphError> {
        let name = name.into();
        let expression = expression.into();
        let graph = FactorGraph::from_expression(&expression)?;
        let primary = graph
            .expression_primary()
            .ok_or_else(|| FactorGraphError::UnknownPrimary {
                id: expression.clone(),
            })?
            .to_string();
        let dependencies = graph.used_inputs();
        let plan = graph.build(&primary)?;
        Ok(Self {
            name,
            expression,
            dependencies,
            direction,
            plan,
        })
    }

    /// Factor name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The expression this factor compiles from.
    #[must_use]
    pub fn expression(&self) -> &str {
        &self.expression
    }

    /// External series this factor reads.
    #[must_use]
    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    /// Preferred ranking direction.
    #[must_use]
    pub const fn direction(&self) -> FactorDirection {
        self.direction
    }

    /// The compiled plan, for a benchmark that wants to show it is reused.
    #[must_use]
    pub const fn plan(&self) -> &FactorGraphPlan {
        &self.plan
    }

    /// Evaluate against a prepared formula context.
    ///
    /// This is the zero-copy entry point: the plan reads the context's series
    /// in place.
    ///
    /// # Errors
    ///
    /// [`FactorGraphError::MissingInput`] if the context lacks a series the plan
    /// binds, and [`FactorGraphError::Execution`] if the plan fails.
    pub fn evaluate(&self, ctx: &FormulaContext) -> Result<Vec<f64>, FactorGraphError> {
        self.plan.execute_node(ctx, self.plan.primary())
    }

    /// Adapter for the [`FactorRegistry`] callback ABI.
    ///
    /// Builds a borrowing [`FormulaContext`] over the inputs, so the only copy
    /// is for a non-OHLCV series such as `vwap`.
    fn evaluate_inputs(&self, inputs: &FactorInputs<'_>) -> FactorResult<Vec<f64>> {
        let context = self.context(inputs)?;
        self.evaluate(&context)
            .map_err(|error| FactorError::Compute(format!("{}: {error}", self.name)))
    }

    /// Bind `inputs` into a formula context the plan can read.
    fn context(&self, inputs: &FactorInputs<'_>) -> FactorResult<FormulaContext> {
        const NONE: &[f64] = &[];
        let (mut open, mut high, mut low, mut close, mut volume) = (NONE, NONE, NONE, NONE, NONE);
        let mut variables: Vec<(String, Vec<f64>)> = Vec::new();

        for dependency in &self.dependencies {
            let values = resolve(inputs, dependency)?;
            // The plan's expressions are canonical upper-case (`MA(CLOSE, 20)`),
            // so every slot must be bound under its canonical spelling however
            // the caller spelled it.
            match dependency.to_ascii_uppercase().as_str() {
                "OPEN" => open = values,
                "HIGH" => high = values,
                "LOW" => low = values,
                "CLOSE" => close = values,
                "VOLUME" => volume = values,
                _ => variables.push((dependency.to_ascii_uppercase(), values.to_vec())),
            }
        }

        let mut context = FormulaContext::from_borrowed_ohlcv(open, high, low, close, volume, None);
        for (name, values) in variables {
            context.set_variable(name, Array1::from_vec(values));
        }
        Ok(context)
    }

    /// A [`FactorDefinition`] that runs this compiled plan.
    ///
    /// Takes the shared handle rather than `&self` so the definition can keep
    /// the same [`CompiledFactor`] alive instead of cloning it: a
    /// `FactorGraphPlan` is not `Clone`, and rebuilding it here would recompile
    /// the graph that the whole design exists to compile once.
    fn definition(factor: &Arc<CompiledFactor>) -> FactorDefinition {
        let shared = Arc::clone(factor);
        let name = shared.name.clone();
        // Declared **lower-case**, which is the convention the shipped demo
        // factors already set (`momentum_5` declares `["close"]`) and therefore
        // the spelling a caller writes. This is not cosmetic: the engine
        // validates every dependency with an exact-match `raw_get` *before*
        // computing (`FactorEngine::evaluate_inner`), so a definition declaring
        // `CLOSE` is rejected outright for a caller holding `close` — the
        // fallback inside [`resolve`] never gets a chance to run. Canonicalising
        // happens in [`CompiledFactor::context`], where the plan needs it.
        let dependencies: Vec<String> = shared
            .dependencies
            .iter()
            .map(|dependency| dependency.to_ascii_lowercase())
            .collect();
        let direction = shared.direction;
        let compute = Arc::new(move |inputs: &FactorInputs<'_>| shared.evaluate_inputs(inputs))
            as crate::factors::FactorFn;
        FactorDefinition::new(
            name,
            dependencies,
            FactorKind::TimeSeries,
            direction,
            compute,
        )
    }
}

impl fmt::Debug for CompiledFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledFactor")
            .field("name", &self.name)
            .field("expression", &self.expression)
            .field("dependencies", &self.dependencies)
            .field("direction", &self.direction)
            .finish_non_exhaustive()
    }
}

/// Resolve one dependency under any of the three plausible spellings.
///
/// `FactorInputs::get` is an exact-match lookup, while the formula context
/// classifies OHLCV names case-insensitively and the library's own expressions
/// are canonical upper-case. A caller may reasonably hold any of `close`,
/// `CLOSE` or `Close`, and the compiled plan only understands the canonical
/// form, so the binding step accepts all three rather than forcing the caller to
/// know which spelling this module happens to use internally.
///
/// The three probes are ordered so the common case costs one lookup: the
/// declared spelling first (which is what the engine already validated), then
/// the other two.
fn resolve<'a>(inputs: &'a FactorInputs<'_>, name: &str) -> FactorResult<&'a [f64]> {
    inputs
        .get(name)
        .or_else(|_| inputs.get(&name.to_ascii_lowercase()))
        .or_else(|_| inputs.get(&name.to_ascii_uppercase()))
        .map_err(|_| FactorError::MissingInput(name.to_string()))
}

/// A named collection of [`CompiledFactor`]s.
pub struct FactorLibrary {
    name: &'static str,
    factors: BTreeMap<String, Arc<CompiledFactor>>,
}

impl FactorLibrary {
    /// Build a library from `(name, expression, direction)` triples.
    ///
    /// # Errors
    ///
    /// [`FactorError::DuplicateFactor`] for a repeated name, and
    /// [`FactorError::Compute`] if an expression fails to compile — the message
    /// carries the underlying graph error, because a library that silently
    /// dropped an uncompilable factor would under-report its own size.
    pub fn from_expressions(
        name: &'static str,
        expressions: &[(&str, &str, FactorDirection)],
    ) -> FactorResult<Self> {
        let mut factors = BTreeMap::new();
        for (factor_name, expression, direction) in expressions {
            let compiled = CompiledFactor::compile(*factor_name, *expression, *direction)
                .map_err(|error| FactorError::Compute(format!("{name}/{factor_name}: {error}")))?;
            let previous = factors.insert((*factor_name).to_string(), Arc::new(compiled));
            if previous.is_some() {
                return Err(FactorError::DuplicateFactor((*factor_name).to_string()));
            }
        }
        Ok(Self { name, factors })
    }

    /// Library name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Number of factors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.factors.len()
    }

    /// Whether the library holds no factors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.factors.is_empty()
    }

    /// Factor names in ascending order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.factors.keys().map(String::as_str)
    }

    /// Look up one factor.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Arc<CompiledFactor>> {
        self.factors.get(name)
    }

    /// Iterate over the factors in name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<CompiledFactor>)> {
        self.factors
            .iter()
            .map(|(name, factor)| (name.as_str(), factor))
    }

    /// Evaluate one factor against a prepared context.
    ///
    /// # Errors
    ///
    /// [`FactorError::UnknownFactor`] if `name` is not in this library.
    pub fn evaluate(&self, name: &str, ctx: &FormulaContext) -> FactorResult<Vec<f64>> {
        let factor = self
            .factors
            .get(name)
            .ok_or_else(|| FactorError::UnknownFactor(name.to_string()))?;
        factor
            .evaluate(ctx)
            .map_err(|error| FactorError::Compute(format!("{}: {error}", factor.name())))
    }

    /// A [`FactorRegistry`] holding every factor in this library.
    ///
    /// The definitions share the library's already-compiled plans, so registering
    /// a 158-factor library does not recompile 158 graphs.
    ///
    /// # Panics
    ///
    /// If two factors in the library share a name. [`FactorLibrary::from_expressions`]
    /// already rejects duplicates at construction, so reaching this means the
    /// library's own table was mutated inconsistently — a crate defect, not a
    /// caller error.
    #[must_use]
    pub fn registry(&self) -> FactorRegistry {
        let mut registry = FactorRegistry::new();
        for factor in self.factors.values() {
            registry
                .register(CompiledFactor::definition(factor))
                .expect("library factor names are unique");
        }
        registry
    }
}

impl fmt::Debug for FactorLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FactorLibrary")
            .field("name", &self.name)
            .field("factors", &self.factors.len())
            .finish()
    }
}

/// Build a built-in factor library by name.
///
/// ```
/// use finkit::factors::builtin::factor_library;
///
/// let library = factor_library("alpha158").unwrap();
/// assert_eq!(library.len(), 158);
/// assert!(library.get("MA20").is_some());
/// ```
///
/// # Errors
///
/// [`FactorError::UnknownLibrary`] if `name` is not one of [`LIBRARY_NAMES`],
/// and [`FactorError::Compute`] if a factor fails to compile — which would be a
/// bug in the library, not in the caller, and is surfaced rather than swallowed.
pub fn factor_library(name: &str) -> FactorResult<FactorLibrary> {
    match name {
        "alpha158" => alpha158::library(),
        "worldquant101" => worldquant101::library(),
        other => Err(FactorError::UnknownLibrary(other.to_string())),
    }
}

/// Every factor from every built-in library, as one registry.
///
/// # Errors
///
/// Propagates any library build failure.
pub fn builtin_library_registry() -> FactorResult<FactorRegistry> {
    let mut registry = FactorRegistry::new();
    for name in LIBRARY_NAMES {
        for definition in factor_library(name)?.registry().iter() {
            registry.register(definition.clone())?;
        }
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_library_names_are_rejected() {
        let error = factor_library("no-such-library").unwrap_err();
        assert_eq!(
            error,
            FactorError::UnknownLibrary("no-such-library".to_string())
        );
    }

    #[test]
    fn every_advertised_library_builds() {
        for name in LIBRARY_NAMES {
            let library = factor_library(name).expect("advertised library builds");
            assert_eq!(library.name(), *name);
            assert!(!library.is_empty(), "{name} is empty");
        }
    }

    #[test]
    fn a_factor_exposes_its_expression_and_dependencies() {
        let library = factor_library("alpha158").unwrap();
        let factor = library.get("MA20").expect("MA20 exists");
        // The expression is kept verbatim from the table, so it can be audited
        // against Qlib's `Mean($close, 20)/$close` as a token substitution.
        assert_eq!(factor.expression(), "MA(CLOSE, 20)/CLOSE");
        assert_eq!(factor.dependencies(), ["CLOSE"]);
    }

    #[test]
    fn duplicate_factor_names_are_rejected() {
        let error = FactorLibrary::from_expressions(
            "dupes",
            &[
                ("A", "CLOSE+0", FactorDirection::Neutral),
                ("A", "OPEN+0", FactorDirection::Neutral),
            ],
        )
        .unwrap_err();
        assert_eq!(error, FactorError::DuplicateFactor("A".to_string()));
    }

    #[test]
    fn an_uncompilable_expression_names_the_factor() {
        let error = FactorLibrary::from_expressions(
            "broken",
            &[("A", "CLOSE + ", FactorDirection::Neutral)],
        )
        .unwrap_err();
        let FactorError::Compute(message) = error else {
            panic!("expected a Compute error");
        };
        assert!(message.starts_with("broken/A:"), "message was {message}");
    }
}
