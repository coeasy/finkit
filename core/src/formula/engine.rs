use crate::formula::analysis::{analyze_formula, FormulaAnalysis, FormulaSeriesMetadata};
use crate::formula::ast::AstNode;
use crate::formula::bytecode::{compile_to_bytecode, Bytecode, BytecodeVM};
use crate::formula::compiler::{CompiledFormula, FormulaCache};
use crate::formula::compute_ir::FormulaComputePlan;
use crate::formula::custom::FormulaRegistry;
use crate::formula::debugger::FormulaDebugger;
use crate::formula::executor::FormulaExecutor;
use crate::formula::hot_plan::FormulaHotPlan;
use crate::formula::jit::JitCompiler;
#[cfg(feature = "formula-jit")]
use crate::formula::jit::OptimizedBytecode;
use crate::formula::optimizer::{DependencyAnalyzer, FormulaOptimizer};
use crate::formula::params::{apply_params, parse_params, validate_params, ParamDef, ParamValues};
use crate::formula::parser::parse_formula;
use crate::formula::pine::{map_pine_to_alphata_with_security, parse_pine, PineSecurityResolver};
use crate::formula::templates::{FormulaTemplate, FormulaTemplates};
use crate::formula::types::*;
use crate::formula::{
    normalize_formula_source, parse_formula_with_dialect, unified_formula_executor_with_host,
    FormulaDialect,
};
use crate::streaming::indicators::{StreamingRsi, StreamingSma};
use crate::streaming::StreamingIndicator;
use ndarray::Array1;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// Identity of a compiled formula plan.
///
/// All three components are load-bearing:
///
/// * `source` — the formula text as the caller wrote it, before normalisation.
/// * `dialect` — the same text parses to different ASTs per dialect (Pine goes
///   through `parse_pine` + `map_pine_to_alphata`), so a plan is only valid for
///   the dialect it was parsed under.
/// * `params` — [`apply_params`] rewrites parameter references into numeric
///   literals *before* the plan binds them into its parameter arena, so
///   `SMA(CLOSE,N)` with `N=14` and with `N=20` are genuinely different plans.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FormulaPlanKey {
    source: String,
    dialect: FormulaDialect,
    params: String,
}

impl FormulaPlanKey {
    fn new(source: &str, dialect: FormulaDialect, params: &ParamValues) -> Self {
        Self {
            source: source.to_string(),
            dialect,
            params: parameter_fingerprint(params),
        }
    }
}

/// Order-independent fingerprint of a parameter set.
///
/// [`ParamValues`] is a `HashMap`, whose iteration order is unspecified, so the
/// fingerprint sorts by name: two callers that pass the same parameters in a
/// different order must share one cached plan.
///
/// Values are rendered with `{:?}` rather than `{}` because a cache key must not
/// collide. Rust's float `Debug` prints the shortest representation that
/// round-trips, so it keeps `0.0` and `-0.0` distinct — they really do behave
/// differently — while folding every NaN into one key, which is right because
/// they do not.
fn parameter_fingerprint(params: &ParamValues) -> String {
    let mut entries: Vec<(&str, f64)> = params
        .iter()
        .map(|(name, &value)| (name.as_str(), value))
        .collect();
    // Names are unique by construction (`HashMap` keys), so this is a total
    // order and the result is deterministic.
    entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
    entries
        .into_iter()
        .map(|(name, value)| format!("{name}={value:?}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// Statistics for the compiled-plan cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FormulaPlanCacheStats {
    /// Lookups that returned an already-compiled plan.
    pub hits: u64,
    /// Lookups that had to compile.
    pub misses: u64,
}

/// Compiled-plan cache.
///
/// Unbounded, matching the engine's other caches (`semantic_plan_cache`,
/// `bytecode_cache`): a process evaluates a handful of distinct formulas, and a
/// bound here would evict plans that are about to be reused.
/// [`FormulaEngine::clear_plan_cache`] drops everything if that ever stops
/// being true.
///
/// Both counters live here, next to the lookup that decides them, so a call site
/// cannot bump the wrong one.
#[derive(Debug, Default)]
struct FormulaPlanCache {
    entries: HashMap<FormulaPlanKey, FormulaHotPlan>,
    hits: u64,
    misses: u64,
}

impl FormulaPlanCache {
    fn get(&mut self, key: &FormulaPlanKey) -> Option<FormulaHotPlan> {
        if let Some(plan) = self.entries.get(key) {
            self.hits = self.hits.saturating_add(1);
            Some(plan.clone())
        } else {
            self.misses = self.misses.saturating_add(1);
            None
        }
    }

    fn insert(&mut self, key: FormulaPlanKey, plan: FormulaHotPlan) {
        self.entries.insert(key, plan);
    }

    fn stats(&self) -> FormulaPlanCacheStats {
        FormulaPlanCacheStats {
            hits: self.hits,
            misses: self.misses,
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    /// Drop every plan but keep the counters.
    ///
    /// Registering a custom component can change what an existing source
    /// resolves to, so the plans must go; the counters still describe this
    /// engine's behaviour, so they stay.
    fn invalidate(&mut self) {
        self.entries.clear();
    }

    /// Drop every plan and reset the counters.
    fn clear(&mut self) {
        self.entries.clear();
        self.hits = 0;
        self.misses = 0;
    }
}

/// One compiled-plan evaluation: the primary series plus every named channel.
///
/// The primary is always the value of the formula's last value-producing
/// statement (see [`FormulaHotPlan::outputs`]); `channels` are the `OUTPUT:`
/// declarations, so a MACD script reports `DIF`/`DEA`/`MACD` rather than only
/// its last line.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FormulaPlanOutput {
    primary: Vec<f64>,
    channels: Vec<(String, Vec<f64>)>,
}

impl FormulaPlanOutput {
    /// The formula's primary result series.
    #[must_use]
    pub fn primary(&self) -> &[f64] {
        &self.primary
    }

    /// Named output channels in declaration order.
    #[must_use]
    pub fn channels(&self) -> &[(String, Vec<f64>)] {
        &self.channels
    }

    /// One named channel's series, if the formula declares it.
    #[must_use]
    pub fn channel(&self, name: &str) -> Option<&[f64]> {
        self.channels
            .iter()
            .find(|(channel, _)| channel == name)
            .map(|(_, values)| values.as_slice())
    }

    /// Consume this output and return its primary series.
    #[must_use]
    pub fn into_primary(self) -> Vec<f64> {
        self.primary
    }
}

#[derive(Debug, Clone)]
struct StreamingEmaState {
    period: usize,
    len: usize,
    seed_sum: f64,
    value: f64,
    valid: bool,
    previous_input: f64,
}

enum StreamingFormulaIndicator {
    Sma(StreamingSma),
    Rsi(StreamingRsi),
    Atr(StreamingSmaAtr),
}

/// Formula ATR follows the canonical batch Wilder seed: the first bar is used
/// only to establish the previous close, then TR[1..=period] seeds the RMA.
/// Keeping this state local avoids rebuilding the complete formula range while
/// preserving the public ATR warm-up and recurrence exactly.
struct StreamingSmaAtr {
    period: usize,
    atr_value: f64,
    tr_sum: f64,
    previous_close: f64,
    count: usize,
}

impl StreamingSmaAtr {
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
        self.count += 1;
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

impl StreamingFormulaIndicator {
    fn next(&mut self, input: [f64; 3]) -> Option<f64> {
        match self {
            Self::Sma(indicator) => indicator.next(input[0]),
            Self::Rsi(indicator) => indicator.next(input[0]),
            Self::Atr(indicator) => indicator.next(input[0], input[1], input[2]),
        }
    }
}

struct StreamingFormulaState {
    len: usize,
    last_input: [f64; 3],
    indicator: StreamingFormulaIndicator,
}

/// 公式引擎主入口
pub struct FormulaEngine {
    executor: FormulaExecutor,
    cache: FormulaCache,
    /// Semantic Compute IR plans keyed by the exact formula source.
    semantic_plan_cache: RefCell<HashMap<String, FormulaComputePlan>>,
    /// Compiled hot plans keyed by source + dialect + parameter fingerprint.
    ///
    /// Distinct from `semantic_plan_cache`, which is keyed by source alone and
    /// holds the *semantic* DAG: this one holds the *numeric* plan the unified
    /// executor runs, and it is dialect- and parameter-sensitive.
    plan_cache: RefCell<FormulaPlanCache>,
    templates: FormulaTemplates,
    jit_compiler: RefCell<JitCompiler>,
    /// Persistent bytecode cache and VM scratch buffers.
    bytecode_cache: RefCell<HashMap<String, Bytecode>>,
    bytecode_vm: RefCell<BytecodeVM>,
    /// Stateful fast paths for append/eval_last.  A failed continuity check
    /// simply falls back to the exact range evaluator.
    streaming_ema: RefCell<HashMap<String, StreamingEmaState>>,
    /// O(1) append paths for common direct formula indicators whose existing
    /// streaming implementations have exactly the same warm-up contract.
    streaming_common: RefCell<HashMap<String, StreamingFormulaState>>,
    /// User-defined expression components expanded before semantic planning.
    custom_formulas: FormulaRegistry,
}

impl Default for FormulaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl FormulaEngine {
    pub fn new() -> Self {
        Self {
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(100),
            semantic_plan_cache: RefCell::new(HashMap::new()),
            plan_cache: RefCell::new(FormulaPlanCache::default()),
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(HashMap::new()),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
            streaming_ema: RefCell::new(HashMap::new()),
            streaming_common: RefCell::new(HashMap::new()),
            custom_formulas: FormulaRegistry::new(),
        }
    }

    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
            semantic_plan_cache: RefCell::new(HashMap::new()),
            plan_cache: RefCell::new(FormulaPlanCache::default()),
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(HashMap::new()),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
            streaming_ema: RefCell::new(HashMap::new()),
            streaming_common: RefCell::new(HashMap::new()),
            custom_formulas: FormulaRegistry::new(),
        }
    }

    /// 编译公式字符串
    pub fn compile(&mut self, source: &str) -> Result<CompiledFormula, FormulaError> {
        if let Some(formula) = self.cache.get_cloned(source) {
            return Ok(formula);
        }

        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = self
            .custom_formulas
            .expand(&ast)
            .map_err(FormulaError::InvalidOperation)?;
        // Semantic analysis is deliberately performed before AST optimization.
        // This locks dependencies/effects against the source program so later
        // optimization and incremental execution cannot accidentally erase an
        // assignment, output, drawing command, or control-flow barrier.
        let semantic_plan = FormulaComputePlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!("formula compute planning failed: {error}"))
        })?;
        // Compile the optimized AST once so repeated evaluations share the
        // same CSE and constant-folding decisions while preserving assignment
        // side effects exposed through FormulaContext::variables.
        let ast = FormulaOptimizer::optimize_for_execution(&ast);
        let formula = CompiledFormula {
            ast,
            source: source.to_string(),
        };

        self.semantic_plan_cache
            .borrow_mut()
            .insert(source.to_string(), semantic_plan);
        self.cache.insert(source, formula.clone());

        Ok(formula)
    }

    /// Compile `source` into an executable hot plan, cached by
    /// `source + dialect + parameter fingerprint`.
    ///
    /// This is the compile half of P0-2 step 2: it makes the compiled plan
    /// reachable from the engine so the execution path can be switched onto it.
    /// The AST pipeline deliberately mirrors [`Self::eval_with_dialect`] (source
    /// normalisation, then a dialect parse) followed by [`Self::compile`]'s
    /// custom-component expansion, so the plan describes exactly what the tree
    /// path would have evaluated.
    ///
    /// # Errors
    ///
    /// A formula the plan path cannot compile returns [`FormulaError`]; this
    /// **never** silently falls back to the tree-walker. A caller that wants the
    /// tree path must ask for it explicitly, which is what makes
    /// `formula_execution_mode` a real switch rather than a hint.
    ///
    /// Failure is reported as [`FormulaError::InvalidOperation`] rather than as a
    /// new variant because `ffi/c-binding` matches `FormulaError` exhaustively to
    /// produce its error-code ABI; adding a variant would silently change that
    /// contract for every language binding. The message is prefixed so it stays
    /// distinguishable in logs.
    pub fn compile_plan(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
    ) -> Result<FormulaHotPlan, FormulaError> {
        let key = FormulaPlanKey::new(source, dialect, params);
        // Bind the lookup to a local so the `RefCell` borrow provably ends here,
        // rather than relying on `if let` scrutinee temporary lifetimes.
        let cached = self.plan_cache.borrow_mut().get(&key);
        if let Some(plan) = cached {
            return Ok(plan);
        }

        let plan = self.build_plan(source, dialect, params)?;
        self.plan_cache.borrow_mut().insert(key, plan.clone());
        Ok(plan)
    }

    /// [`Self::compile_plan`] with the default dialect and no parameters.
    ///
    /// # Errors
    ///
    /// As [`Self::compile_plan`].
    pub fn compile_plan_default(&self, source: &str) -> Result<FormulaHotPlan, FormulaError> {
        self.compile_plan(source, FormulaDialect::default(), &ParamValues::new())
    }

    /// Build a plan without consulting or populating the cache.
    fn build_plan(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
    ) -> Result<FormulaHotPlan, FormulaError> {
        let normalized = normalize_formula_source(source, dialect);
        let ast =
            parse_formula_with_dialect(&normalized, dialect).map_err(FormulaError::ParseError)?;
        let ast = self
            .custom_formulas
            .expand(&ast)
            .map_err(FormulaError::InvalidOperation)?;
        // Parameters are substituted *before* planning, not after: the plan
        // binds numeric literals into its parameter arena, so the substituted
        // values are precisely what make two parameterisations different plans.
        let ast = apply_params(&ast, params);
        FormulaHotPlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!("formula plan compilation failed: {error}"))
        })
    }

    /// Compiled-plan cache statistics.
    pub fn plan_cache_stats(&self) -> FormulaPlanCacheStats {
        self.plan_cache.borrow().stats()
    }

    /// Number of compiled plans currently cached.
    pub fn plan_cache_size(&self) -> usize {
        self.plan_cache.borrow().len()
    }

    /// Drop every cached plan and reset the cache statistics.
    pub fn clear_plan_cache(&mut self) {
        self.plan_cache.borrow_mut().clear();
    }

    /// Evaluate `source` through the compiled plan path, returning its primary series.
    ///
    /// This is the execution half of P0-2 step 3: it drives the same
    /// `UnifiedExecutor` the differential gate validates, using the cached plan
    /// from [`Self::compile_plan`].
    ///
    /// The tree path is untouched — this is an *additional* entry point, so
    /// nothing switches over until `formula_execution_mode` lands in step 4.
    ///
    /// # Errors
    ///
    /// Returns [`FormulaError`] if the formula cannot be parsed or planned (see
    /// [`Self::compile_plan`]), if the context does not supply every input series
    /// the plan declares, or if execution fails — most often because the plan
    /// contains an operator with no numeric kernel yet.
    pub fn eval_plan(
        &self,
        source: &str,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        Ok(Array1::from_vec(
            self.eval_plan_channels(source, FormulaDialect::default(), &ParamValues::new(), ctx)?
                .into_primary(),
        ))
    }

    /// [`Self::eval_plan`] for an explicit dialect.
    ///
    /// # Errors
    ///
    /// As [`Self::eval_plan`].
    pub fn eval_plan_with_dialect(
        &self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        Ok(Array1::from_vec(
            self.eval_plan_channels(source, dialect, &ParamValues::new(), ctx)?
                .into_primary(),
        ))
    }

    /// Evaluate `source` through the compiled plan path and return every channel.
    ///
    /// Unlike the tree path this takes `&FormulaContext`: the plan reads inputs and
    /// returns outputs, and never writes assignment results back into
    /// `ctx.variables`, so a shared borrow is all it needs.
    ///
    /// # Errors
    ///
    /// As [`Self::eval_plan`].
    pub fn eval_plan_channels(
        &self,
        source: &str,
        dialect: FormulaDialect,
        params: &ParamValues,
        ctx: &FormulaContext,
    ) -> Result<FormulaPlanOutput, FormulaError> {
        let plan = self.compile_plan(source, dialect, params)?;

        // Bind every numeric input slot the plan declared. A slot that no binding
        // fills is a hard error: substituting another series would convert an
        // input-layout bug into a silent numeric mismatch.
        let mut slots: Vec<Option<&[f64]>> = vec![None; plan.hot().input_layout().len()];
        for binding in plan.input_bindings() {
            let values = ctx.get_data(binding.name()).ok_or_else(|| {
                FormulaError::RuntimeError(format!(
                    "formula plan requires input series `{}`, which the context does not provide",
                    binding.name()
                ))
            })?;
            slots[binding.slot().0] = Some(values);
        }
        let mut inputs = Vec::with_capacity(slots.len());
        for (index, slot) in slots.into_iter().enumerate() {
            inputs.push(slot.ok_or_else(|| {
                FormulaError::RuntimeError(format!(
                    "formula plan input slot {index} was never bound"
                ))
            })?);
        }

        // Host data (chip distribution, chart period) cannot travel in a numeric
        // slot, so it is handed to the dispatcher separately. Without this,
        // `WINNER`/`COST`/`PERIODTYPE` would return NaN on the plan path while
        // the tree path returned real values -- a silent cross-path divergence.
        let mut executor = unified_formula_executor_with_host(
            &plan,
            HostContext {
                chip: ctx.chip_data.clone(),
                period_type: ctx.period_type,
            },
        );
        let output = executor.execute(&inputs).map_err(|error| {
            FormulaError::RuntimeError(format!("formula plan execution failed: {error}"))
        })?;

        // `ExecutionOutput::values` is ordered by retained output, so resolve each
        // named channel through the output layout instead of assuming the binding
        // order matches it.
        //
        // The primary and the *last* named output are usually the same retained
        // buffer — `DIF:...;DEA:...;MACD:...` reports `MACD` as the result — so
        // neither may simply be `take`n first: taking the channel would leave the
        // primary empty. `values` is therefore consumed through `Option`s, and a
        // channel whose slot was already taken clones the primary.
        let layout = plan.hot().output_layout().outputs();
        let mut values: Vec<Option<Vec<f64>>> = output.values.into_iter().map(Some).collect();
        let primary = values.first_mut().and_then(Option::take).ok_or_else(|| {
            FormulaError::RuntimeError("formula plan produced no output series".to_string())
        })?;
        let channels = plan
            .outputs()
            .iter()
            .filter_map(|binding| {
                let index = layout.iter().position(|(_, slot)| *slot == binding.slot())?;
                let series = values[index].take().unwrap_or_else(|| primary.clone());
                Some((binding.name().to_string(), series))
            })
            .collect();

        Ok(FormulaPlanOutput { primary, channels })
    }

    /// Register a reusable, parameterized expression component.
    ///
    /// Components are expanded before analysis and execution, so a formula
    /// such as `SIGNAL(CLOSE)` can be composed from built-ins without adding a
    /// new runtime function. Registration invalidates compiled plans because
    /// an existing source may now resolve a newly registered component.
    pub fn register_custom_formula(
        &mut self,
        name: &str,
        parameters: &[&str],
        source: &str,
    ) -> Result<(), FormulaError> {
        self.custom_formulas
            .register(name, parameters, source)
            .map_err(FormulaError::InvalidOperation)?;
        self.invalidate_formula_caches();
        Ok(())
    }

    /// Alias for [`Self::register_custom_formula`] for registry-oriented APIs.
    pub fn register_formula(
        &mut self,
        name: &str,
        parameters: &[&str],
        source: &str,
    ) -> Result<(), FormulaError> {
        self.register_custom_formula(name, parameters, source)
    }

    /// Remove one custom component and invalidate compiled plans if removed.
    pub fn unregister_custom_formula(&mut self, name: &str) -> Result<bool, FormulaError> {
        let removed = self
            .custom_formulas
            .unregister(name)
            .map_err(FormulaError::InvalidOperation)?;
        if removed {
            self.invalidate_formula_caches();
        }
        Ok(removed)
    }

    /// Remove all custom components and invalidate compiled plans.
    pub fn clear_custom_formulas(&mut self) {
        self.custom_formulas.clear();
        self.invalidate_formula_caches();
    }

    /// Return registered custom component names in deterministic order.
    pub fn custom_formula_names(&self) -> Vec<String> {
        self.custom_formulas.names()
    }

    /// Inspect the registry used by this engine.
    pub fn custom_formula_registry(&self) -> &FormulaRegistry {
        &self.custom_formulas
    }

    fn invalidate_formula_caches(&mut self) {
        self.cache.clear();
        self.semantic_plan_cache.borrow_mut().clear();
        // `invalidate`, not `clear`: the plans are stale because a custom
        // component changed, but the counters still describe this engine.
        self.plan_cache.borrow_mut().invalidate();
        self.bytecode_cache.borrow_mut().clear();
        self.streaming_ema.borrow_mut().clear();
        self.streaming_common.borrow_mut().clear();
    }

    /// 执行已编译的公式
    pub fn execute(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        // Fast-path the common single-indicator formulas before entering the
        // general AST executor. This avoids materialising input argument
        // arrays and lets the SIMD _into kernels write directly into one
        // result buffer. Complex formulas keep the existing semantics.
        let sandbox_unlimited = ctx.sandbox.timeout_ms.is_none()
            && ctx.sandbox.max_recursion_depth.is_none()
            && ctx.sandbox.max_memory_bytes.is_none();
        if sandbox_unlimited {
            if let Some(result) = self.try_execute_simple_formula(&formula.ast, ctx) {
                return Ok(result);
            }
        }
        self.executor.execute(&formula.ast, ctx)
    }

    /// Execute a simple built-in formula directly into a caller-owned buffer.
    fn try_execute_simple_formula_into(
        &self,
        ast: &AstNode,
        ctx: &FormulaContext,
        output: &mut Array1<f64>,
    ) -> bool {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return false,
        };

        let input = match &args[0] {
            AstNode::Variable(name) => ctx.get_data(name),
            _ => None,
        };
        let Some(input) = input else {
            return false;
        };
        if input.len() != output.len() {
            return false;
        }

        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return false,
        };
        let period = period_value as usize;
        if period == 0 {
            return false;
        }

        if input.iter().any(|value| !value.is_finite()) {
            output.fill(f64::NAN);
            return true;
        }

        match name.to_ascii_uppercase().as_str() {
            "MA" | "BOLLMID" => crate::math::simd_kernels::sma_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            "EMA" => crate::math::simd_kernels::ema_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            "RSI" => crate::math::simd_kernels::rsi_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            _ => return false,
        }
        true
    }

    /// Execute a simple built-in formula through the native SIMD kernel.
    ///
    /// This is deliberately conservative: only a single function call with a
    /// literal period is specialised, so formulas with assignments, nested
    /// expressions, or dynamic periods still use the general executor.
    fn try_execute_simple_formula(
        &self,
        ast: &AstNode,
        ctx: &FormulaContext,
    ) -> Option<Array1<f64>> {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return None,
        };

        let input = match &args[0] {
            AstNode::Variable(name) => ctx.get_data(name),
            _ => None,
        }?;

        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return None,
        };
        let period = period_value as usize;
        if period == 0 {
            return None;
        }

        // The formula functions intentionally turn invalid market data into
        // an all-NaN result. Preserve that behaviour while bypassing the
        // allocation-heavy function-call path for valid input.
        if input.iter().any(|value| !value.is_finite()) {
            return Some(Array1::from_elem(ctx.data_len, f64::NAN));
        }

        let mut output = Array1::from_elem(input.len(), f64::NAN);
        match name.to_ascii_uppercase().as_str() {
            "MA" | "BOLLMID" => {
                crate::math::simd_kernels::sma_simd_into(
                    input,
                    period,
                    output.as_slice_mut().expect("Array1 is contiguous"),
                );
            }
            "EMA" => {
                crate::math::simd_kernels::ema_simd_into(
                    input,
                    period,
                    output.as_slice_mut().expect("Array1 is contiguous"),
                );
            }
            "RSI" => {
                crate::math::simd_kernels::rsi_simd_into(
                    input,
                    period,
                    output.as_slice_mut().expect("Array1 is contiguous"),
                );
            }
            _ => return None,
        }

        Some(output)
    }

    /// Execute a compiled formula into a caller-provided output buffer.
    ///
    /// Reuse the same buffer and engine across calls to avoid allocating the
    /// final result on every evaluation. The formula's intermediate buffers
    /// are also recycled by the executor.
    pub fn eval_into(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
        output: &mut Array1<f64>,
    ) -> Result<(), FormulaError> {
        if output.len() != ctx.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "output length {} != ctx.data_len {}",
                output.len(),
                ctx.data_len
            )));
        }
        let sandbox_unlimited = ctx.sandbox.timeout_ms.is_none()
            && ctx.sandbox.max_recursion_depth.is_none()
            && ctx.sandbox.max_memory_bytes.is_none();
        if !sandbox_unlimited {
            let result = self.executor.execute(&formula.ast, ctx)?;
            output.assign(&result);
            return Ok(());
        }
        if self.try_execute_simple_formula_into(&formula.ast, ctx, output) {
            return Ok(());
        }
        self.executor.eval_into(&formula.ast, ctx, output)
    }

    /// Evaluate only the requested half-open output range.
    ///
    /// For formulas with a finite dependency lookback, the executor receives
    /// only the dependency window. Recursive/unknown functions conservatively
    /// use the full prefix to preserve exact historical semantics.
    pub fn eval_range(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
        start: usize,
        end: usize,
    ) -> Result<Array1<f64>, FormulaError> {
        if start > end || end > ctx.data_len {
            return Err(FormulaError::InvalidParameter(format!(
                "invalid eval_range [{start}, {end}) for data_len {}",
                ctx.data_len
            )));
        }
        if start == end {
            return Ok(Array1::zeros(0));
        }
        let cached_effects = {
            let cache = self.semantic_plan_cache.borrow();
            cache
                .get(&formula.source)
                .map(|plan| plan.plan().has_observable_effects())
        };
        let has_observable_effects = match cached_effects {
            Some(value) => value,
            None => {
                // CompiledFormula is public and can originate from another
                // FormulaCompiler, so rebuild semantic metadata defensively
                // when this engine did not create the object itself.
                let plan = FormulaComputePlan::compile(&formula.ast).map_err(|error| {
                    FormulaError::InvalidOperation(format!(
                        "formula compute planning failed: {error}"
                    ))
                })?;
                let value = plan.plan().has_observable_effects();
                self.semantic_plan_cache
                    .borrow_mut()
                    .insert(formula.source.clone(), plan);
                value
            }
        };
        let window_start = if has_observable_effects {
            // Effectful formulas may depend on assignments, outputs, drawing,
            // or control flow before the requested range. Preserve the full
            // prefix until a dedicated control-flow-aware incremental backend
            // can prove a smaller window is safe.
            0
        } else {
            FormulaOptimizer::required_lookback(&formula.ast)
                .map(|lookback| start.saturating_sub(lookback))
                .unwrap_or(0)
        };
        // The OHLCV arrays are borrowed for this synchronous call.  The old
        // owned window copied five full slices on every chart refresh; only
        // optional ndarray-backed metadata still needs a defensive copy.
        let mut window = ctx.borrowed_window(window_start, end)?;
        let result = self.execute(formula, &mut window)?;
        let local_start = start - window_start;
        Ok(result
            .slice(ndarray::s![local_start..(local_start + end - start)])
            .to_owned())
    }

    /// Evaluate a half-open range directly from borrowed contiguous OHLCV
    /// slices.  This is the public high-throughput range API used by bindings;
    /// it avoids copying the complete history before dependency trimming.
    pub fn eval_range_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        start: usize,
        end: usize,
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
            || start > end
            || end > close.len()
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy range inputs must be non-empty, aligned, and satisfy 0 <= start <= end <= len"
                    .to_string(),
            ));
        }
        let context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.eval_range(formula, &context, start, end)
    }

    /// Evaluate the last bar only.
    pub fn eval_last(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Result<f64, FormulaError> {
        if ctx.data_len == 0 {
            return Err(FormulaError::InvalidParameter(
                "cannot eval_last an empty context".to_string(),
            ));
        }
        if let Some(value) = self.try_eval_last_streaming_ema(formula, ctx) {
            return Ok(value);
        }
        if let Some(value) = self.try_eval_last_streaming_common(formula, ctx) {
            return Ok(value);
        }
        let result = self.eval_range(formula, ctx, ctx.data_len - 1, ctx.data_len)?;
        Ok(result[0])
    }

    /// O(1) EMA append path for a direct built-in formula.  This path is
    /// deliberately conservative: it is used only for a literal period and
    /// a direct OHLCV/variable input, and continuity is checked by length and
    /// the previous sample.  Any mismatch uses the exact batch evaluator.
    fn try_eval_last_streaming_ema(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Option<f64> {
        let (input_name, period) = match &formula.ast {
            AstNode::FunctionCall { name, args }
                if name.eq_ignore_ascii_case("EMA") && args.len() >= 2 =>
            {
                let AstNode::Variable(input_name) = &args[0] else {
                    return None;
                };
                let AstNode::Number(period) = args[1] else {
                    return None;
                };
                if !period.is_finite() || period < 1.0 || period.fract() != 0.0 {
                    return None;
                }
                (input_name.clone(), period as usize)
            }
            _ => return None,
        };
        let input = ctx.get_data(&input_name)?;
        if input.len() != ctx.data_len || input.is_empty() {
            self.streaming_ema.borrow_mut().remove(&formula.source);
            return None;
        }

        let mut states = self.streaming_ema.borrow_mut();
        let state_was_existing = states.contains_key(&formula.source);
        if !state_was_existing && input.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let state = states.entry(formula.source.clone()).or_insert_with(|| {
            let seed_sum = input.iter().sum::<f64>();
            if input.len() >= period {
                let initial_sum = input[input.len() - period..].iter().sum::<f64>();
                let value = if input.len() == period {
                    initial_sum / period as f64
                } else {
                    // A state created after a full evaluation must match the
                    // last batch output; calculate that one time only.
                    crate::math::moving_avg::ema(input, period)
                        .ok()
                        .and_then(|v| v.last().copied())
                        .unwrap_or(f64::NAN)
                };
                StreamingEmaState {
                    period,
                    len: input.len(),
                    seed_sum: initial_sum,
                    value,
                    valid: value.is_finite(),
                    previous_input: *input.last().unwrap(),
                }
            } else {
                StreamingEmaState {
                    period,
                    len: input.len(),
                    seed_sum,
                    value: f64::NAN,
                    valid: false,
                    previous_input: *input.last().unwrap(),
                }
            }
        });

        if state.period != period {
            *state = StreamingEmaState {
                period,
                len: 0,
                seed_sum: 0.0,
                value: f64::NAN,
                valid: false,
                previous_input: f64::NAN,
            };
        }

        if !state_was_existing {
            return Some(state.value);
        }
        // A same-length call may observe a caller mutation or a different
        // context.  Do not trust the cached state; invalidate and use the
        // exact evaluator.  Only a genuine one-bar append is O(1).
        if state.len == ctx.data_len {
            states.remove(&formula.source);
            return None;
        }
        if state.len + 1 != ctx.data_len
            || state.len == 0
            || state.previous_input != input[state.len - 1]
        {
            states.remove(&formula.source);
            return None;
        }

        let current = input[state.len];
        if !current.is_finite() {
            states.remove(&formula.source);
            return None;
        }
        state.len = ctx.data_len;
        state.previous_input = current;
        if !state.valid {
            state.seed_sum += current;
            if state.len >= period {
                state.value = state.seed_sum / period as f64;
                state.valid = true;
            } else {
                state.value = f64::NAN;
            }
        } else {
            let alpha = 2.0 / (period as f64 + 1.0);
            state.value = (current - state.value).mul_add(alpha, state.value);
        }
        Some(state.value)
    }

    /// O(1) append path for direct MA/RSI/ATR formula calls.  The first call
    /// seeds the indicator from the supplied history; subsequent calls are
    /// accepted only for a genuine one-bar append with an unchanged previous
    /// input.  Any mutation or discontinuity falls back to exact evaluation.
    fn try_eval_last_streaming_common(
        &self,
        formula: &CompiledFormula,
        ctx: &FormulaContext,
    ) -> Option<f64> {
        let (kind, input_names, period) = match &formula.ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => {
                let upper = name.to_ascii_uppercase();
                let period_index = match upper.as_str() {
                    "MA" | "RSI" => 1,
                    "ATR" => 3,
                    _ => return None,
                };
                let Some(AstNode::Number(period)) = args.get(period_index) else {
                    return None;
                };
                if !period.is_finite() || *period < 1.0 || period.fract() != 0.0 {
                    return None;
                }
                let required_inputs = if upper == "ATR" { 3 } else { 1 };
                let names = args
                    .get(..required_inputs)?
                    .iter()
                    .map(|arg| match arg {
                        AstNode::Variable(name) => Some(name.clone()),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                (upper, names, *period as usize)
            }
            _ => return None,
        };
        if ctx.data_len == 0
            || input_names.iter().any(|name| {
                ctx.get_data(name)
                    .is_none_or(|values| values.len() != ctx.data_len)
            })
        {
            self.streaming_common.borrow_mut().remove(&formula.source);
            return None;
        }

        let values = |index: usize| -> Option<[f64; 3]> {
            let mut input = [0.0; 3];
            for (slot, name) in input_names.iter().enumerate() {
                let series = ctx.get_data(name)?;
                input[slot] = *series.get(index)?;
            }
            if input[..input_names.len()]
                .iter()
                .any(|value| !value.is_finite())
            {
                None
            } else {
                Some(input)
            }
        };

        let mut states = self.streaming_common.borrow_mut();
        let state_was_existing = states.contains_key(&formula.source);
        if !state_was_existing {
            let mut indicator = match kind.as_str() {
                "MA" => StreamingFormulaIndicator::Sma(StreamingSma::new(period)),
                "RSI" => StreamingFormulaIndicator::Rsi(StreamingRsi::new(period)),
                "ATR" => StreamingFormulaIndicator::Atr(StreamingSmaAtr::new(period)),
                _ => return None,
            };
            let mut last = [f64::NAN; 3];
            let mut value = None;
            for index in 0..ctx.data_len {
                let input = values(index)?;
                last = input;
                value = indicator.next(input);
            }
            let value = value.unwrap_or(f64::NAN);
            states.insert(
                formula.source.clone(),
                StreamingFormulaState {
                    len: ctx.data_len,
                    last_input: last,
                    indicator,
                },
            );
            return Some(value);
        }

        let state = states.get_mut(&formula.source)?;
        if state.len == ctx.data_len {
            states.remove(&formula.source);
            return None;
        }
        if state.len + 1 != ctx.data_len {
            states.remove(&formula.source);
            return None;
        }
        let previous = values(state.len - 1)?;
        if previous != state.last_input {
            states.remove(&formula.source);
            return None;
        }
        let current = values(state.len)?;
        let value = state.indicator.next(current);
        state.len = ctx.data_len;
        state.last_input = current;
        Some(value.unwrap_or(f64::NAN))
    }

    /// Analyze a formula without executing it.
    pub fn analyze(&mut self, source: &str) -> Result<FormulaAnalysis, FormulaError> {
        let formula = self.compile(source)?;
        Ok(analyze_formula(&formula.ast))
    }

    /// Analyze an already parsed/compiled AST without executing it.
    pub fn analyze_ast(&self, ast: &AstNode) -> FormulaAnalysis {
        analyze_formula(ast)
    }

    /// Return the stable result-shape and warm-up contract for a formula.
    pub fn metadata(
        &mut self,
        source: &str,
        data_len: usize,
    ) -> Result<FormulaSeriesMetadata, FormulaError> {
        let analysis = self.analyze(source)?;
        Ok(analysis.result_metadata(data_len))
    }

    /// Return metadata for an already compiled formula.
    pub fn metadata_for_formula(
        &self,
        formula: &CompiledFormula,
        data_len: usize,
    ) -> FormulaSeriesMetadata {
        analyze_formula(&formula.ast).result_metadata(data_len)
    }

    /// Evaluate common NumPy-backed formulas directly from borrowed slices.
    ///
    /// The fast path never materialises an Array1 for the five OHLCV inputs.
    /// Complex formulas fall back to the regular executor, where intermediate
    /// arrays are required by the formula function ABI.
    pub fn eval_zero_copy_inputs(
        &self,
        formula: &CompiledFormula,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
        amount: Option<&[f64]>,
    ) -> Result<Array1<f64>, FormulaError> {
        if close.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != close.len())
            || amount.is_some_and(|values| values.len() != close.len())
        {
            return Err(FormulaError::InvalidParameter(
                "zero-copy formula inputs must be non-empty and have equal lengths".to_string(),
            ));
        }

        if let Some(result) =
            self.try_execute_simple_formula_slices(&formula.ast, open, high, low, close, volume)
        {
            return Ok(result);
        }

        let mut context = FormulaContext::from_borrowed_ohlcv(
            open,
            high,
            low,
            close,
            volume,
            amount.map(|values| Array1::from_vec(values.to_vec())),
        );
        self.execute(formula, &mut context)
    }

    fn try_execute_simple_formula_slices(
        &self,
        ast: &AstNode,
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
        volume: &[f64],
    ) -> Option<Array1<f64>> {
        let (name, args) = match ast {
            AstNode::FunctionCall { name, args } if args.len() >= 2 => (name.as_str(), args),
            _ => return None,
        };
        let input = match &args[0] {
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("C") || name.eq_ignore_ascii_case("CLOSE") =>
            {
                close
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("O") || name.eq_ignore_ascii_case("OPEN") =>
            {
                open
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("H") || name.eq_ignore_ascii_case("HIGH") =>
            {
                high
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("L") || name.eq_ignore_ascii_case("LOW") =>
            {
                low
            }
            AstNode::Variable(name)
                if name.eq_ignore_ascii_case("V")
                    || name.eq_ignore_ascii_case("VOL")
                    || name.eq_ignore_ascii_case("VOLUME") =>
            {
                volume
            }
            _ => return None,
        };
        if input.is_empty()
            || [open, high, low, close, volume]
                .iter()
                .any(|values| values.len() != input.len())
        {
            return None;
        }
        let period_value = match &args[1] {
            AstNode::Number(value) if value.is_finite() && *value > 0.0 => *value,
            _ => return None,
        };
        let period = period_value as usize;
        if period == 0 || (period as f64 - period_value).abs() > f64::EPSILON {
            return None;
        }
        if input.iter().any(|value| !value.is_finite()) {
            return Some(Array1::from_elem(input.len(), f64::NAN));
        }
        let mut output = Array1::from_elem(input.len(), f64::NAN);
        match name.to_ascii_uppercase().as_str() {
            "MA" | "BOLLMID" => crate::math::simd_kernels::sma_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            "EMA" => crate::math::simd_kernels::ema_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            "RSI" => crate::math::simd_kernels::rsi_simd_into(
                input,
                period,
                output.as_slice_mut().expect("Array1 is contiguous"),
            ),
            _ => return None,
        }
        Some(output)
    }

    /// 便捷方法：编译并执行
    #[cfg_attr(feature = "tracing", tracing::instrument(level = "debug", skip_all, fields(source_len = source.len())))]
    pub fn eval(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        self.execute(&formula, ctx)
    }

    /// Compile and evaluate a formula through the selected dialect contract.
    ///
    /// Domestic dialects share the canonical AlphaTA AST/runtime, while Pine
    /// is parsed and lowered by its dedicated subset mapper. Source
    /// normalization is performed once here so native, panel, and FFI callers
    /// cannot diverge on BOM or line-ending handling.
    pub fn eval_with_dialect(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let normalized = normalize_formula_source(source, dialect);
        match dialect {
            FormulaDialect::AlphaTA
            | FormulaDialect::TongDaXin
            | FormulaDialect::TongHuaShun
            | FormulaDialect::EastMoney => self.eval(&normalized, ctx),
            FormulaDialect::Pine => {
                let ast = parse_formula_with_dialect(&normalized, dialect)
                    .map_err(FormulaError::ParseError)?;
                self.eval_ast(&ast, ctx)
            }
        }
    }

    /// Evaluate a pre-built AST directly (no string parsing).
    ///
    /// This is the integration point for alternative dialects such as Pine
    /// Script v5: callers parse with [`parse_formula_with_dialect`] (which
    /// maps Pine → AlphaTA `AstNode`) and hand the resulting node here, so
    /// Pine indicators reuse the full AlphaTA execution pipeline (bytecode,
    /// JIT, SIMD, partial-eval).
    pub fn eval_ast(
        &self,
        ast: &AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.executor.execute(ast, ctx)
    }

    /// Partial-evaluation entry point (R-2).
    ///
    /// Attempts to evaluate the formula; on per-element runtime failures
    /// (e.g. divide-by-zero, log-of-negative, undefined variables in array
    /// positions) the result array is filled with `f64::NAN` at those
    /// indices and a vector of human-readable error messages is returned
    /// alongside the result. Callers can then decide whether to retry,
    /// patch the input, or surface the errors upstream.
    ///
    /// # Returns
    /// A `(result, errors)` tuple. `errors` is empty on a fully successful
    /// evaluation. The `result` is always a fully-shaped `Array1<f64>` —
    /// either the actual computed values, or NaN where evaluation failed.
    pub fn eval_partial(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> (Array1<f64>, Vec<String>) {
        match self.eval(source, ctx) {
            Ok(v) => (v, Vec::new()),
            Err(e) => {
                #[cfg(feature = "tracing")]
                crate::warn!(source, error = %e, "formula partial-eval failed");
                #[cfg(feature = "metrics")]
                crate::metrics::formula_error("partial");
                let err_msg = format!("{e}");
                let n = ctx.data_len;
                (Array1::from_elem(n, f64::NAN), vec![err_msg])
            }
        }
    }

    /// 多输出执行：返回所有 Output 变量及最终值
    pub fn eval_multi(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<MultiOutput, FormulaError> {
        let vars_before: std::collections::HashSet<Arc<str>> =
            ctx.variables.keys().cloned().collect();
        let final_value = self.eval(source, ctx)?;
        let mut multi = MultiOutput::new(final_value);
        for (name, value) in &ctx.variables {
            if !vars_before.contains(name) {
                multi.outputs.insert(name.to_string(), value.clone());
            }
        }
        Ok(multi)
    }

    /// Evaluate a formula with dialect-aware parsing and named outputs.
    ///
    /// This is the canonical multi-output entry point used by the operation
    /// engine and language-neutral bindings.
    pub fn eval_multi_with_dialect(
        &mut self,
        source: &str,
        dialect: FormulaDialect,
        ctx: &mut FormulaContext,
    ) -> Result<MultiOutput, FormulaError> {
        let normalized = normalize_formula_source(source, dialect);
        match dialect {
            FormulaDialect::AlphaTA
            | FormulaDialect::TongDaXin
            | FormulaDialect::TongHuaShun
            | FormulaDialect::EastMoney => self.eval_multi(&normalized, ctx),
            FormulaDialect::Pine => {
                let vars_before: std::collections::HashSet<Arc<str>> =
                    ctx.variables.keys().cloned().collect();
                let ast = parse_formula_with_dialect(&normalized, dialect)
                    .map_err(FormulaError::ParseError)?;
                let final_value = self.eval_ast(&ast, ctx)?;
                let mut multi = MultiOutput::new(final_value);
                for (name, value) in &ctx.variables {
                    if !vars_before.contains(name) {
                        multi.outputs.insert(name.to_string(), value.clone());
                    }
                }
                Ok(multi)
            }
        }
    }

    /// Evaluate Pine with an explicit host/provider-backed
    /// `request.security` resolver. The resolver must return an already
    /// timestamp-aligned series variable; this method does not resample or
    /// infer higher-timeframe values.
    pub fn eval_multi_with_pine_security(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        resolver: &dyn PineSecurityResolver,
    ) -> Result<MultiOutput, FormulaError> {
        let normalized = normalize_formula_source(source, FormulaDialect::Pine);
        let pine = parse_pine(&normalized)
            .map_err(|error| FormulaError::ParseError(format!("Pine parse error: {error}")))?;
        let ast = map_pine_to_alphata_with_security(&pine, Some(resolver))
            .map_err(|error| FormulaError::ParseError(format!("Pine map error: {error}")))?;
        let vars_before: std::collections::HashSet<Arc<str>> =
            ctx.variables.keys().cloned().collect();
        let final_value = self.eval_ast(&ast, ctx)?;
        let mut multi = MultiOutput::new(final_value);
        for (name, value) in &ctx.variables {
            if !vars_before.contains(name) {
                multi.outputs.insert(name.to_string(), value.clone());
            }
        }
        Ok(multi)
    }

    /// 惰性求值：通过依赖分析只计算最终输出所需的变量
    pub fn eval_lazy(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let pruned = DependencyAnalyzer::analyze_and_prune(&formula.ast);
        self.executor.execute(&pruned, ctx)
    }

    /// 增量计算：追加新 bar 后重算公式，利用编译缓存加速
    /// 返回完整结果（包含所有 bars）。
    /// 如果 ctx 已有之前的 variables，会先清除再重算。
    pub fn eval_incremental(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        ctx.variables.clear();
        ctx.output_names.clear();
        ctx.output_modifiers.clear();
        self.eval(source, ctx)
    }

    /// 并行计算：分析 AST 中的独立子表达式，并行求值无依赖的分支。
    /// 当 rayon feature 未启用时，退化为串行求值（结果一致）。
    pub fn eval_parallel(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        self.execute_parallel(&ast, ctx)
    }

    fn execute_parallel(
        &self,
        ast: &crate::formula::ast::AstNode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        use crate::formula::ast::AstNode;
        use crate::formula::result_statement_index;

        match ast {
            AstNode::Statements(stmts) => {
                // This path overwrites `last_result` once per dependency group,
                // which cannot express "the last *value-producing* statement".
                // When trailing drawing directives or level markers would change
                // the result, fall back to the serial executor — the same
                // no-rayon behaviour this path already documents.
                if result_statement_index(stmts) != stmts.len().checked_sub(1) {
                    return self.executor.execute(ast, ctx);
                }

                // Group statements into independent batches based on dependencies
                let groups = DependencyAnalyzer::group_independent_stmts(stmts);

                let mut last_result = Array1::zeros(ctx.data_len);
                for group in groups {
                    if group.len() == 1 {
                        last_result = self.executor.execute(&group[0], ctx)?;
                    } else {
                        #[cfg(feature = "rayon")]
                        {
                            use rayon::prelude::*;
                            let local_ctxs: Vec<_> =
                                (0..group.len()).map(|_| ctx.clone()).collect();
                            let results: Vec<_> = group
                                .par_iter()
                                .zip(local_ctxs.into_par_iter())
                                .map(|(stmt, mut local_ctx)| {
                                    let local_exec = FormulaExecutor::new();
                                    let result = local_exec.execute(stmt, &mut local_ctx);
                                    (
                                        result,
                                        local_ctx.variables,
                                        local_ctx.output_names,
                                        local_ctx.output_modifiers,
                                        local_ctx.draw_commands.into_inner(),
                                    )
                                })
                                .collect();
                            for (r, vars, output_names, mods, draws) in results {
                                last_result = r?;
                                ctx.variables.extend(vars);
                                for output in output_names {
                                    if !ctx.output_names.iter().any(|existing| existing == &output)
                                    {
                                        ctx.output_names.push(output);
                                    }
                                }
                                ctx.output_modifiers.extend(mods);
                                ctx.draw_commands
                                    .borrow_mut()
                                    .commands
                                    .extend(draws.commands);
                            }
                        }
                        #[cfg(not(feature = "rayon"))]
                        {
                            for stmt in &group {
                                last_result = self.executor.execute(stmt, ctx)?;
                            }
                        }
                    }
                }
                Ok(last_result)
            }
            other => self.executor.execute(other, ctx),
        }
    }

    /// 带参数执行
    pub fn eval_with_params(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        params: &ParamValues,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let ast_with_params = apply_params(&formula.ast, params);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// 获取公式参数定义
    pub fn get_param_defs(&self, formula: &CompiledFormula) -> Result<Vec<ParamDef>, FormulaError> {
        parse_params(&formula.ast)
    }

    /// 验证参数并执行
    pub fn eval_with_validation(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
        params: &ParamValues,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let param_defs = parse_params(&formula.ast)?;
        validate_params(&param_defs, params)?;
        let ast_with_params = apply_params(&formula.ast, params);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// 使用默认参数执行
    pub fn eval_with_defaults(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let param_defs = parse_params(&formula.ast)?;
        let defaults: ParamValues = param_defs
            .iter()
            .map(|p| (p.name.clone(), p.default))
            .collect();
        let ast_with_params = apply_params(&formula.ast, &defaults);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// Batch evaluation: compute multiple formulas in a single pass.
    /// Shares the same context across all formulas, reducing data traversal overhead.
    pub fn eval_batch(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        let analyses: Vec<_> = formulas
            .iter()
            .map(|source| self.analyze(source))
            .collect::<Result<_, _>>()?;
        if analyses
            .iter()
            .all(|analysis| !analysis.has_observable_effects)
        {
            return self.eval_batch_shared_compiled(formulas, ctx);
        }
        let mut results: Vec<Option<Array1<f64>>> = vec![None; formulas.len()];
        let mut completed: HashMap<String, Array1<f64>> = HashMap::new();
        for (index, &source) in formulas.iter().enumerate() {
            if let Some(cached) = completed.get(source) {
                results[index] = Some(cached.clone());
                continue;
            }
            let result = self.eval(source, ctx)?;
            // Reusing a result is only semantics-preserving for formulas that
            // do not expose assignments, outputs or drawing side effects.
            completed.insert(source.to_string(), result.clone());
            results[index] = Some(result);
        }
        Ok(results
            .into_iter()
            .map(|result| result.expect("every batch formula is evaluated"))
            .collect())
    }

    /// Evaluate independent formulas as one statement graph.
    ///
    /// Pure formulas are wrapped as named outputs and executed in one pooled
    /// pass.  This gives the optimizer a single graph in which common
    /// subexpressions such as `EMA(CLOSE, 20)` can be reused.  Formulas with
    /// assignments, drawing or other observable effects deliberately retain
    /// sequential semantics and use the regular batch path.
    pub fn eval_batch_shared(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        self.eval_batch(formulas, ctx)
    }

    fn eval_batch_shared_compiled(
        &mut self,
        formulas: &[&str],
        ctx: &mut FormulaContext,
    ) -> Result<Vec<Array1<f64>>, FormulaError> {
        if formulas.is_empty() {
            return Ok(Vec::new());
        }
        let mut statements = Vec::with_capacity(formulas.len());
        let mut names = Vec::with_capacity(formulas.len());
        for (index, source) in formulas.iter().enumerate() {
            let formula = self.compile(source)?;
            let analysis = analyze_formula(&formula.ast);
            if analysis.has_observable_effects {
                return formulas
                    .iter()
                    .map(|source| self.eval(source, ctx))
                    .collect();
            }
            let name = format!("__FINKIT_BATCH_{index}");
            names.push(name.clone());
            statements.push(AstNode::Output {
                name,
                expr: Box::new(formula.ast),
                modifier: None,
            });
        }
        let combined = FormulaOptimizer::optimize_for_execution(&AstNode::Statements(statements));
        self.executor.execute(&combined, ctx)?;
        names
            .iter()
            .map(|name| {
                ctx.variables.get(name.as_str()).cloned().ok_or_else(|| {
                    FormulaError::RuntimeError(format!("shared batch output `{name}` missing"))
                })
            })
            .collect()
    }

    /// 缓存相关方法
    pub fn cache_hit(&self, source: &str) -> bool {
        self.cache.contains(source)
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.streaming_ema.borrow_mut().clear();
        self.streaming_common.borrow_mut().clear();
    }

    pub fn compile_bytecode(&mut self, source: &str) -> Result<Bytecode, FormulaError> {
        if let Some(bytecode) = self.bytecode_cache.borrow().get(source).cloned() {
            return Ok(bytecode);
        }
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = FormulaOptimizer::optimize(&ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        self.bytecode_cache
            .borrow_mut()
            .insert(source.to_string(), bytecode.clone());
        Ok(bytecode)
    }

    /// Execute bytecode with the VM owned by this engine.
    ///
    /// Keeping the VM alive lets its stack, variable map, and hash tables
    /// retain capacity between calls instead of reallocating every time.
    pub fn execute_bytecode(
        &self,
        bytecode: &Bytecode,
        ctx: &FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let mut vm = self.bytecode_vm.borrow_mut();
        let exec_result = vm.execute(bytecode, ctx)?;
        Ok(exec_result.final_value)
    }

    pub fn eval_optimized(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let optimized = FormulaOptimizer::optimize(&ast);
        self.executor.execute(&optimized, ctx)
    }

    pub fn eval_with_debug(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<(Array1<f64>, FormulaDebugger), FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let mut debugger = FormulaDebugger::new();
        debugger.enable_trace();
        let result = debugger.run_with_debug(&ast, ctx, &self.executor)?;
        Ok((result, debugger))
    }

    pub fn get_template(&self, name: &str) -> Option<&FormulaTemplate> {
        self.templates.get(name)
    }

    pub fn search_templates(&self, keyword: &str) -> Vec<&FormulaTemplate> {
        self.templates.search(keyword)
    }

    pub fn eval_template(
        &mut self,
        name: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let tmpl = self
            .templates
            .get(name)
            .ok_or_else(|| FormulaError::RuntimeError(format!("Template not found: {}", name)))?;
        let source = tmpl.source.clone();
        let param_defaults: Vec<(String, f64)> = tmpl
            .parameters
            .iter()
            .map(|(n, _min, _max, default)| (n.clone(), *default))
            .collect();
        let formula = self.compile(&source)?;
        let defaults: ParamValues = param_defaults.into_iter().collect();
        let ast_with_params = apply_params(&formula.ast, &defaults);
        self.executor.execute(&ast_with_params, ctx)
    }

    /// Evaluate through the **frozen** experimental bytecode path.
    ///
    /// See `formula::jit` for the freeze rules: opt-in only, never on the
    /// default path, compared against the tree path by the differential gate.
    /// Prefer [`Self::eval`] or the `plan` path for anything new.
    #[cfg(feature = "formula-jit")]
    pub fn eval_jit(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let ast = FormulaOptimizer::optimize(&formula.ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        let optimized = jit.compile_cached(bytecode);
        jit.execute(&optimized, ctx).map(|r| r.final_value)
    }

    /// **Frozen** compatibility alias — this is `eval`, verbatim.
    ///
    /// There is no SIMD formula-evaluation path: the body delegates to
    /// [`Self::eval`]. It exists because four language bindings export
    /// `formula_eval_simd`, and removing it would be a cross-language breaking
    /// change. SIMD is real at the *kernel* level (`math::simd_kernels`, reached
    /// from the normal path) — not here.
    ///
    /// Freeze rule: keep it an exact alias. If it ever needs different results
    /// from `eval`, that is a new feature and belongs in a new entry point.
    #[cfg(feature = "formula-simd")]
    pub fn eval_simd(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.eval(source, ctx)
    }

    /// Execute an already compiled formula through the persistent pooled path.
    pub fn execute_zero_copy_cached(
        &self,
        formula: &CompiledFormula,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        self.executor.execute_zero_copy_cached(&formula.ast, ctx)
    }

    pub fn eval_zero_copy(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        self.executor.execute_zero_copy(&formula.ast, ctx)
    }

    /// 使用 VarNameCache 的零拷贝执行路径，避免重复创建 Arc<str>
    pub fn eval_zero_copy_cached(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        self.executor.execute_zero_copy_cached(&formula.ast, ctx)
    }

    pub fn eval_zero_alloc(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let formula = self.compile(source)?;
        let val = self.executor.execute_val(&formula.ast, ctx)?;
        Ok(val.to_array(ctx.data_len))
    }

    /// Compile into the **frozen** experimental bytecode representation.
    ///
    /// See [`Self::eval_jit`] and `formula::jit`; opt-in only, no new callers.
    #[cfg(feature = "formula-jit")]
    pub fn compile_jit(&mut self, source: &str) -> Result<OptimizedBytecode, FormulaError> {
        let formula = self.compile(source)?;
        let ast = FormulaOptimizer::optimize(&formula.ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        Ok(jit.compile_cached(bytecode))
    }

    /// Execute an already-compiled **frozen** bytecode program.
    ///
    /// See [`Self::eval_jit`] and `formula::jit`; opt-in only, no new callers.
    #[cfg(feature = "formula-jit")]
    pub fn execute_jit(
        &self,
        optimized: &OptimizedBytecode,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let jit = self.jit_compiler.borrow();
        jit.execute(optimized, ctx).map(|r| r.final_value)
    }
}

/// 公式执行结果
pub struct FormulaResult {
    /// 输出变量及其值
    pub outputs: HashMap<String, Array1<f64>>,
    /// 最后一个表达式的值
    pub final_value: Array1<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx(len: usize) -> FormulaContext {
        let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
        let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
        let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
        let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
        let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
        FormulaContext::new(open, high, low, close, volume, None)
    }

    #[test]
    fn test_engine_eval_errors_return_err_not_panic() {
        // T3 regression guard: user-reachable formula errors (syntax, unknown
        // function, arity mismatch) must surface as `Err(FormulaError)` through
        // `eval` — never as a panic that an FFI `catch_unwind` guard (A3) would
        // silently swallow into a null result with the error lost.
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let cases = [
            "SMA(CLOSE",         // unterminated syntax error
            "1 +",               // incomplete expression
            ")",                 // stray token
            "FOOBAR(CLOSE, 20)", // unknown function
            "MA(CLOSE)",         // too few arguments
        ];
        for src in cases {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                engine.eval(src, &mut ctx)
            }));
            match res {
                Ok(r) => assert!(
                    r.is_err(),
                    "bad formula '{}' should return Err, got Ok",
                    src
                ),
                Err(_) => panic!(
                    "T3 regression: bad formula '{}' PANICKED instead of returning Err",
                    src
                ),
            }
        }
    }

    #[test]
    fn test_simple_formula_dispatch_matches_general_executor() {
        for source in [
            "MA(CLOSE, 20)",
            "EMA(CLOSE, 12)",
            "RSI(CLOSE, 14)",
            "BOLLMID(CLOSE, 20)",
        ] {
            let mut fast_engine = FormulaEngine::new();
            let formula = fast_engine.compile(source).unwrap();
            let mut fast_ctx = make_ctx(128);
            let fast = fast_engine.execute(&formula, &mut fast_ctx).unwrap();

            let ast = parse_formula(source).unwrap();
            let executor = FormulaExecutor::new();
            let mut reference_ctx = make_ctx(128);
            let reference = executor.execute(&ast, &mut reference_ctx).unwrap();

            assert_eq!(fast.len(), reference.len(), "{source}");
            for (actual, expected) in fast.iter().zip(reference.iter()) {
                assert!(
                    (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() < 1e-12,
                    "{source}: {actual} != {expected}"
                );
            }
        }
    }

    #[test]
    fn test_simple_formula_dispatch_writes_reusable_output() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("RSI(CLOSE, 14)").unwrap();
        let mut ctx = make_ctx(64);
        let mut output = Array1::zeros(64);
        engine.eval_into(&formula, &mut ctx, &mut output).unwrap();

        let reference = FormulaExecutor::new();
        let ast = parse_formula("RSI(CLOSE, 14)").unwrap();
        let mut reference_ctx = make_ctx(64);
        let expected = reference.execute(&ast, &mut reference_ctx).unwrap();

        for (actual, expected) in output.iter().zip(expected.iter()) {
            assert!(
                (actual.is_nan() && expected.is_nan()) || (actual - expected).abs() < 1e-12,
                "{actual} != {expected}"
            );
        }
    }

    #[test]
    fn test_simple_formula_dispatch_falls_back_for_nested_expression() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(32);
        let result = engine.eval("MA(CLOSE, 20) + 1", &mut ctx).unwrap();
        assert!(result.iter().skip(19).all(|value| value.is_finite()));
    }

    #[test]
    fn test_engine_new() {
        let engine = FormulaEngine::new();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_with_cache_size() {
        let engine = FormulaEngine::with_cache_size(50);
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_compile_and_execute() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let formula = engine.compile("10 + 20").unwrap();
        let result = engine.execute(&formula, &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_engine_compile_caching() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);

        engine.eval("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert!(engine.cache_hit("MA(CLOSE, 5)"));

        engine.eval("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(engine.cache_size(), 1);
    }

    #[test]
    fn test_engine_eval_with_params() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 10.0);

        let result = engine
            .eval_with_params("MA(CLOSE, N)", &mut ctx, &params)
            .unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_engine_get_param_defs() {
        let mut engine = FormulaEngine::new();
        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let formula = engine.compile(source).unwrap();
        let param_defs = engine.get_param_defs(&formula).unwrap();
        assert_eq!(param_defs.len(), 1);
        assert_eq!(param_defs[0].name, "N");
        assert_eq!(param_defs[0].min, 1.0);
        assert_eq!(param_defs[0].max, 100.0);
        assert_eq!(param_defs[0].default, 20.0);
    }

    #[test]
    fn test_engine_eval_with_validation_valid() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 50.0);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_validation(source, &mut ctx, &params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_engine_eval_with_validation_invalid() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let mut params = ParamValues::new();
        params.insert("N".to_string(), 150.0);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_validation(source, &mut ctx, &params);
        assert!(result.is_err());
    }

    #[test]
    fn test_engine_eval_with_defaults() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);

        let source = "PARAMS: N(1, 100, 20); MA5: MA(CLOSE, N)";
        let result = engine.eval_with_defaults(source, &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_engine_clear_cache() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        engine.eval("CLOSE + 1", &mut ctx).unwrap();
        assert_eq!(engine.cache_size(), 1);
        engine.clear_cache();
        assert_eq!(engine.cache_size(), 0);
    }

    #[test]
    fn test_engine_eval_batch() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let formulas = vec!["MA(CLOSE, 5)", "MA(CLOSE, 10)", "CLOSE + OPEN"];
        let results = engine.eval_batch(&formulas, &mut ctx).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].len(), 30);
        assert_eq!(results[1].len(), 30);
        assert_eq!(results[2].len(), 30);
    }

    #[test]
    fn test_engine_eval_batch_shared_matches_individual_formulas() {
        let formulas = [
            "EMA(CLOSE, 5)",
            "EMA(CLOSE, 5) + MA(CLOSE, 3)",
            "RSI(CLOSE, 5)",
        ];
        let mut shared_engine = FormulaEngine::new();
        let mut shared_ctx = make_ctx(40);
        let shared = shared_engine
            .eval_batch_shared(&formulas, &mut shared_ctx)
            .unwrap();
        let mut individual_engine = FormulaEngine::new();
        let mut individual_ctx = make_ctx(40);
        let individual = formulas
            .iter()
            .map(|source| individual_engine.eval(source, &mut individual_ctx).unwrap())
            .collect::<Vec<_>>();
        for (actual, expected) in shared.iter().zip(individual.iter()) {
            assert!(actual
                .iter()
                .zip(expected.iter())
                .all(|(a, b)| { (a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()) }));
        }
    }

    #[test]
    fn test_engine_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval("CLOSE +", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_compile_bytecode_simple() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("10 + 20").unwrap();
        assert!(!bytecode.instructions.is_empty());
    }

    #[test]
    fn test_compile_bytecode_with_function() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("MA(CLOSE, 5)").unwrap();
        assert!(bytecode.instructions.len() >= 2);
    }

    #[test]
    fn test_execute_bytecode_constant() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("42").unwrap();
        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_bytecode_expression() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("CLOSE + OPEN").unwrap();
        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_constant_folding() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_optimized("1 + 2 + 3", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 6.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_optimized("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_eval_with_debug_basic() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let (result, debugger) = engine.eval_with_debug("CLOSE + 1", &mut ctx).unwrap();
        assert_eq!(result.len(), 5);
        assert!(!debugger.get_events().is_empty());
    }

    #[test]
    fn test_eval_with_debug_complex() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let (result, debugger) = engine
            .eval_with_debug("MA5 := MA(CLOSE, 5); MA5 > 10", &mut ctx)
            .unwrap();
        assert_eq!(result.len(), 30);
        let events = debugger.get_events();
        assert!(events.len() > 2);
    }

    #[test]
    fn test_get_template_exists() {
        let engine = FormulaEngine::new();
        let tmpl = engine.get_template("ma_cross");
        assert!(tmpl.is_some());
        assert_eq!(tmpl.unwrap().name, "均线金叉死叉");
    }

    #[test]
    fn test_get_template_not_exists() {
        let engine = FormulaEngine::new();
        assert!(engine.get_template("nonexistent_template").is_none());
    }

    #[test]
    fn test_search_templates_by_keyword() {
        let engine = FormulaEngine::new();
        let results = engine.search_templates("MACD");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_templates_empty() {
        let engine = FormulaEngine::new();
        let results = engine.search_templates("zzzzznotfound");
        assert!(results.is_empty());
    }

    #[test]
    fn test_eval_template_existing() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_template("kdj_overbought", &mut ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 30);
    }

    #[test]
    fn test_eval_template_nonexistent() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_template("nonexistent", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_template_ma_cross() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(50);
        let result = engine.eval_template("ma_cross", &mut ctx).unwrap();
        assert_eq!(result.len(), 50);
    }

    #[test]
    fn test_bytecode_roundtrip_compile_execute() {
        let mut engine = FormulaEngine::new();
        let ctx = make_ctx(30);

        let bytecode = engine.compile_bytecode("CLOSE > OPEN").unwrap();
        let result = engine.execute_bytecode(&bytecode, &ctx).unwrap();

        for i in 0..30 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            let expected = if close_val > open_val { 1.0 } else { 0.0 };
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_optimized_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);

        let result1 = engine.eval("MA(CLOSE, 10)", &mut ctx1).unwrap();
        let result2 = engine.eval_optimized("MA(CLOSE, 10)", &mut ctx2).unwrap();

        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_compile_bytecode_complex_formula() {
        let mut engine = FormulaEngine::new();
        let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); MA5 > MA10";
        let bytecode = engine.compile_bytecode(source).unwrap();
        assert!(bytecode.instructions.len() > 10);
    }

    #[test]
    fn test_execute_bytecode_after_serialize() {
        let mut engine = FormulaEngine::new();
        let bytecode = engine.compile_bytecode("CLOSE * 2").unwrap();
        let data = bytecode.serialize();
        let restored = Bytecode::deserialize(&data).expect("Deserialize failed");

        let ctx = make_ctx(5);
        let result = engine.execute_bytecode(&restored, &ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_jit_with_function() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(30);
        let result = engine.eval_jit("MA(CLOSE, 5)", &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_eval_jit_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_jit("CLOSE +", &mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_eval_jit_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 10)", &mut ctx1).unwrap();
        let result2 = engine.eval_jit("MA(CLOSE, 10)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_eval_simd_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_simd("10 + 20", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 30.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_simd_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_simd("CLOSE * 2", &mut ctx).unwrap();
        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result[i] - expected).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_simd_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 5)", &mut ctx1).unwrap();
        let result2 = engine.eval_simd("MA(CLOSE, 5)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_eval_zero_copy_constant() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_zero_copy("42", &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_zero_copy_with_variables() {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(5);
        let result = engine.eval_zero_copy("CLOSE + OPEN", &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_eval_zero_copy_matches_eval() {
        let mut engine = FormulaEngine::new();
        let mut ctx1 = make_ctx(30);
        let mut ctx2 = make_ctx(30);
        let result1 = engine.eval("MA(CLOSE, 5)", &mut ctx1).unwrap();
        let result2 = engine.eval_zero_copy("MA(CLOSE, 5)", &mut ctx2).unwrap();
        for i in 0..30 {
            if result1[i].is_nan() {
                assert!(result2[i].is_nan());
            } else {
                assert!((result1[i] - result2[i]).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_compile_jit_simple() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("10 + 20").unwrap();
        assert!(optimized.buffer_size() >= 2);
        assert!(!optimized.is_hot());
        assert_eq!(optimized.source(), "10 + 20");
    }

    #[test]
    fn test_compile_jit_with_function() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("MA(CLOSE, 5)").unwrap();
        assert!(optimized.cached_call_count() > 0);
    }

    #[test]
    fn test_compile_jit_invalid_formula() {
        let mut engine = FormulaEngine::new();
        let result = engine.compile_jit("CLOSE +");
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_jit_constant() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("42").unwrap();
        let mut ctx = make_ctx(5);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        for i in 0..5 {
            assert!((result[i] - 42.0).abs() < 1e-10);
        }
    }

    #[test]
    fn test_execute_jit_expression() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("CLOSE + OPEN").unwrap();
        let mut ctx = make_ctx(5);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        for i in 0..5 {
            let close_val = 10.0 + i as f64 * 0.15;
            let open_val = 10.0 + i as f64 * 0.1;
            assert!((result[i] - (close_val + open_val)).abs() < 1e-10);
        }
    }

    #[test]
    fn test_compile_jit_then_execute_jit() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("MA(CLOSE, 5)").unwrap();
        let mut ctx = make_ctx(30);
        let result = engine.execute_jit(&optimized, &mut ctx).unwrap();
        assert_eq!(result.len(), 30);
    }

    #[test]
    fn test_compile_jit_reuse_multiple_executions() {
        let mut engine = FormulaEngine::new();
        let optimized = engine.compile_jit("CLOSE * 2").unwrap();

        let mut ctx1 = make_ctx(5);
        let result1 = engine.execute_jit(&optimized, &mut ctx1).unwrap();

        let mut ctx2 = make_ctx(10);
        let result2 = engine.execute_jit(&optimized, &mut ctx2).unwrap();

        for i in 0..5 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result1[i] - expected).abs() < 1e-10);
        }
        for i in 0..10 {
            let expected = (10.0 + i as f64 * 0.15) * 2.0;
            assert!((result2[i] - expected).abs() < 1e-10);
        }
    }
    #[test]
    fn test_eval_range_and_last_match_full_evaluation() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("MA(CLOSE, 5)").unwrap();
        let mut full_ctx = make_ctx(40);
        let full = engine.execute(&formula, &mut full_ctx).unwrap();

        let range_ctx = make_ctx(40);
        let range = engine.eval_range(&formula, &range_ctx, 10, 25).unwrap();
        assert_eq!(range.len(), 15);
        for (offset, value) in range.iter().enumerate() {
            assert!((*value - full[10 + offset]).abs() < 1e-12);
        }

        let last_ctx = make_ctx(40);
        let last = engine.eval_last(&formula, &last_ctx).unwrap();
        assert!((last - full[39]).abs() < 1e-12);
    }

    #[test]
    fn test_streaming_ema_eval_last_matches_append_full_recompute() {
        let mut engine = FormulaEngine::new();
        let formula = engine.compile("EMA(CLOSE, 5)").unwrap();
        let mut ctx = make_ctx(20);
        let first = engine.eval_last(&formula, &ctx).unwrap();
        let expected_first = crate::math::moving_avg::ema(&ctx.close, 5).unwrap();
        assert!((first - expected_first[19]).abs() < 1e-12);

        ctx.append_bar(12.0, 13.0, 11.0, 13.25, 2000.0);
        let streamed = engine.eval_last(&formula, &ctx).unwrap();
        let expected = crate::math::moving_avg::ema(&ctx.close, 5).unwrap();
        assert!((streamed - expected[20]).abs() < 1e-12);
        assert!((engine.eval_last(&formula, &ctx).unwrap() - streamed).abs() < 1e-12);
    }

    #[test]
    fn test_streaming_common_formula_paths_match_batch() {
        for source in ["MA(CLOSE, 5)", "RSI(CLOSE, 5)", "ATR(HIGH, LOW, CLOSE, 5)"] {
            let mut engine = FormulaEngine::new();
            let formula = engine.compile(source).unwrap();
            let mut ctx = make_ctx(20);
            let _first = engine.eval_last(&formula, &ctx).unwrap();

            ctx.append_bar(12.0, 13.0, 11.0, 13.25, 2000.0);
            let streamed = engine.eval_last(&formula, &ctx).unwrap();

            let mut expected_ctx = make_ctx(21);
            expected_ctx.close[20] = 13.25;
            expected_ctx.high[20] = 13.0;
            expected_ctx.low[20] = 11.0;
            expected_ctx.open[20] = 12.0;
            expected_ctx.volume[20] = 2000.0;
            let expected = FormulaEngine::new()
                .eval(source, &mut expected_ctx)
                .unwrap()[20];
            if expected.is_nan() {
                assert!(streamed.is_nan(), "{source} streamed value should be NaN");
            } else {
                assert!(
                    (streamed - expected).abs() < 1e-10,
                    "{source}: streamed={streamed} expected={expected}"
                );
            }
        }
    }

    #[test]
    fn test_formula_analysis_is_available_from_engine() {
        let mut engine = FormulaEngine::new();
        let report = engine.analyze("MA5:=MA(CLOSE,5); MA5 + OPEN").unwrap();
        assert_eq!(report.required_lookback, Some(4));
        assert!(report.input_variables.contains(&"CLOSE".to_string()));
        assert!(report.input_variables.contains(&"OPEN".to_string()));
        assert!(report.assigned_variables.contains(&"MA5".to_string()));
    }

    #[test]
    fn test_formula_metadata_is_stable_for_bindings() {
        let mut engine = FormulaEngine::new();
        let metadata = engine.metadata("MA5:=MA(CLOSE,5); MA5", 20).unwrap();
        assert_eq!(metadata.schema_version, "finkit.formula-series.v1");
        assert_eq!(metadata.length, 20);
        assert_eq!(metadata.output_names, vec!["MA5", "__result__"]);
        assert_eq!(metadata.valid_start, Some(4));
        assert_eq!(metadata.null_policy, "nan");
    }

    #[test]
    fn test_borrowed_slice_fast_path_matches_owned_context() {
        let ctx = make_ctx(32);
        let formula = FormulaEngine::new();
        let compiled = {
            let mut compiler = FormulaEngine::new();
            compiler.compile("MA(CLOSE, 5)").unwrap()
        };
        let borrowed = formula
            .eval_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                None,
            )
            .unwrap();
        let mut owned_ctx = make_ctx(32);
        let mut engine = FormulaEngine::new();
        let owned_formula = engine.compile("MA(CLOSE, 5)").unwrap();
        let owned = engine.execute(&owned_formula, &mut owned_ctx).unwrap();
        assert_eq!(borrowed.len(), owned.len());
        for (a, b) in borrowed.iter().zip(owned.iter()) {
            assert!((a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()));
        }
    }

    #[test]
    fn custom_components_are_compiled_and_cached_as_canonical_formulas() {
        let mut engine = FormulaEngine::new();
        engine
            .register_custom_formula("ZMA", &["X", "N"], "MA(X, N) + EMA(X, N)")
            .unwrap();
        let mut ctx = make_ctx(32);
        let composed = engine.eval("zma(CLOSE, 5)", &mut ctx).unwrap();

        let mut baseline_ctx = make_ctx(32);
        let baseline = FormulaEngine::new()
            .eval("MA(CLOSE, 5) + EMA(CLOSE, 5)", &mut baseline_ctx)
            .unwrap();
        for (actual, expected) in composed.iter().zip(baseline.iter()) {
            assert!((actual - expected).abs() < 1e-12 || (actual.is_nan() && expected.is_nan()));
        }
        assert_eq!(engine.custom_formula_names(), vec!["ZMA".to_string()]);
        assert!(engine.unregister_custom_formula("zma").unwrap());
    }

    #[test]
    fn test_borrowed_slice_path_supports_complex_formula() {
        let ctx = make_ctx(32);
        let mut compiler = FormulaEngine::new();
        let compiled = compiler.compile("MA(CLOSE, 5) + 1").unwrap();
        let borrowed = FormulaEngine::new()
            .eval_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                None,
            )
            .unwrap();

        let mut owned_ctx = make_ctx(32);
        let mut engine = FormulaEngine::new();
        let owned_formula = engine.compile("MA(CLOSE, 5) + 1").unwrap();
        let owned = engine.execute(&owned_formula, &mut owned_ctx).unwrap();
        for (a, b) in borrowed.iter().zip(owned.iter()) {
            assert!((a - b).abs() < 1e-12 || (a.is_nan() && b.is_nan()));
        }
    }

    #[test]
    fn test_borrowed_range_matches_full_history() {
        let mut compiler = FormulaEngine::new();
        let compiled = compiler.compile("MA(CLOSE, 5) + EMA(CLOSE, 3)").unwrap();
        let ctx = make_ctx(64);
        let full = compiler.execute(&compiled, &mut ctx.clone()).unwrap();
        let range = compiler
            .eval_range_zero_copy_inputs(
                &compiled,
                &ctx.open,
                &ctx.high,
                &ctx.low,
                &ctx.close,
                &ctx.volume,
                17,
                41,
                None,
            )
            .unwrap();
        assert_eq!(range.len(), 24);
        for (offset, actual) in range.iter().enumerate() {
            let expected = full[17 + offset];
            assert!((actual - expected).abs() < 1e-12 || (actual.is_nan() && expected.is_nan()));
        }
    }
}

#[cfg(test)]
mod pr14_compute_ir_production_tests {
    use super::*;

    #[test]
    fn compile_populates_semantic_compute_plan_before_execution() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("X:=MA(CLOSE,5);X").unwrap();
        let cache = engine.semantic_plan_cache.borrow();
        let plan = cache.get(&compiled.source).expect("semantic plan cached");
        assert!(!plan.plan().is_empty());
        assert!(plan.plan().has_observable_effects());
    }

    #[test]
    fn pure_formula_plan_remains_incremental_candidate() {
        let mut engine = FormulaEngine::new();
        let compiled = engine.compile("MA(CLOSE,5)").unwrap();
        let cache = engine.semantic_plan_cache.borrow();
        let plan = cache.get(&compiled.source).expect("semantic plan cached");
        assert!(!plan.plan().is_empty());
        assert!(!plan.plan().has_observable_effects());
    }
}

#[cfg(test)]
mod plan_cache_tests {
    use super::*;
    use crate::execution_plan::{ParameterSlot, ParameterValue};

    fn params(pairs: &[(&str, f64)]) -> ParamValues {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_string(), *value))
            .collect()
    }

    /// Every scalar the plan bound, as `f64`, in slot order.
    ///
    /// Integer parameters are stored as `Usize`, so decode both representations
    /// rather than assuming every literal was bound as raw bits.
    fn bound_parameters(plan: &FormulaHotPlan) -> Vec<f64> {
        let arena = plan.hot().parameter_arena();
        (0..arena.len())
            .filter_map(|slot| arena.get(ParameterSlot(slot)))
            .map(|value| match value {
                ParameterValue::F64Bits(bits) => f64::from_bits(bits),
                ParameterValue::Usize(value) => value as f64,
            })
            .collect()
    }

    #[test]
    fn a_repeated_compile_returns_the_cached_plan() {
        let engine = FormulaEngine::new();

        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");

        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn parameter_values_are_part_of_the_plan_identity() {
        let engine = FormulaEngine::new();

        let short = engine
            .compile_plan(
                "MA(CLOSE,N)",
                FormulaDialect::AlphaTA,
                &params(&[("N", 14.0)]),
            )
            .expect("plan compiles");
        let long = engine
            .compile_plan(
                "MA(CLOSE,N)",
                FormulaDialect::AlphaTA,
                &params(&[("N", 20.0)]),
            )
            .expect("plan compiles");

        // Two entries, not one reused plan: the parameter is part of the key.
        assert_eq!(engine.plan_cache_size(), 2);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats {
                hits: 0,
                misses: 2
            }
        );
        // And the plans really differ, rather than being two keys over one plan.
        assert_eq!(bound_parameters(&short), vec![14.0]);
        assert_eq!(bound_parameters(&long), vec![20.0]);
    }

    #[test]
    fn parameter_insertion_order_does_not_split_the_cache() {
        let engine = FormulaEngine::new();
        let source = "MA(CLOSE,A)+MA(CLOSE,B)";

        engine
            .compile_plan(
                source,
                FormulaDialect::AlphaTA,
                &params(&[("A", 3.0), ("B", 5.0)]),
            )
            .expect("plan compiles");
        engine
            .compile_plan(
                source,
                FormulaDialect::AlphaTA,
                &params(&[("B", 5.0), ("A", 3.0)]),
            )
            .expect("plan compiles");

        // `ParamValues` is a `HashMap`, whose iteration order is unspecified, so
        // without a sorted fingerprint these two identical requests would miss
        // twice and compile the same plan twice.
        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn the_dialect_is_part_of_the_plan_identity() {
        let engine = FormulaEngine::new();

        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("AlphaTA plan compiles");
        engine
            .compile_plan(
                "MA(CLOSE,5)",
                FormulaDialect::TongDaXin,
                &ParamValues::new(),
            )
            .expect("TongDaXin plan compiles");

        // AlphaTA and TongDaXin currently share `parse_formula`, so this costs a
        // duplicate plan rather than preventing a wrong reuse. That is the side
        // to err on: Pine genuinely parses the same text to a different AST.
        assert_eq!(engine.plan_cache_size(), 2);
    }

    #[test]
    fn a_pine_source_compiles_through_the_pine_parser() {
        let engine = FormulaEngine::new();

        let plan = engine
            .compile_plan(
                "ta.sma(close, 5)",
                FormulaDialect::Pine,
                &ParamValues::new(),
            )
            .expect("Pine plan compiles");

        assert!(plan.hot().buffer_layout().slot_count() > 0);
    }

    #[test]
    fn an_unplannable_formula_reports_an_error_instead_of_falling_back() {
        let engine = FormulaEngine::new();

        // A statement block whose statements are all drawing directives has no
        // value-producing statement, so the plan layer cannot name a result and
        // must refuse the plan. The tree path would hand back a placeholder
        // buffer; the plan path says so instead of quietly returning something
        // else. (A single `STICKLINE` is not enough — it lowers to a numeric
        // node and only fails later, at execution.)
        let error = engine
            .compile_plan_default(
                "STICKLINE(CLOSE>OPEN,CLOSE,OPEN,3,TRUE);STICKLINE(CLOSE>OPEN,OPEN,CLOSE,3,FALSE)",
            )
            .expect_err("a draw-only block cannot be planned");

        let message = error.to_string();
        assert!(
            message.contains("formula plan compilation failed"),
            "unexpected error: {message}"
        );
        // A failed compile must not be cached, or a later call would report a
        // stale success.
        assert_eq!(engine.plan_cache_size(), 0);
    }

    #[test]
    fn a_parse_failure_is_reported_rather_than_silently_ignored() {
        let engine = FormulaEngine::new();

        assert!(engine.compile_plan_default("MA(CLOSE,").is_err());
        assert_eq!(engine.plan_cache_size(), 0);
    }

    #[test]
    fn registering_a_custom_formula_drops_plans_but_keeps_the_counters() {
        let mut engine = FormulaEngine::new();
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        assert_eq!(engine.plan_cache_size(), 1);

        engine
            .register_custom_formula("SIGNAL", &["X"], "MA(X,5)")
            .expect("custom formula registers");

        // A newly registered component can change what an existing source
        // resolves to, so the plans must go; the counters still describe this
        // engine's behaviour and are kept.
        assert_eq!(engine.plan_cache_size(), 0);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 0, misses: 1 }
        );
    }

    #[test]
    fn clear_plan_cache_drops_entries_and_resets_the_counters() {
        let mut engine = FormulaEngine::new();
        engine
            .compile_plan_default("MA(CLOSE,5)")
            .expect("plan compiles");
        engine.clear_plan_cache();

        assert_eq!(engine.plan_cache_size(), 0);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 0, misses: 0 }
        );
    }
}

#[cfg(test)]
mod plan_execution_tests {
    use super::*;

    /// Deterministic OHLCV with a trend plus a wave, long enough for the longest
    /// warm-up used below.
    #[allow(clippy::cast_precision_loss)] // bar counts here are far below 2^53
    fn ctx(len: usize) -> FormulaContext {
        let series = |base: f64, step: f64, wave: f64| {
            Array1::from_vec(
                (0..len)
                    .map(|index| {
                        let i = index as f64;
                        base + step * i + wave * (i * 0.7).sin()
                    })
                    .collect(),
            )
        };
        FormulaContext::new(
            series(10.0, 0.10, 0.30),
            series(10.5, 0.10, 0.40),
            series(9.5, 0.10, 0.20),
            series(10.0, 0.10, 0.35),
            series(1000.0, 10.0, 50.0),
            None,
        )
    }

    /// Compare two series element-wise, treating two NaNs as equal.
    ///
    /// Warm-up bars are legitimately NaN — `EMA(CLOSE, 26)` has no value for its
    /// first 25 samples — so a plain equality assertion on the vectors would fail
    /// on a correct result.
    fn assert_same_series(plan: &[f64], tree: &[f64], tolerance: f64) {
        assert_eq!(plan.len(), tree.len(), "series lengths differ");
        for (index, (left, right)) in plan.iter().zip(tree.iter()).enumerate() {
            assert!(
                (left - right).abs() <= tolerance || (left.is_nan() && right.is_nan()),
                "index {index}: plan {left} vs tree {right}"
            );
        }
    }

    /// Evaluate `source` on both paths and assert they agree element-wise.
    fn assert_plan_matches_tree(source: &str, len: usize) {
        let context = ctx(len);
        let plan = FormulaEngine::new()
            .eval_plan(source, &context)
            .unwrap_or_else(|error| panic!("plan path failed for `{source}`: {error}"));

        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval(source, &mut tree_context)
            .unwrap_or_else(|error| panic!("tree path failed for `{source}`: {error}"));

        assert_same_series(&plan.to_vec(), &tree.to_vec(), 1e-9);
    }

    #[test]
    fn a_stateless_plan_matches_the_tree_path() {
        assert_plan_matches_tree("CLOSE * 2 + 1", 60);
        assert_plan_matches_tree("MA(CLOSE, 5)", 60);
        assert_plan_matches_tree("HHV(HIGH, 10) - LLV(LOW, 10)", 60);
    }

    #[test]
    fn a_stateful_plan_matches_the_tree_path() {
        assert_plan_matches_tree("EMA(CLOSE, 12)", 80);
        assert_plan_matches_tree("RSI(CLOSE, 14)", 80);
        assert_plan_matches_tree("EMA(CLOSE, 12) + ROC(CLOSE, 10)", 80);
    }

    #[test]
    fn plan_and_tree_agree_on_warm_up_nans() {
        let context = ctx(40);
        let plan = FormulaEngine::new()
            .eval_plan("EMA(CLOSE, 12)", &context)
            .expect("plan evaluates");
        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval("EMA(CLOSE, 12)", &mut tree_context)
            .expect("tree evaluates");

        let plan = plan.to_vec();
        let tree = tree.to_vec();
        // The first 11 samples have no EMA yet, on both paths.
        assert!(
            plan[..11].iter().all(|value| value.is_nan()),
            "plan warm-up should be NaN: {:?}",
            &plan[..11]
        );
        assert!(tree[..11].iter().all(|value| value.is_nan()));
        assert!(plan[11].is_finite() && tree[11].is_finite());
        assert_same_series(&plan, &tree, 1e-9);
    }

    #[test]
    fn a_multi_channel_formula_exposes_every_named_output() {
        let engine = FormulaEngine::new();
        let context = ctx(80);

        let output = engine
            .eval_plan_channels(
                "DIF:EMA(CLOSE,12)-EMA(CLOSE,26);DEA:EMA(DIF,9);MACD:(DIF-DEA)*2",
                FormulaDialect::AlphaTA,
                &ParamValues::new(),
                &context,
            )
            .expect("multi-channel plan evaluates");

        // Every `OUTPUT:` declaration is addressable by name, not just the last
        // line — that is what multi-root retention in the plan layer buys.
        assert_eq!(output.channels().len(), 3);
        for name in ["DIF", "DEA", "MACD"] {
            let channel = output
                .channel(name)
                .unwrap_or_else(|| panic!("channel `{name}` missing from {:?}", output.channels()));
            assert_eq!(channel.len(), 80);
        }
        assert_eq!(output.primary().len(), 80);

        // `DIF` is the first channel and must carry real data: the plan lowered
        // `DIF:...` into a local write (see the `OUTPUT:` arm of
        // `lower_formula_plumbing`), so the later `EMA(DIF, 9)` reads it as a
        // variable rather than demanding a caller-supplied series named `DIF`.
        let dif = output.channel("DIF").unwrap();
        assert!(
            dif.iter().any(|value| value.is_finite()),
            "the DIF channel should carry computed values, not warm-up NaNs only"
        );

        // The primary is the value of the last value-producing statement, which
        // for this formula is `MACD` — so the two must be the *same* series. This
        // is the case that used to hand back an empty primary, because resolving
        // the channels consumed the very buffer the primary also points at.
        //
        // Compared NaN-aware, not with `assert_eq!`: `DEA` is `EMA(DIF, 9)` and
        // `DIF` is NaN for its first 25 samples, so the EMA seed averages a NaN
        // window and `DEA`/`MACD` are legitimately all-NaN here. Slice equality
        // would fail on two identical vectors because `NaN != NaN`.
        assert_same_series(output.primary(), output.channel("MACD").unwrap(), 0.0);
        assert!(output.channel("NOPE").is_none());
    }

    #[test]
    fn a_multi_channel_formula_carries_distinct_finite_series() {
        // A channel that is *not* the primary must still be its own series: this
        // formula's last statement is `C`, but `A` and `B` are retained roots in
        // their own right. Using rolling means (rather than EMAs) keeps the
        // values finite from index 9, so the comparison is numeric rather than a
        // warm-up-NaN artefact.
        let engine = FormulaEngine::new();
        let context = ctx(60);

        let output = engine
            .eval_plan_channels(
                "A:MA(CLOSE,5);B:MA(CLOSE,10);C:A-B",
                FormulaDialect::AlphaTA,
                &ParamValues::new(),
                &context,
            )
            .expect("multi-channel plan evaluates");

        let a = output.channel("A").expect("channel A").to_vec();
        let b = output.channel("B").expect("channel B").to_vec();
        let c = output.channel("C").expect("channel C").to_vec();

        assert!(c.iter().any(|value| value.is_finite()));
        assert_eq!(c[9], a[9] - b[9], "C should be A - B at index 9");
        assert!(
            a.iter().zip(b.iter()).any(|(left, right)| left != right),
            "A and B should be different series"
        );

        // `C` is the last statement, so it is both the primary and a channel.
        assert_same_series(output.primary(), &c, 0.0);

        // And the whole channel set must agree with the reference tree path.
        let mut tree_context = context.clone();
        let tree = FormulaEngine::new()
            .eval("A:MA(CLOSE,5);B:MA(CLOSE,10);C:A-B", &mut tree_context)
            .expect("tree evaluates");
        assert_same_series(output.primary(), &tree.to_vec(), 1e-9);
    }

    #[test]
    fn a_missing_input_series_is_reported_rather_than_substituted() {
        let engine = FormulaEngine::new();
        let context = ctx(40);

        // `MYVAR` is neither OHLCV nor a context variable. The plan path must
        // refuse, because silently substituting another series would hide an
        // input-layout bug behind a numeric mismatch.
        let error = engine
            .eval_plan("MA(MYVAR, 5)", &context)
            .expect_err("an unbound input must fail");

        let message = error.to_string();
        assert!(
            message.contains("MYVAR"),
            "error should name the missing series: {message}"
        );
    }

    #[test]
    fn a_repeated_evaluation_reuses_the_cached_plan() {
        let engine = FormulaEngine::new();
        let context = ctx(40);

        engine
            .eval_plan("MA(CLOSE,5)", &context)
            .expect("plan evaluates");
        engine
            .eval_plan("MA(CLOSE,5)", &context)
            .expect("plan evaluates");

        assert_eq!(engine.plan_cache_size(), 1);
        assert_eq!(
            engine.plan_cache_stats(),
            FormulaPlanCacheStats { hits: 1, misses: 1 }
        );
    }

    #[test]
    fn an_unsupported_operator_fails_loudly_instead_of_falling_back() {
        let engine = FormulaEngine::new();
        let context = ctx(40);

        // `FILTER` is a documented kernel gap. The plan path must report it
        // rather than quietly re-running the tree path, otherwise a caller
        // could never tell which engine produced a number.
        //
        // `IF` used to stand in here and had to be swapped out once it gained
        // a kernel. If this fails because the function below now works, pick
        // another entry from the plan-kernel backlog -- the point of the test
        // is that *some* gap still fails loudly, so do not delete the check.
        let error = engine
            .eval_plan("FILTER(CLOSE>OPEN, 5)", &context)
            .expect_err("a kernel-less operator must fail");

        let message = error.to_string();
        assert!(
            message.contains("formula plan"),
            "error should identify the plan path: {message}"
        );
    }
}
