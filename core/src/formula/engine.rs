use crate::formula::ast::AstNode;
use crate::formula::bytecode::{compile_to_bytecode, Bytecode, BytecodeVM};
use crate::formula::compiler::{CompiledFormula, FormulaCache};
use crate::formula::debugger::FormulaDebugger;
use crate::formula::executor::FormulaExecutor;
use crate::formula::jit::{JitCompiler, OptimizedBytecode};
use crate::formula::optimizer::{DependencyAnalyzer, FormulaOptimizer};
use crate::formula::params::{apply_params, parse_params, validate_params, ParamDef, ParamValues};
use crate::formula::parser::parse_formula;
use crate::formula::templates::{FormulaTemplate, FormulaTemplates};
use crate::formula::types::*;
use ndarray::Array1;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

/// 公式引擎主入口
pub struct FormulaEngine {
    executor: FormulaExecutor,
    cache: FormulaCache,
    templates: FormulaTemplates,
    jit_compiler: RefCell<JitCompiler>,
    /// Persistent bytecode cache and VM scratch buffers.
    bytecode_cache: RefCell<HashMap<String, Bytecode>>,
    bytecode_vm: RefCell<BytecodeVM>,
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
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(HashMap::new()),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
        }
    }

    pub fn with_cache_size(cache_size: usize) -> Self {
        Self {
            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
            templates: FormulaTemplates::new(),
            jit_compiler: RefCell::new(JitCompiler::new()),
            bytecode_cache: RefCell::new(HashMap::new()),
            bytecode_vm: RefCell::new(BytecodeVM::new()),
        }
    }

    /// 编译公式字符串
    pub fn compile(&mut self, source: &str) -> Result<CompiledFormula, FormulaError> {
        if let Some(formula) = self.cache.get_cloned(source) {
            return Ok(formula);
        }

        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        // Compile the optimized AST once so repeated evaluations share the
        // same CSE and constant-folding decisions while preserving assignment
        // side effects exposed through FormulaContext::variables.
        let ast = FormulaOptimizer::optimize_for_execution(&ast);
        let formula = CompiledFormula {
            ast,
            source: source.to_string(),
        };

        self.cache.insert(source, formula.clone());

        Ok(formula)
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
        let window_start = FormulaOptimizer::required_lookback(&formula.ast)
            .map(|lookback| start.saturating_sub(lookback))
            .unwrap_or(0);
        let mut window = ctx.window(window_start, end)?;
        let result = self.execute(formula, &mut window)?;
        let local_start = start - window_start;
        Ok(result
            .slice(ndarray::s![local_start..(local_start + end - start)])
            .to_owned())
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
        let result = self.eval_range(formula, ctx, ctx.data_len - 1, ctx.data_len)?;
        Ok(result[0])
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

    /// 惰性求值：通过依赖分析只计算最终输出所需的变量
    pub fn eval_lazy(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let pruned = DependencyAnalyzer::analyze_and_prune(&ast);
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

        match ast {
            AstNode::Statements(stmts) => {
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
                                    let mut local_exec = FormulaExecutor::new();
                                    let result = local_exec.execute(stmt, &mut local_ctx);
                                    (
                                        result,
                                        local_ctx.variables,
                                        local_ctx.output_modifiers,
                                        local_ctx.draw_commands.into_inner(),
                                    )
                                })
                                .collect();
                            for (r, vars, mods, draws) in results {
                                last_result = r?;
                                ctx.variables.extend(vars);
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
        let mut results = Vec::with_capacity(formulas.len());
        for &source in formulas {
            let result = self.eval(source, ctx)?;
            results.push(result);
        }
        Ok(results)
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

    #[cfg(feature = "formula-jit")]
    pub fn eval_jit(
        &mut self,
        source: &str,
        ctx: &mut FormulaContext,
    ) -> Result<Array1<f64>, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = FormulaOptimizer::optimize(&ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        let optimized = jit.compile_cached(bytecode);
        jit.execute(&optimized, ctx).map(|r| r.final_value)
    }

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

    #[cfg(feature = "formula-jit")]
    pub fn compile_jit(&mut self, source: &str) -> Result<OptimizedBytecode, FormulaError> {
        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = FormulaOptimizer::optimize(&ast);
        let bytecode = compile_to_bytecode(&ast, source).map_err(FormulaError::RuntimeError)?;
        let mut jit = self.jit_compiler.borrow_mut();
        Ok(jit.compile_cached(bytecode))
    }

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
}
