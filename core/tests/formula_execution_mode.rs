//! `FormulaExecutionMode` is a real switch, not a hint.
//!
//! Parity alone would not prove that: with the differential gates green on both
//! corpora, a switch wired to nothing would also produce identical numbers.
//! So these tests pin the two things that actually differ between the paths,
//! and the parity that must hold for everything else.

use finkit::formula::{FormulaContext, FormulaDialect, FormulaEngine, FormulaExecutionMode};
use finkit::FormulaError;
use ndarray::Array1;

fn context(n: usize) -> FormulaContext {
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut rng = || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (u64::MAX as f64)
    };
    let mut close = Vec::with_capacity(n);
    let mut c = 100.0;
    for _ in 0..n {
        c += (rng() - 0.5) * 4.0;
        close.push(c);
    }
    let open = close.clone();
    let high: Vec<f64> = close.iter().map(|v| v + 1.0).collect();
    let low: Vec<f64> = close.iter().map(|v| v - 1.0).collect();
    let volume: Vec<f64> = (0..n).map(|i| 1000.0 + i as f64).collect();
    FormulaContext::new(
        Array1::from_vec(open),
        Array1::from_vec(high),
        Array1::from_vec(low),
        Array1::from_vec(close),
        Array1::from_vec(volume),
        None,
    )
}

const FORMULAS: &[&str] = &[
    "MA(CLOSE, 5)",
    "EMA(CLOSE, 12)",
    "RSI(CLOSE, 14)",
    "IF(CLOSE > 0, 1, 0)",
    "(CLOSE + OPEN) / 2",
    "HHV(HIGH, 10)",
];

/// The source the backend-contract tests below evaluate.
const TREE_ONLY_SOURCE: &str = "MA(CLOSE, 5)";

/// Assert that a source-level entry point refuses to run on the `Plan` backend
/// with `BackendUnsupported` naming itself, and stays reachable on the tree
/// backend.
///
/// `$call` is invoked twice — once on a Plan engine, once on a Tree engine — and
/// its result is normalised to `Result<(), FormulaError>` so entries with
/// different return types share one assertion. The tree half only asserts that
/// the guard did *not* fire: whether the formula itself is valid is the business
/// of the entry point's own tests.
macro_rules! assert_tree_only {
    ($entry:literal, $call:expr) => {{
        let mut plan_engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
        let mut plan_ctx = context(32);
        match $call(&mut plan_engine, &mut plan_ctx) {
            Err(FormulaError::BackendUnsupported { backend, entry }) => {
                assert_eq!(backend, "plan", "{}: wrong backend named", $entry);
                assert_eq!(entry, $entry, "{}: wrong entry named", $entry);
            }
            other => panic!(
                "{}: plan mode must report BackendUnsupported, got {other:?}",
                $entry
            ),
        }

        let mut tree_engine = FormulaEngine::new();
        let mut tree_ctx = context(32);
        if let Err(FormulaError::BackendUnsupported { .. }) = $call(&mut tree_engine, &mut tree_ctx)
        {
            panic!("{}: the tree backend must serve this entry", $entry);
        }
    }};
}

#[test]
fn tree_is_the_default() {
    assert_eq!(
        FormulaEngine::new().execution_mode(),
        FormulaExecutionMode::Tree,
        "the reference path must stay the default until flipping it is a deliberate release decision"
    );
}

#[test]
fn both_modes_agree_on_the_primary_series() {
    for source in FORMULAS {
        let ctx = context(64);
        let tree = FormulaEngine::new()
            .eval(source, &mut ctx.clone())
            .unwrap_or_else(|e| panic!("tree failed on {source}: {e}"));
        let plan = FormulaEngine::new()
            .with_execution_mode(FormulaExecutionMode::Plan)
            .eval(source, &mut ctx.clone())
            .unwrap_or_else(|e| panic!("plan failed on {source}: {e}"));

        assert_eq!(tree.len(), plan.len(), "length mismatch on {source}");
        for index in 0..tree.len() {
            let matches = (tree[index] - plan[index]).abs() < 1e-9
                || (tree[index].is_nan() && plan[index].is_nan());
            assert!(
                matches,
                "{source} index {index}: tree={} plan={}",
                tree[index], plan[index]
            );
        }
    }
}

/// The switch must actually move work onto the other path.
///
/// The tree-walker writes assignment results back into `ctx.variables`; the
/// plan path reads inputs and returns outputs without touching it. Observing
/// that difference is what distinguishes a wired switch from a no-op — parity
/// tests cannot, because both paths now agree numerically.
#[test]
fn the_mode_changes_which_path_runs() {
    let source = "X:MA(CLOSE, 5);";

    // `ctx.variables` can no longer tell the two paths apart: both publish the
    // same assignments, which is the point of the parity work. The compiled-plan
    // cache is the remaining signal — only the plan path ever populates it — so a
    // non-zero plan cache is what proves Plan mode compiled a plan instead of
    // quietly delegating to the tree-walker.
    let mut tree_engine = FormulaEngine::new();
    let mut tree_ctx = context(32);
    tree_engine
        .eval(source, &mut tree_ctx)
        .expect("tree path must evaluate");
    assert!(
        tree_ctx.variables.contains_key("X"),
        "the tree path is expected to publish assignments into ctx.variables; \
         if it stops, this test no longer discriminates the two paths"
    );
    assert_eq!(
        tree_engine.plan_cache_size(),
        0,
        "the tree path must not compile a plan"
    );

    let mut plan_engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let mut plan_ctx = context(32);
    plan_engine
        .eval(source, &mut plan_ctx)
        .expect("plan path must evaluate");
    assert_eq!(
        plan_engine.plan_cache_size(),
        1,
        "Plan mode must have compiled a plan — a zero plan cache means the engine \
         is still running the tree-walker"
    );

    // The parity itself: the two paths must agree on which variables they
    // publish. Values are compared by the differential gates, which know how to
    // treat the warm-up NaN that `MA` emits.
    let names = |ctx: &FormulaContext| {
        let mut names: Vec<String> = ctx.variables.keys().map(|k| k.to_string()).collect();
        names.sort();
        names
    };
    assert_eq!(
        names(&tree_ctx),
        names(&plan_ctx),
        "the two modes must publish the same variables"
    );
}

/// Plan mode must reject a formula it cannot lower rather than quietly
/// producing a tree-path answer.
#[test]
fn plan_mode_fails_loudly_instead_of_falling_back() {
    let ctx = context(32);
    // `WHILE` has no acyclic lowering, so this is compile-time unsupported.
    let source = "X:0; WHILE CLOSE > 0 X:X+1;";

    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval(source, &mut ctx.clone());

    assert!(
        plan.is_err(),
        "Plan mode must not silently succeed on a formula it cannot lower"
    );
}

/// The switch is reachable through the dialect-aware entry point too, since
/// that is what FFI callers use.
#[test]
fn the_switch_applies_to_dialect_entry_points() {
    let ctx = context(32);
    let tree = FormulaEngine::new()
        .eval_with_dialect("MA(CLOSE, 5)", FormulaDialect::TongDaXin, &mut ctx.clone())
        .expect("tree");
    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval_with_dialect("MA(CLOSE, 5)", FormulaDialect::TongDaXin, &mut ctx.clone())
        .expect("plan");

    assert_eq!(tree.len(), plan.len());
    for index in 0..tree.len() {
        let matches = (tree[index] - plan[index]).abs() < 1e-9
            || (tree[index].is_nan() && plan[index].is_nan());
        assert!(
            matches,
            "index {index}: tree={} plan={}",
            tree[index], plan[index]
        );
    }
}

/// A formula with no bound input still has a length: the context's.
///
/// The plan path used to derive the execution length from the first bound input
/// slot, so a constant expression (`10 + 20`) had no length to run over and
/// failed with "cannot infer execution length without bound inputs". The tree
/// path broadcast the constant across `ctx.data_len`, so the two modes
/// disagreed on whether the formula was even runnable.
#[test]
fn constant_only_formulas_run_at_the_context_length() {
    for source in ["10 + 20", "SQRT(9) * 2", "IF(1, 7, 8)"] {
        let tree = FormulaEngine::new()
            .eval(source, &mut context(5))
            .unwrap_or_else(|error| panic!("tree failed on `{source}`: {error:?}"));
        let plan = FormulaEngine::new()
            .with_execution_mode(FormulaExecutionMode::Plan)
            .eval(source, &mut context(5))
            .unwrap_or_else(|error| panic!("plan failed on `{source}`: {error:?}"));

        assert_eq!(tree.len(), 5, "`{source}` must span the whole context");
        assert_eq!(plan.len(), 5, "`{source}` must span the whole context");
        for index in 0..tree.len() {
            assert!(
                (tree[index] - plan[index]).abs() < 1e-12,
                "`{source}` index {index}: tree={} plan={}",
                tree[index],
                plan[index]
            );
        }
    }
}

/// The public cache statistics must describe the cache the engine is using.
///
/// Under `Plan` the engine compiles through the plan cache and never touches the
/// AST cache, so reporting the AST cache made `cache_hit` return false and
/// `cache_size` return 0 for a formula that was in fact cached. A caller could
/// not tell a warm engine from a cold one.
#[test]
fn cache_statistics_follow_the_active_mode() {
    for mode in [FormulaExecutionMode::Tree, FormulaExecutionMode::Plan] {
        let mut engine = FormulaEngine::new().with_execution_mode(mode);
        assert!(
            !engine.cache_hit("MA(CLOSE, 5)"),
            "{mode:?}: nothing is compiled yet"
        );

        engine.eval("MA(CLOSE, 5)", &mut context(32)).expect("eval");
        assert!(
            engine.cache_hit("MA(CLOSE, 5)"),
            "{mode:?}: the formula just ran, so its compiled form is held"
        );
        assert_eq!(engine.cache_size(), 1, "{mode:?}");

        engine.clear_cache();
        assert!(
            !engine.cache_hit("MA(CLOSE, 5)"),
            "{mode:?}: clear_cache must drop the cache this mode reads"
        );
        assert_eq!(engine.cache_size(), 0, "{mode:?}");
    }
}

/// `clear_cache` must drop both caches, not just the active one.
///
/// Leaving the plan cache behind would let an evaluation in plan mode skip
/// recompilation immediately after the caller asked for a clean slate — the
/// reason to call this at all is that a source may now resolve differently.
#[test]
fn clear_cache_drops_the_plan_cache_from_tree_mode_too() {
    let mut engine = FormulaEngine::new();
    engine
        .eval_plan("MA(CLOSE, 5)", &mut context(32))
        .expect("plan");
    assert_eq!(engine.plan_cache_size(), 1);

    engine.clear_cache();
    assert_eq!(
        engine.plan_cache_size(),
        0,
        "clear_cache left a compiled plan behind while in Tree mode"
    );
}

// ============================================================================
// Backend contract: which entry points the switch governs.
//
// The table in `FormulaEngine`'s documentation claims an exhaustive split.
// These tests are what make that claim checkable: moving an entry between
// families without updating both the table and this file turns them red.
// ============================================================================

/// Every source-level entry point the plan backend cannot serve must *say so*,
/// and must stay reachable on the tree backend.
#[test]
fn tree_only_entries_refuse_to_run_under_plan() {
    let ast = finkit::formula::parse_formula(TREE_ONLY_SOURCE).expect("parse TREE_ONLY_SOURCE");

    assert_tree_only!(
        "eval_ast",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_ast(&ast, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_lazy",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_lazy(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_parallel",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_parallel(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_optimized",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_optimized(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_with_debug",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_with_debug(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_template",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_template("均线金叉死叉", ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_with_validation",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_with_validation(TREE_ONLY_SOURCE, ctx, &finkit::formula::ParamValues::new())
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_with_defaults",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_with_defaults(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_zero_copy",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_zero_copy(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
}

/// The tail of the same roster: `eval_zero_copy_cached`, `eval_zero_alloc` and
/// the two Pine entries. `eval_multi_with_dialect` is plan-capable for every
/// dialect *except* Pine; `eval_multi_with_pine_security` is tree-only outright.
/// Split out only so each test stays inside the lint's size budget — this is one
/// roster, not two categories.
#[test]
fn remaining_tree_only_entries_refuse_to_run_under_plan() {
    assert_tree_only!(
        "eval_zero_copy_cached",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_zero_copy_cached(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_zero_alloc",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_zero_alloc(TREE_ONLY_SOURCE, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_multi_with_dialect(Pine)",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_multi_with_dialect(TREE_ONLY_SOURCE, FormulaDialect::Pine, ctx)
            .map(|_| ())
    );
    assert_tree_only!(
        "eval_multi_with_pine_security",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_multi_with_pine_security(TREE_ONLY_SOURCE, ctx, &RejectingSecurityResolver)
            .map(|_| ())
    );
}

/// A resolver that never supplies host data. These contract tests exercise the
/// backend guard, which fires before any `request.security` lookup, so the
/// resolver only needs to be a valid value of the right type.
struct RejectingSecurityResolver;

impl finkit::formula::PineSecurityResolver for RejectingSecurityResolver {
    fn resolve_security(
        &self,
        _args: &[(Option<String>, finkit::formula::PineAstNode)],
    ) -> Result<finkit::formula::AstNode, finkit::formula::PineMapperError> {
        Err(finkit::formula::PineMapperError {
            message: "no host security data in the backend-contract test".to_string(),
        })
    }
}

/// The plan-capable source-level entries stay governed: they must not report
/// `BackendUnsupported`, and they must actually populate the plan cache.
#[test]
fn plan_capable_entries_stay_governed() {
    macro_rules! assert_governed {
        ($entry:literal, $call:expr) => {{
            let mut engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
            let mut ctx = context(32);
            $call(&mut engine, &mut ctx)
                .unwrap_or_else(|error| panic!("{}: plan must serve this entry: {error}", $entry));
            assert!(
                engine.plan_cache_size() > 0,
                "{}: a zero plan cache means the engine ran the tree-walker instead",
                $entry
            );
        }};
    }

    assert_governed!(
        "eval",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval("MA(CLOSE, 5)", ctx)
            .map(|_| ())
    );
    assert_governed!(
        "eval_with_dialect",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_with_dialect("MA(CLOSE, 5)", FormulaDialect::TongDaXin, ctx)
            .map(|_| ())
    );
    assert_governed!(
        "eval_incremental",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_incremental("MA(CLOSE, 5)", ctx)
            .map(|_| ())
    );
    assert_governed!(
        "eval_multi",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_multi("MA(CLOSE, 5)", ctx)
            .map(|_| ())
    );
    assert_governed!(
        "eval_batch",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_batch(&["MA(CLOSE, 5)"], ctx)
            .map(|_| ())
    );
    assert_governed!(
        "eval_batch_shared",
        |engine: &mut FormulaEngine, ctx: &mut FormulaContext| engine
            .eval_batch_shared(&["MA(CLOSE, 5)"], ctx)
            .map(|_| ())
    );
}

/// `eval_simd` is a frozen alias of `eval`, so it inherits the switch.
#[test]
#[cfg(feature = "formula-simd")]
fn eval_simd_alias_is_governed_like_eval() {
    let mut engine = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let mut ctx = context(32);
    engine
        .eval_simd("MA(CLOSE, 5)", &mut ctx)
        .expect("eval_simd must follow the selected backend, not pin itself to the tree");
    assert!(
        engine.plan_cache_size() > 0,
        "eval_simd must route through the plan backend when Plan is selected"
    );
}

/// A batch containing an observable effect is evaluated strictly in order.
///
/// The duplicate-result cache used to store every result, including effectful
/// ones, so a repeated formula had its assignment/drawing side effects silently
/// skipped the second time round.
#[test]
fn eval_batch_does_not_reuse_results_across_observable_effects() {
    // The first entry assigns `X`; the second reads it. If the batch reused a
    // cached read from before the assignment, the two modes would disagree.
    let batch: &[&str] = &["X:10; X", "X + CLOSE"];

    let mut tree_ctx = context(8);
    let tree = FormulaEngine::new()
        .eval_batch(batch, &mut tree_ctx)
        .expect("tree batch");

    let mut plan_ctx = context(8);
    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval_batch(batch, &mut plan_ctx)
        .expect("plan batch");

    assert_eq!(tree.len(), 2);
    assert_eq!(plan.len(), 2);
    // `X + CLOSE` must have seen the assignment from the first formula.
    for index in 0..8 {
        let expected = 10.0 + tree_ctx_close(&tree_ctx, index);
        assert!(
            (tree[1][index] - expected).abs() < 1e-9,
            "tree: second batch entry index {index} = {} but X+CLOSE = {expected}",
            tree[1][index]
        );
        assert!(
            (plan[1][index] - expected).abs() < 1e-9,
            "plan: second batch entry index {index} = {} but X+CLOSE = {expected}",
            plan[1][index]
        );
    }
}

fn tree_ctx_close(ctx: &FormulaContext, index: usize) -> f64 {
    ctx.close_view()[index]
}
