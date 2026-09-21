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
    // Bind every slot the plan declares, resolving each name through the same
    // `get_data` the tree path uses. A hard-coded alias list would miss names
    // like `VOL` (whose slot is `VARIABLE:VOL`, not `VARIABLE:VOLUME`) and leave
    // them bound to the CLOSE default, which turns a harness gap into a
    // phantom kernel divergence.
    for (operation, slot) in plan.hot().input_layout().operations() {
        let Some(name) = operation.strip_prefix("VARIABLE:") else {
            continue;
        };
        if let Some(values) = ctx.get_data(name) {
            inputs[slot.0] = values;
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

/// `HMA` is a three-pass WMA with no `_into` helper, so its kernel delegates to
/// `math::moving_avg::hma` — the same function the tree path calls.
///
/// The case worth pinning is the warm-up: Hull MA's first finite value is at
/// `period + round(sqrt(period)) - 2`, not at `period - 1` like a plain WMA. A
/// kernel that reused `wma_into` semantics would agree in the interior and be
/// wrong for the first few bars, which is exactly the kind of difference a
/// "does it run" check misses.
#[test]
fn formula_differential_hma_kernel() {
    check_all_paths("HMA", "HMA(CLOSE, 9)", 80);
    check_all_paths("HMA_SHORT", "HMA(CLOSE, 4)", 80);
    // Composed: the input is a computed series rather than a bound one.
    check_all_paths("HMA_OF_MA", "HMA(MA(CLOSE, 3), 9)", 80);
}

/// `CORREL` is a two-series rolling kernel with the period last, like `VWMA`.
///
/// It delegates to `rolling_correlation_into`, the same helper `fn_correl`
/// calls, so the plan path cannot pick a different correlation convention (e.g.
/// sample vs population, or a different NaN policy) than the tree path.
#[test]
fn formula_differential_correl_kernel() {
    check_all_paths("CORREL", "CORREL(CLOSE, OPEN, 5)", 80);
    check_all_paths("CORREL_SELF", "CORREL(CLOSE, CLOSE, 5)", 80);
    // Reversed operands: Pearson correlation is symmetric, so a kernel that
    // swapped its inputs would still pass this one — the two cases together
    // pin the argument order rather than just the result.
    check_all_paths("CORREL_SWAPPED", "CORREL(OPEN, CLOSE, 5)", 80);
    check_all_paths("CORREL_OF_MA", "CORREL(MA(CLOSE, 3), OPEN, 5)", 80);
}

/// `ISNA` is a predicate, not a transform: it must report gaps as `1.0`
/// rather than propagating them, or Pine's `na(x)` and `nz(x, y)` invert.
#[test]
fn formula_differential_isna_kernel() {
    // No gap at all — the trivial branch, which a kernel that returned NaN
    // everywhere would still "pass" if this were the only case.
    check_all_paths("ISNA_PLAIN", "ISNA(CLOSE)", 80);
    // `REF` leaves a NaN prefix, so the boundary between gap and non-gap is
    // exercised rather than just the two constant regimes.
    check_all_paths("ISNA_REF", "ISNA(REF(CLOSE, 3))", 80);
    // Entirely NaN. `SQRT` of a negative is NaN on every path, which reaches
    // the all-gap regime without asking for a period longer than the series —
    // that combination is a *separate*, pre-existing divergence recorded in
    // `docs/refactor-plan-2026-09-21.md` (tree path returns NaN, plan path
    // fails the kernel with `ERR_PARAMETER`), and mixing it in here would make
    // an ISNA failure indistinguishable from that one.
    check_all_paths("ISNA_ALL_NAN", "ISNA(SQRT(0 - CLOSE))", 80);
    // The composed form, which is what the Pine mapper actually emits.
    check_all_paths("ISNA_IN_IF", "IF(ISNA(REF(CLOSE, 3)), 7, CLOSE)", 80);
}

/// `TRANGE` has no period operand, so it cannot ride the HLC periodic family.
///
/// It also pins a shadowed-implementation trap: the router overrides the legacy
/// `fn_trange` with `volatility::trange`, and the two disagree on bar 0 (NaN vs
/// `high[0] - low[0]`). A kernel that copied the legacy body would diverge here
/// on exactly one index.
#[test]
fn formula_differential_trange_kernel() {
    check_all_paths("TRANGE", "TRANGE(HIGH, LOW, CLOSE)", 80);
    // Reversed and self-referential operands, to pin the argument order rather
    // than only the value: `TRANGE(H, L, C)` is not symmetric in any pair.
    check_all_paths("TRANGE_REORDERED", "TRANGE(HIGH, CLOSE, LOW)", 80);
    check_all_paths("TRANGE_SELF", "TRANGE(HIGH, HIGH, HIGH)", 80);
    check_all_paths("TRANGE_OF_MA", "TRANGE(MA(HIGH, 3), LOW, CLOSE)", 80);
}

/// `CUMSUM` accumulates from bar 0, and a NaN contributes nothing rather than
/// poisoning the rest of the prefix.
#[test]
fn formula_differential_cumsum_kernel() {
    check_all_paths("CUMSUM", "CUMSUM(CLOSE)", 80);
    // A NaN prefix must not zero or NaN the whole result.
    check_all_paths("CUMSUM_NAN_PREFIX", "CUMSUM(REF(CLOSE, 5))", 80);
    check_all_paths("CUMSUM_OF_ROC", "CUMSUM(ROC(CLOSE, 1))", 80);
}

/// `BARSSINCE` is NaN until the first truthy bar, then counts bars since it.
#[test]
fn formula_differential_barssince_kernel() {
    // Never true: every bar stays NaN, which is the case a zero-initialised
    // accumulator would get wrong. The literal is written out because the
    // grammar has no scientific-notation suffix.
    check_all_paths("BARSSINCE_NEVER", "BARSSINCE(CLOSE > 1000000000000)", 80);
    // True on a known bar, so the count-up and the leading NaN both appear.
    check_all_paths("BARSSINCE_LATE", "BARSSINCE(CLOSE > 120)", 80);
    // A NaN condition is not truthy, matching `fn_barssince`.
    check_all_paths("BARSSINCE_NAN_COND", "BARSSINCE(REF(CLOSE, 3) > 120)", 80);
    check_all_paths(
        "BARSSINCE_CROSS",
        "BARSSINCE(CROSS(CLOSE, MA(CLOSE, 5)))",
        80,
    );
}

/// `RMA` is Wilder's smoothing with a NaN-skipping seed.
///
/// The NaN cases are the point: `indicators::talib_ext::rma_profile` implements
/// the same recurrence but seeds from `input[..period]` unconditionally, so it
/// produces a fully-NaN series where this must produce a live one.
#[test]
fn formula_differential_rma_kernel() {
    check_all_paths("RMA", "RMA(CLOSE, 5)", 80);
    check_all_paths("RMA_14", "RMA(CLOSE, 14)", 80);
    check_all_paths("RMA_NAN_PREFIX", "RMA(REF(CLOSE, 5), 5)", 80);
    check_all_paths("RMA_OF_ROC", "RMA(ROC(CLOSE, 1), 5)", 80);
}

/// `MEDIAN` drops NaN before taking the window median, and averages the two
/// middle values on an even-sized window.
#[test]
fn formula_differential_median_kernel() {
    check_all_paths("MEDIAN_ODD", "MEDIAN(CLOSE, 5)", 80);
    // Even window: the average-the-two-middles branch.
    check_all_paths("MEDIAN_EVEN", "MEDIAN(CLOSE, 4)", 80);
    // Window 1 degenerates to the identity, which a broken warm-up would miss.
    check_all_paths("MEDIAN_ONE", "MEDIAN(CLOSE, 1)", 80);
    check_all_paths("MEDIAN_NAN_PREFIX", "MEDIAN(REF(CLOSE, 3), 5)", 80);
}

/// `ROLLING_RANGE` is `rolling_max - rolling_min` over the same window, so it
/// shares their NaN policy instead of inventing a third one.
#[test]
fn formula_differential_rolling_range_kernel() {
    check_all_paths("ROLLING_RANGE", "ROLLING_RANGE(CLOSE, 5)", 80);
    check_all_paths("ROLLING_RANGE_HIGH", "ROLLING_RANGE(HIGH, 3)", 80);
    check_all_paths(
        "ROLLING_RANGE_NAN_PREFIX",
        "ROLLING_RANGE(REF(CLOSE, 4), 3)",
        80,
    );
}

/// Known, **recorded** divergence: a period longer than the series.
///
/// The tree path's `canonical_*` wrappers swallow a kernel error and return an
/// all-NaN series, so `MA(CLOSE, 100)` over 80 bars evaluates to NaN. The
/// compiled-plan path propagates the underlying `InsufficientData` as
/// `ERR_PARAMETER` and fails the execution instead.
///
/// This is pre-existing — it predates the kernel-coverage work and no corpus
/// case asks for a period longer than its data, which is why the differential
/// gate never saw it. It is pinned rather than fixed so that (a) the divergence
/// cannot drift unnoticed, and (b) promoting the plan path to the default fails
/// *here*, forcing the dispatcher's error policy to be decided deliberately
/// instead of silently turning a NaN result into a hard failure.
#[test]
fn out_of_range_period_is_a_recorded_divergence() {
    use finkit::formula::FormulaExecutionMode;

    let mut tree = FormulaEngine::new();
    let mut tree_ctx = make_ctx(80);
    let reference = tree
        .eval("MA(CLOSE, 100)", &mut tree_ctx)
        .expect("the tree path must tolerate an out-of-range period");
    assert!(
        reference.iter().all(|value| value.is_nan()),
        "the tree path must swallow the insufficient-data error and yield NaN, \
         got {:?}",
        &reference.as_slice().unwrap()[..5.min(reference.len())]
    );

    let mut plan = FormulaEngine::new().with_execution_mode(FormulaExecutionMode::Plan);
    let mut plan_ctx = make_ctx(80);
    let outcome = plan.eval("MA(CLOSE, 100)", &mut plan_ctx);
    assert!(
        outcome.is_err(),
        "the plan path now agrees with the tree path on an out-of-range period, \
         so this divergence is closed: delete this test and update \
         docs/refactor-plan-2026-09-21.md"
    );
}

/// `MOD` must follow the *function*, not the `%` operator.
///
/// `fn_mod` uses Rust's truncating remainder and yields NaN for a near-zero
/// divisor; `BINARY:Mod` uses a floor-based remainder. They disagree in sign for
/// negative operands, so the negative case here is what pins the choice.
#[test]
fn formula_differential_mod_kernel() {
    check_all_paths("MOD", "MOD(CLOSE, 3)", 80);
    check_all_paths("MOD_NEGATIVE_LHS", "MOD(0 - CLOSE, 3)", 80);
    check_all_paths("MOD_NEGATIVE_RHS", "MOD(CLOSE, 0 - 3)", 80);
    // The near-zero guard: the function returns NaN where the operator would
    // divide by zero.
    check_all_paths("MOD_ZERO_DIVISOR", "MOD(CLOSE, 0)", 80);
}

/// `INTPART` truncates toward zero and `FRACPART` keeps the sign, so the two
/// recombine into the original value on both sides of zero.
#[test]
fn formula_differential_intpart_fracpart_kernel() {
    check_all_paths("INTPART", "INTPART(CLOSE)", 80);
    check_all_paths("FRACPART", "FRACPART(CLOSE)", 80);
    check_all_paths("INTPART_NEGATIVE", "INTPART(0 - CLOSE)", 80);
    check_all_paths("FRACPART_NEGATIVE", "FRACPART(0 - CLOSE)", 80);
    // `INTPART(X) + FRACPART(X) == X` — the identity that makes truncation
    // toward zero (rather than `floor`) observable.
    check_all_paths(
        "INTPART_FRACPART_IDENTITY",
        "INTPART(0 - CLOSE) + FRACPART(0 - CLOSE) - (0 - CLOSE)",
        80,
    );
}

/// `REVERSE` mirrors the series, so applying it twice must be the identity.
#[test]
fn formula_differential_reverse_kernel() {
    check_all_paths("REVERSE", "REVERSE(CLOSE)", 80);
    check_all_paths("REVERSE_TWICE", "REVERSE(REVERSE(CLOSE)) - CLOSE", 80);
    check_all_paths("REVERSE_OF_MA", "REVERSE(MA(CLOSE, 5))", 80);
}

/// `SUMBARS` scans backwards to a per-bar threshold, so it needs a case where
/// the threshold is never reached as well as one where it is.
#[test]
fn formula_differential_sumbars_kernel() {
    check_all_paths("SUMBARS", "SUMBARS(VOL, 5000)", 80);
    check_all_paths("SUMBARS_SMALL", "SUMBARS(VOL, 100)", 80);
    // Never reached: the answer is the distance to the start of the series, not
    // NaN, which is the branch a NaN-initialised accumulator would get wrong.
    check_all_paths("SUMBARS_UNREACHED", "SUMBARS(CLOSE, 1000000)", 80);
    check_all_paths("SUMBARS_OF_ROC", "SUMBARS(ROC(CLOSE, 1), 0.5)", 80);
}

/// Every `DrawGeneric` command lowers to the same `DRAW_GENERIC` operation.
///
/// The plan path used to emit `DRAW:<command>`, which needed one kernel per
/// command; the dispatcher enumerated four names by hand and eleven real
/// commands — `DRAWLINE` first among them — fell through unhandled, so any
/// formula drawing a line failed under the compiled plan. These two cases are
/// different commands from that set, so they fail if the gate ever regresses to
/// per-command names.
#[test]
fn formula_differential_generic_draw_kernel() {
    check_all_paths(
        "DRAWLINE",
        "MA5 := MA(CLOSE, 5);\n\
         DRAWLINE(CROSS(CLOSE, MA5), CLOSE, CROSS(MA5, CLOSE), MA5, 1);\n\
         MA5",
        80,
    );
    check_all_paths("DRAWKLINE", "DRAWKLINE(HIGH, OPEN, LOW, CLOSE);\nCLOSE", 80);
}

/// Both execution modes must leave the same observable state on the context.
///
/// The tree path publishes every `ASSIGN:`/`OUTPUT:`/`COMPOUND:` write into
/// `ctx.variables` and every declared channel into `ctx.output_names`; the
/// language bindings read those two fields back to build their result
/// dictionaries. A plan path that returned only its own channel list would make
/// switching modes silently drop named results — a difference no numeric
/// comparison of the primary series would catch.
#[test]
fn formula_differential_both_modes_publish_the_same_context() {
    use finkit::formula::FormulaExecutionMode;

    // A plain assignment, an assignment that depends on it, a compound write and
    // a declared output: all four write shapes the tree path publishes.
    //
    // The *output* name is deliberately lower case. Operation strings inside the
    // plan are canonicalised (upper-cased) so that `VARIABLE:` reads resolve,
    // while `ctx.output_names` is keyed by the name as written and the parser
    // preserves case. An upper-case-only test cannot tell the two apart, and
    // publishing `GOLDEN` where the tree published `golden` would be a silent
    // cross-mode difference in the result dictionaries the bindings build.
    //
    // Variables stay upper case on purpose: the tree path resolves a variable
    // read through the canonical name but stores the write under the source
    // spelling, so a lower-case *variable* already fails on the reference path
    // (`Unknown variable: MA5`). That is a pre-existing limitation of the tree
    // path, not something this test should paper over.
    let source = "MA5 := MA(CLOSE, 5); MA10 := MA(CLOSE, 10); \
                  TALLY := MA5 - MA10; TALLY += 1; golden: CROSS(MA5, MA10)";

    let mut tree_ctx = make_ctx(80);
    let mut plan_ctx = make_ctx(80);

    let tree = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Tree)
        .eval(source, &mut tree_ctx)
        .expect("tree path evaluates");
    let plan = FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval(source, &mut plan_ctx)
        .expect("plan path evaluates");

    assert_arrays_match("both_modes_publish", "plan", &tree, &plan);

    let mut tree_names: Vec<&str> = tree_ctx.variables.keys().map(|k| k.as_ref()).collect();
    let mut plan_names: Vec<&str> = plan_ctx.variables.keys().map(|k| k.as_ref()).collect();
    tree_names.sort_unstable();
    plan_names.sort_unstable();
    assert_eq!(
        tree_names, plan_names,
        "the two modes published different variable names"
    );
    assert!(
        tree_names.contains(&"MA5") && tree_names.contains(&"TALLY"),
        "the reference path stopped publishing intermediate assignments, so this \
         test no longer proves anything: {tree_names:?}"
    );

    for name in &tree_names {
        let reference = tree_ctx
            .variables
            .get(*name)
            .expect("name came from the tree context");
        let candidate = plan_ctx
            .variables
            .get(*name)
            .unwrap_or_else(|| panic!("plan path did not publish `{name}`"));
        assert_arrays_match(name, "plan", reference, candidate);
    }

    assert_eq!(
        tree_ctx.output_names, plan_ctx.output_names,
        "the two modes disagree on the declared output channels"
    );
    assert_eq!(tree_ctx.output_names, vec!["golden"]);
}

/// Chart styling is the last piece of context state both paths must agree on.
///
/// `ctx.output_modifiers` is what the FFI layer reads to describe a series'
/// colour and line style, so a plan path that skipped it would hand back a chart
/// with every series unstyled — invisible to any numeric comparison.
///
/// `OutputModifier` has no `PartialEq` and `ctx.output_modifiers` is a
/// `HashMap`, so the maps are compared as *sorted* `(name, debug)` pairs rather
/// than through their own rendering, whose order is hash-dependent.
#[test]
fn formula_differential_both_modes_publish_the_same_output_modifiers() {
    use finkit::formula::FormulaExecutionMode;

    let source = "FAST: MA(CLOSE, 5), COLORRED, LINETHICK2; \
                  SLOW: MA(CLOSE, 10), COLORGREEN";

    let mut tree_ctx = make_ctx(80);
    let mut plan_ctx = make_ctx(80);

    FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Tree)
        .eval(source, &mut tree_ctx)
        .expect("tree path evaluates");
    FormulaEngine::new()
        .with_execution_mode(FormulaExecutionMode::Plan)
        .eval(source, &mut plan_ctx)
        .expect("plan path evaluates");

    let collect = |ctx: &FormulaContext| {
        let mut entries: Vec<(String, String)> = ctx
            .output_modifiers
            .iter()
            .map(|(name, modifier)| (name.clone(), format!("{modifier:?}")))
            .collect();
        entries.sort();
        entries
    };
    let reference = collect(&tree_ctx);
    let candidate = collect(&plan_ctx);

    let rendered = format!("{reference:?}");
    assert!(
        rendered.contains("COLORRED") && rendered.contains("LineStyle"),
        "the reference path stopped publishing modifiers, so this test no longer \
         proves anything: {rendered}"
    );
    assert_eq!(
        reference, candidate,
        "the two modes disagree on the output modifiers"
    );
    assert_eq!(tree_ctx.output_names, plan_ctx.output_names);
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
