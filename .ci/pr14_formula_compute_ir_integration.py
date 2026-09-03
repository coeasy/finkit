from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, got {count}")
    return text.replace(old, new, 1)


# Standalone FormulaCompiler must validate semantic Compute IR before AST
# optimization/bytecode-style execution, so the IR is part of the production
# compile contract rather than an isolated documentation/test artifact.
compiler_path = Path("core/src/formula/compiler.rs")
compiler = compiler_path.read_text(encoding="utf-8")
compiler = replace_once(
    compiler,
    "use crate::formula::ast::AstNode;\n",
    "use crate::formula::ast::AstNode;\nuse crate::formula::compute_ir::FormulaComputePlan;\n",
    "FormulaCompiler Compute IR import",
)
compiler = replace_once(
    compiler,
    '''        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        let ast = FormulaOptimizer::optimize(&ast);
''',
    '''        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        FormulaComputePlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!(
                "formula compute planning failed: {error}"
            ))
        })?;
        let ast = FormulaOptimizer::optimize(&ast);
''',
    "FormulaCompiler semantic planning",
)
compiler_path.write_text(compiler, encoding="utf-8")


# FormulaEngine retains semantic plans in its compiled-source cache. The AST
# remains the numerical executor for compatibility, while Compute IR provides
# semantic validation and effect barriers to the incremental planner.
engine_path = Path("core/src/formula/engine.rs")
engine = engine_path.read_text(encoding="utf-8")
engine = replace_once(
    engine,
    "use crate::formula::compiler::{CompiledFormula, FormulaCache};\n",
    "use crate::formula::compiler::{CompiledFormula, FormulaCache};\nuse crate::formula::compute_ir::FormulaComputePlan;\n",
    "FormulaEngine Compute IR import",
)
engine = replace_once(
    engine,
    '''    executor: FormulaExecutor,
    cache: FormulaCache,
    templates: FormulaTemplates,
''',
    '''    executor: FormulaExecutor,
    cache: FormulaCache,
    /// Semantic Compute IR plans keyed by the exact formula source.
    semantic_plan_cache: RefCell<HashMap<String, FormulaComputePlan>>,
    templates: FormulaTemplates,
''',
    "FormulaEngine semantic plan cache field",
)
engine = replace_once(
    engine,
    '''            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(100),
            templates: FormulaTemplates::new(),
''',
    '''            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(100),
            semantic_plan_cache: RefCell::new(HashMap::new()),
            templates: FormulaTemplates::new(),
''',
    "FormulaEngine default semantic plan cache",
)
engine = replace_once(
    engine,
    '''            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
            templates: FormulaTemplates::new(),
''',
    '''            executor: FormulaExecutor::new(),
            cache: FormulaCache::new(cache_size),
            semantic_plan_cache: RefCell::new(HashMap::new()),
            templates: FormulaTemplates::new(),
''',
    "FormulaEngine sized semantic plan cache",
)
engine = replace_once(
    engine,
    '''        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        // Compile the optimized AST once so repeated evaluations share the
        // same CSE and constant-folding decisions while preserving assignment
        // side effects exposed through FormulaContext::variables.
        let ast = FormulaOptimizer::optimize_for_execution(&ast);
        let formula = CompiledFormula {
            ast,
            source: source.to_string(),
        };

        self.cache.insert(source, formula.clone());
''',
    '''        let ast = parse_formula(source).map_err(FormulaError::ParseError)?;
        // Semantic analysis is deliberately performed before AST optimization.
        // This locks dependencies/effects against the source program so later
        // optimization and incremental execution cannot accidentally erase an
        // assignment, output, drawing command, or control-flow barrier.
        let semantic_plan = FormulaComputePlan::compile(&ast).map_err(|error| {
            FormulaError::InvalidOperation(format!(
                "formula compute planning failed: {error}"
            ))
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
''',
    "FormulaEngine semantic planning before optimization",
)
engine = replace_once(
    engine,
    '''        let window_start = FormulaOptimizer::required_lookback(&formula.ast)
            .map(|lookback| start.saturating_sub(lookback))
            .unwrap_or(0);
''',
    '''        let cached_effects = {
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
''',
    "FormulaEngine effect-aware incremental window",
)
if "mod pr14_compute_ir_production_tests" not in engine:
    engine += r'''

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
'''
engine_path.write_text(engine, encoding="utf-8")

print("Formula compile path now validates and consumes semantic Compute IR")
