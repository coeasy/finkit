//! Differential consistency tests across formula execution paths.
//!
//! For each formula and input, compares AST interpretation, bytecode VM,
//! JIT (when enabled), SIMD (when enabled), and the compiled plan path
//! (`FormulaHotPlan` + `UnifiedExecutor`). Any divergence beyond tolerance
//! `1e-10` fails the test with index and values printed.
//!
//! The plan path is the production target of the P0-2 workstream: it must be
//! proven bit-comparable to the AST tree-walker *before* it is allowed to
//! become the default execution mode. This file is that gate for the
//! hand-written formula set; `formula_plan_differential.rs` extends the same
//! gate over the on-disk corpora.

use finkit::execution_plan::KernelId;
use finkit::formula::{
    parse_formula, unified_formula_executor, FormulaContext, FormulaEngine, FormulaHotPlan,
};
use ndarray::Array1;

const TOLERANCE: f64 = 1e-10;

fn make_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 100.0 + i as f64 * 0.5).collect());
    let high = Array1::from_vec((0..len).map(|i| 105.0 + i as f64 * 0.7).collect());
    let low = Array1::from_vec((0..len).map(|i| 95.0 + i as f64 * 0.3).collect());
    let close = Array1::from_vec((0..len).map(|i| 102.0 + i as f64 * 0.6).collect());
    let volume = Array1::from_vec((0..len).map(|i| 10000.0 + i as f64 * 100.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

fn values_match(a: f64, b: f64) -> bool {
    if a.is_nan() && b.is_nan() {
        return true;
    }
    if a.is_nan() || b.is_nan() {
        return false;
    }
    (a - b).abs() <= TOLERANCE
}

fn assert_arrays_match(
    formula_name: &str,
    path_name: &str,
    reference: &Array1<f64>,
    candidate: &Array1<f64>,
) {
    assert_eq!(
        reference.len(),
        candidate.len(),
        "formula {formula_name}: path {path_name} length mismatch (ref={}, got={})",
        reference.len(),
        candidate.len()
    );
    for i in 0..reference.len() {
        if !values_match(reference[i], candidate[i]) {
            panic!(
                "formula {}: divergence at index {} between ast and {}: \
                 ast={}, {}={} (tolerance={})",
                formula_name, i, path_name, reference[i], path_name, candidate[i], TOLERANCE
            );
        }
    }
}

fn run_ast(engine: &mut FormulaEngine, source: &str, ctx: &mut FormulaContext) -> Array1<f64> {
    engine.eval(source, ctx).expect("AST eval failed")
}

fn run_bytecode(engine: &mut FormulaEngine, source: &str, ctx: &FormulaContext) -> Array1<f64> {
    let bytecode = engine
        .compile_bytecode(source)
        .expect("bytecode compile failed");
    engine
        .execute_bytecode(&bytecode, ctx)
        .expect("bytecode execute failed")
}

#[cfg(feature = "formula-jit")]
fn run_jit(engine: &mut FormulaEngine, source: &str, ctx: &mut FormulaContext) -> Array1<f64> {
    engine.eval_jit(source, ctx).expect("JIT eval failed")
}

#[cfg(feature = "formula-simd")]
fn run_simd(engine: &mut FormulaEngine, source: &str, ctx: &mut FormulaContext) -> Array1<f64> {
    engine.eval_simd(source, ctx).expect("SIMD eval failed")
}

/// Execute a formula through the compiled plan path.
///
/// This mirrors exactly what a production caller does: parse once, compile to a
/// `FormulaHotPlan` (semantic DAG -> CSE -> hot plan), resolve the context
/// series into the plan's numeric input slots, then drive the unified executor.
fn run_plan(source: &str, ctx: &FormulaContext) -> Array1<f64> {
    let ast = parse_formula(source).expect("plan parse failed");
    let plan = FormulaHotPlan::compile(&ast).expect("plan compile failed");

    let slot_count = plan.hot().input_layout().len();
    let close = ctx.get_data("CLOSE").expect("CLOSE missing from context");
    let mut inputs: Vec<&[f64]> = vec![close; slot_count];
    for name in ["OPEN", "HIGH", "LOW", "CLOSE", "VOLUME"] {
        if let Some(values) = ctx.get_data(name) {
            let op = format!("VARIABLE:{name}");
            if let Some(slot) = plan.hot().input_layout().slot_for_operation(&op) {
                inputs[slot.0] = values;
            }
        }
    }

    let mut executor = unified_formula_executor(&plan);
    let result = executor.execute(&inputs).expect("plan execute failed");
    let values = result
        .values
        .into_iter()
        .next()
        .expect("plan produced no output series");
    Array1::from_vec(values)
}

fn check_all_paths(formula_name: &str, source: &str, data_len: usize) {
    let mut engine = FormulaEngine::new();

    let mut ctx_ast = make_ctx(data_len);
    let reference = run_ast(&mut engine, source, &mut ctx_ast);

    let ctx_bc = make_ctx(data_len);
    let bytecode_result = run_bytecode(&mut engine, source, &ctx_bc);
    assert_arrays_match(formula_name, "bytecode", &reference, &bytecode_result);

    let ctx_plan = make_ctx(data_len);
    let plan_result = run_plan(source, &ctx_plan);
    assert_arrays_match(formula_name, "plan", &reference, &plan_result);

    #[cfg(feature = "formula-jit")]
    {
        let mut ctx_jit = make_ctx(data_len);
        let jit_result = run_jit(&mut engine, source, &mut ctx_jit);
        assert_arrays_match(formula_name, "jit", &reference, &jit_result);
    }

    #[cfg(feature = "formula-simd")]
    {
        let mut ctx_simd = make_ctx(data_len);
        let simd_result = run_simd(&mut engine, source, &mut ctx_simd);
        assert_arrays_match(formula_name, "simd", &reference, &simd_result);
    }
}

const MACD: &str = r#"
    DIF := EMA(CLOSE, 12) - EMA(CLOSE, 26);
    DEA := EMA(DIF, 9);
    MACD := (DIF - DEA) * 2;
    MACD
"#;

const KDJ: &str = r#"
    RSV := (CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100;
    K := EMA(RSV, 3);
    D := EMA(K, 3);
    J := 3 * K - 2 * D;
    J
"#;

const BOLL: &str = r#"
    MID := MA(CLOSE, 20);
    STD_VAL := STD(CLOSE, 20);
    UPPER := MID + 2 * STD_VAL;
    LOWER := MID - 2 * STD_VAL;
    UPPER
"#;

const MA_SUM: &str = "MA(CLOSE, 5) + MA(CLOSE, 10)";

#[test]
fn formula_differential_macd_all_paths() {
    check_all_paths("MACD", MACD, 80);
}

#[test]
fn formula_differential_kdj_all_paths() {
    check_all_paths("KDJ", KDJ, 80);
}

#[test]
fn formula_differential_boll_all_paths() {
    check_all_paths("BOLL", BOLL, 80);
}

#[test]
fn formula_differential_ma_sum_all_paths() {
    check_all_paths("MA_SUM", MA_SUM, 80);
}

/// Newly added plan kernels must agree with the tree path, not merely execute.
///
/// `DIV` is the one worth a second look: the plan kernel guards `rhs == 0.0`
/// (matching `fn_div`) while `BinaryKernel::Div` guards `rhs.abs() < 1e-15`.
/// Routing `CALL:DIV` through the binary kernel would have agreed on ordinary
/// data and diverged only on a tiny-but-nonzero divisor, so that case is pinned
/// explicitly rather than left to luck.
#[test]
fn formula_differential_arithmetic_and_unary_math_kernels() {
    check_all_paths("ADD", "ADD(CLOSE, 2)", 80);
    check_all_paths("SUB", "SUB(CLOSE, 2)", 80);
    check_all_paths("MULT", "MULT(CLOSE, 2)", 80);
    check_all_paths("DIV", "DIV(CLOSE, 2)", 80);
    // Tiny but non-zero: `fn_div` yields a huge finite number, whereas
    // `BinaryKernel::Div`'s epsilon guard would yield NaN.
    check_all_paths("DIV_TINY", "DIV(CLOSE, 0.00000000000000000001)", 80);
    check_all_paths("SQRT", "SQRT(CLOSE)", 80);
    check_all_paths("SINH", "SINH(CLOSE / 100)", 80);
    check_all_paths("COSH", "COSH(CLOSE / 100)", 80);
    check_all_paths("TANH", "TANH(CLOSE / 100)", 80);
}

/// Window kernels, in both warm-up families.
///
/// `MINUS` and `*INDEX` need a full window before they can answer, so the first
/// bars are NaN; `HHVBARS`/`LLVBARS` start from a saturating window and are
/// defined at bar 0. The split is the point: a kernel that got the warm-up rule
/// wrong would not fail, it would quietly shift or extend the series, so each
/// family has to be pinned separately.
#[test]
fn formula_differential_window_kernels() {
    check_all_paths("MINUS", "MINUS(CLOSE, 3)", 80);
    check_all_paths("MAXINDEX", "MAXINDEX(CLOSE, 5)", 80);
    check_all_paths("MININDEX", "MININDEX(HIGH, 5)", 80);
    check_all_paths("HHVBARS", "HHVBARS(HIGH, 5)", 80);
    check_all_paths("LLVBARS", "LLVBARS(LOW, 5)", 80);

    // Composed, so the window reads a computed series rather than a bound input.
    check_all_paths("MINUS_OF_MA", "MINUS(MA(CLOSE, 3), 2)", 80);
    check_all_paths("HHVBARS_OF_HHV", "HHVBARS(HHV(HIGH, 3), 5)", 80);
}

/// Element-wise kernels with no period: `CROSS`, `FIXNAN` and `STDDEV`.
///
/// The interesting cases are the ones where a plausible implementation would
/// disagree with `fn_*` rather than fail: `FIXNAN` must keep leading NaN (not
/// seed from the first finite value), and `CROSS` must emit `0.0` rather than
/// NaN when nothing crossed. `STDDEV` is an alias of `STD` on the formula
/// surface, so it is checked against the same reference and would catch the two
/// names being given different kernels.
#[test]
fn formula_differential_elementwise_and_stddev_kernels() {
    check_all_paths("CROSS", "CROSS(MA(CLOSE, 3), MA(CLOSE, 8))", 80);
    check_all_paths("CROSS_FLAT", "CROSS(CLOSE, CLOSE)", 80);
    check_all_paths("CROSSBELOW", "CROSSBELOW(MA(CLOSE, 3), MA(CLOSE, 8))", 80);
    check_all_paths("CROSSBELOW_FLAT", "CROSSBELOW(CLOSE, CLOSE)", 80);
    // Leading NaN: the first bars have no previous finite value to carry.
    check_all_paths("FIXNAN", "FIXNAN(IF(CLOSE > 105, CLOSE, 0/0))", 80);
    check_all_paths("FIXNAN_PLAIN", "FIXNAN(CLOSE)", 80);
    check_all_paths("STDDEV", "STDDEV(CLOSE, 6)", 80);
    check_all_paths("STDDEV_MATCHES_STD", "STDDEV(CLOSE, 6) - STD(CLOSE, 6)", 80);
}

/// `VAR` on the compiled-plan path must be the *population* variance.
///
/// This is the assertion that catches a subtle mistake: `VAR` looks like
/// `STD * STD`, and the two are the same up to a rounding step, so a kernel
/// built by squaring `stddev_into`'s output would pass a loose comparison while
/// being arithmetically different. Every other path (tree, bytecode, and the
/// streaming engine) uses the population convention via
/// `indicators::statistics::var` -> `math::rolling_stats::variance`, and the
/// kernel delegates to that same helper.
///
/// It also pins that `STD` and `VAR` are consistent with each other, since a
/// kernel could match `VAR` while breaking the `VAR == STD * STD` relation.
#[test]
fn formula_differential_variance_is_population_on_every_path() {
    check_all_paths("VAR", "VAR(CLOSE, 6)", 80);
    check_all_paths(
        "VAR_MATCHES_STD_SQUARED",
        "VAR(CLOSE, 6) - STD(CLOSE, 6) * STD(CLOSE, 6)",
        80,
    );
}

/// Compound assignment has to survive the effect-sequencing edge.
///
/// Every effect node gets the previous effect appended as an extra dependency,
/// so a `+=` that follows another statement carries three dependencies rather
/// than two. The plumbing rewrite used to require exactly two and rejected the
/// formula outright; the other wrong fix — forwarding the third entry as an
/// operand — would fail the kernel's arity check instead. Both are covered by
/// the chained case below, which is what makes the edge appear repeatedly.
#[test]
fn formula_differential_compound_assignment_all_paths() {
    check_all_paths("COMPOUND_ADD", "X := CLOSE; X += 1; X", 80);
    check_all_paths(
        "COMPOUND_CHAINED",
        "X := CLOSE; X += CLOSE; X += 2; X * 2",
        80,
    );
    check_all_paths(
        "COMPOUND_ALL_OPS",
        "X := CLOSE; X -= 1; X *= 2; X /= 3; X",
        80,
    );
}

// --- Regression cases for plan-path defects found by this harness -------------

/// The same node appearing twice as an operand must stay twice.
///
/// `ComputePlan::compile` used to deduplicate stored dependencies, which turned
/// `CLOSE + CLOSE` into a one-operand addition and failed the kernel arity check.
#[test]
fn formula_differential_duplicate_operand_all_paths() {
    check_all_paths("CLOSE_PLUS_CLOSE", "CLOSE + CLOSE", 80);
}

/// Non-commutative operands must not be reordered by compilation.
///
/// Dependency sorting used to invert this division, producing `MA/CLOSE`
/// instead of `CLOSE/MA` — a silent numeric error, not a failure.
#[test]
fn formula_differential_non_commutative_operand_order_all_paths() {
    check_all_paths("CLOSE_OVER_MA", "CLOSE / MA(CLOSE, 6)", 80);
    check_all_paths("MA_OVER_CLOSE", "MA(CLOSE, 6) / CLOSE", 80);
}

/// Multi-statement formulas exercise plumbing resolution (assignments and local
/// variable reads) which the numeric dispatcher has no kernels for.
#[test]
fn formula_differential_multi_statement_plumbing_all_paths() {
    check_all_paths(
        "BIAS3",
        "BIAS1 := (CLOSE - MA(CLOSE,6)) / MA(CLOSE,6) * 100; \
         BIAS2 := (CLOSE - MA(CLOSE,12)) / MA(CLOSE,12) * 100; \
         BIAS3 := (CLOSE - MA(CLOSE,24)) / MA(CLOSE,24) * 100; \
         BIAS3",
        120,
    );
}

#[test]
fn formula_differential_rolling_extrema_all_paths() {
    check_all_paths(
        "HHV_LLV",
        "(CLOSE - LLV(LOW, 9)) / (HHV(HIGH, 9) - LLV(LOW, 9)) * 100",
        80,
    );
}

#[test]
fn formula_differential_elementwise_helpers_all_paths() {
    check_all_paths("ABS", "ABS(CLOSE - MA(CLOSE, 5))", 80);
    check_all_paths("SUM", "SUM(CLOSE, 5)", 80);
    check_all_paths("REF", "CLOSE - REF(CLOSE, 3)", 80);
    check_all_paths(
        "MAX_MIN",
        "MAX(CLOSE, MA(CLOSE,5)) - MIN(CLOSE, MA(CLOSE,5))",
        80,
    );
}

/// A trailing drawing directive must not become the formula's result.
///
/// The tree-walker used to return the last statement's buffer unconditionally,
/// so a formula ending in `DRAWICON(...)` reported that directive's scratch
/// buffer — which is always zeroed — instead of the series computed above it.
/// The bytecode VM never had this bug (its draw opcodes pop their operands and
/// push nothing), so the tree-walker was the outlier; this test pins the two
/// together and anchors the shared rule in `AstNode::produces_value`.
#[test]
fn formula_differential_trailing_draw_directive_all_paths() {
    const SOURCE: &str = "MA5 := MA(CLOSE, 5); DRAWICON(CLOSE > MA5, HIGH, 1);";

    let mut engine = FormulaEngine::new();
    let reference = run_ast(&mut engine, SOURCE, &mut make_ctx(80));

    let bytecode_result = run_bytecode(&mut engine, SOURCE, &make_ctx(80));
    assert_arrays_match("DRAWICON_TAIL", "bytecode", &reference, &bytecode_result);

    // The result is the assignment above the directive, not a zeroed buffer.
    let ma5 = run_ast(&mut engine, "MA(CLOSE, 5)", &mut make_ctx(80));
    assert_arrays_match("DRAWICON_TAIL", "MA5", &ma5, &reference);

    // The plan path cannot execute this yet, and that is deliberate: drawing
    // directives are retained as roots so their side effects are never silently
    // dropped, while the numeric dispatcher has no `DRAW_ICON` kernel. When a
    // real drawing sink lands, replace this with a plan comparison.
    let ast = parse_formula(SOURCE).expect("plan parse failed");
    let plan = FormulaHotPlan::compile(&ast).expect("plan compile failed");
    assert!(
        plan.hot()
            .nodes()
            .iter()
            .any(|node| node.kernel == KernelId::from_static("DRAW_ICON")),
        "a drawing directive must be retained as a root, not dropped by dead-code elimination"
    );
}
