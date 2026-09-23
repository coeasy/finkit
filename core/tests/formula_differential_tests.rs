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

/// Like [`check_all_paths`], but with an upstream rolling output bound as the
/// variable `X` — the composition scenario the warm-up contract is about.
///
/// [`check_all_paths`] builds a clean context and **never binds a variable**, so
/// it structurally cannot see this class: before the round-10..12 fixes every
/// path returned all-`NaN` for a warm-up input and the gate stayed green. This
/// variant also asserts an **absolute** finite-value count, because agreement
/// between five paths that are all all-`NaN` is not evidence of anything.
fn check_all_paths_with_warm_variable(
    formula_name: &str,
    source: &str,
    warm: &Array1<f64>,
    expected_finite: usize,
) {
    const LEN: usize = 200;
    let mut engine = FormulaEngine::new();

    let mut ctx_ast = make_ctx(LEN);
    ctx_ast.set_variable("X".to_string(), warm.clone());
    let reference = run_ast(&mut engine, source, &mut ctx_ast);

    let finite = reference.iter().filter(|value| value.is_finite()).count();
    assert_eq!(
        finite, expected_finite,
        "{formula_name}: ast produced {finite} finite values, expected \
         {expected_finite} (0 means the warm-up prefix poisoned the accumulator)"
    );

    let mut ctx_bc = make_ctx(LEN);
    ctx_bc.set_variable("X".to_string(), warm.clone());
    let bytecode_result = run_bytecode(&mut engine, source, &ctx_bc);
    assert_arrays_match(formula_name, "bytecode", &reference, &bytecode_result);

    let mut ctx_plan = make_ctx(LEN);
    ctx_plan.set_variable("X".to_string(), warm.clone());
    let plan_result = run_plan(source, &ctx_plan);
    assert_arrays_match(formula_name, "plan", &reference, &plan_result);

    #[cfg(feature = "formula-jit")]
    {
        let mut ctx_jit = make_ctx(LEN);
        ctx_jit.set_variable("X".to_string(), warm.clone());
        let jit_result = run_jit(&mut engine, source, &mut ctx_jit);
        assert_arrays_match(formula_name, "jit", &reference, &jit_result);
    }

    #[cfg(feature = "formula-simd")]
    {
        let mut ctx_simd = make_ctx(LEN);
        ctx_simd.set_variable("X".to_string(), warm.clone());
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

/// Context-implicit price arguments must be expanded before lowering.
///
/// `PLUS_DI(CLOSE, 14)` and `AROON_UP(14)` name only their period: the tree
/// path fills `HIGH`/`LOW`/`CLOSE` in from the evaluation context via
/// `resolve_hlc_args` / `resolve_hl_args`. The compiled plan has no such
/// fallback — its input layout only carries series the source text names — so
/// without `expand_implicit_price_args` these calls reach the dispatcher one
/// operand short and are rejected with `ERR_ARITY`. That is why the failure
/// this test pins is an *execution* error on the plan side rather than a
/// numeric mismatch: nothing diverges, the call simply refuses to run.
///
/// The explicit spellings are checked too. They are the same computation, so
/// agreeing with the short form is the property that proves the expansion is
/// value-preserving rather than merely executable.
#[test]
fn formula_differential_implicit_price_arguments() {
    check_all_paths("PLUS_DI_SHORT", "PLUS_DI(CLOSE, 14)", 80);
    check_all_paths("MINUS_DI_SHORT", "MINUS_DI(CLOSE, 14)", 80);
    check_all_paths("AROON_UP_SHORT", "AROON_UP(14)", 80);
    check_all_paths("AROON_DN_SHORT", "AROON_DN(14)", 80);
    check_all_paths("PLUS_DI_EXPLICIT", "PLUS_DI(HIGH, LOW, CLOSE, 14)", 80);
    check_all_paths("MINUS_DI_EXPLICIT", "MINUS_DI(HIGH, LOW, CLOSE, 14)", 80);
    check_all_paths("AROON_UP_EXPLICIT", "AROON_UP(HIGH, LOW, 14)", 80);
    check_all_paths("AROON_DN_EXPLICIT", "AROON_DN(HIGH, LOW, 14)", 80);
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

/// Number of bars used by the rolling-composition cases below. The expected
/// finite counts are derived from this length, so it is a named constant.
const COMPOSITION_LEN: usize = 200;

/// Rolling indicators must survive composition through variables.
///
/// Every rolling kernel seeds an incremental accumulator from its first window,
/// and `NaN` is absorbing for such an accumulator (`NaN - NaN` is still `NaN`).
/// Because every rolling indicator in this crate marks its warm-up with a
/// *leading* `NaN` run, `MA(MA(CLOSE, 5), 9)` used to return an all-`NaN`
/// series -- and so did `DEA := EMA(DIF, 9)`, i.e. the canonical MACD.
///
/// The gates in this file could not see it: they only assert that the paths
/// *agree*, and `values_match` treats `NaN`/`NaN` as a match, so two paths that
/// were wrong in exactly the same way looked like success. These cases therefore
/// pin the finite-value count as well as the agreement -- the count is what
/// makes the regression detectable at all.
#[test]
fn formula_differential_rolling_composition() {
    // (name, source, expected finite values on the 200-bar synthetic series)
    let cases: &[(&str, &str, usize)] = &[
        ("MA_OF_MA", "X:=MA(CLOSE,5); Y:=MA(X,9); Y", 188),
        ("EMA_OF_MA", "X:=MA(CLOSE,5); Y:=EMA(X,9); Y", 188),
        ("EMA_OF_EMA", "X:=EMA(CLOSE,5); Y:=EMA(X,9); Y", 188),
        ("WMA_OF_MA", "X:=MA(CLOSE,5); Y:=WMA(X,9); Y", 188),
        ("SUM_OF_MA", "X:=MA(CLOSE,5); Y:=SUM(X,9); Y", 188),
        ("STD_OF_MA", "X:=MA(CLOSE,5); Y:=STD(X,9); Y", 188),
        ("VAR_OF_MA", "X:=MA(CLOSE,5); Y:=VAR(X,9); Y", 188),
        ("HHV_OF_MA", "X:=MA(CLOSE,5); Y:=HHV(X,9); Y", 188),
        ("LLV_OF_MA", "X:=MA(CLOSE,5); Y:=LLV(X,9); Y", 188),
        ("TRIMA_OF_MA", "X:=MA(CLOSE,5); Y:=TRIMA(X,9); Y", 188),
        ("TRIX_OF_MA", "X:=MA(CLOSE,5); Y:=TRIX(X,9); Y", 171),
        ("RSI_OF_MA", "X:=MA(CLOSE,5); Y:=RSI(X,9); Y", 191),
        ("MA_OF_REF", "X:=REF(CLOSE,10); Y:=MA(X,9); Y", 182),
        (
            "MACD_COMPOSED",
            "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); (DIF-DEA)*2",
            167,
        ),
        ("BOLL_OF_MA", "BOLL(MA(CLOSE,5),20,2)", 177),
        ("ATR_OF_MA", "ATR(MA(HIGH,5),MA(LOW,5),MA(CLOSE,5),9)", 187),
    ];

    for (name, source, expected_finite) in cases {
        check_all_paths(name, source, COMPOSITION_LEN);

        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(COMPOSITION_LEN);
        let series = run_ast(&mut engine, source, &mut ctx);
        let finite = series.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, *expected_finite,
            "formula {name}: expected {expected_finite} finite values out of {COMPOSITION_LEN}, \
             got {finite} -- the inner indicator's leading NaN warm-up run was not skipped"
        );
    }
}

/// Composite indicators applied to a rolling input, pinned on the tree path.
///
/// `DEMA`, `TEMA`, `MACD`, `APO` and `STOCH` reject a non-finite input outright
/// and seed their internal recursions from `input[0]`, so feeding them the output
/// of another rolling indicator used to produce an all-`NaN` series as well.
///
/// These run through `run_ast` only: the compiled plan has no kernel for
/// `DEMA`/`TEMA`/`MACD(<expr>, ...)` yet, so `check_all_paths` cannot drive them
/// (it would fail to compile the plan rather than compare values). Move each case
/// up into `formula_differential_rolling_composition` as its kernel lands.
#[test]
fn formula_rolling_composition_of_composites() {
    let cases: &[(&str, &str, usize)] = &[
        ("DEMA_OF_MA", "X:=MA(CLOSE,5); Y:=DEMA(X,9); Y", 180),
        ("TEMA_OF_MA", "X:=MA(CLOSE,5); Y:=TEMA(X,9); Y", 172),
        ("MACD_OF_MA", "MACD(MA(CLOSE,5),12,26,9)", 163),
        ("APO_OF_MA", "APO(MA(CLOSE,5),12,26)", 171),
        (
            "STOCH_OF_MA",
            "STOCH(MA(HIGH,5),MA(LOW,5),MA(CLOSE,5),9,3,3)",
            188,
        ),
    ];

    for (name, source, expected_finite) in cases {
        let mut engine = FormulaEngine::new();
        let mut ctx = make_ctx(COMPOSITION_LEN);
        let series = run_ast(&mut engine, source, &mut ctx);
        let finite = series.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, *expected_finite,
            "formula {name}: expected {expected_finite} finite values out of {COMPOSITION_LEN}, \
             got {finite} -- the inner indicator's leading NaN warm-up run was not skipped"
        );
    }
}

/// The `try_execute_simple_formula*` fast paths must not answer for a name they
/// do not specialise, and must not answer all-`NaN` for a *leading* NaN run.
///
/// Regression for a divergence that the rolling-composition fix exposed. Those
/// helpers guard with `input.iter().any(|value| !value.is_finite())` and used to
/// do so **before** matching the function name, so any formula whose root was
/// `NAME(<variable>, <literal>)` was answered with an all-`NaN` series whenever
/// the input carried a warm-up run -- including names such as `SUM`, `HHV` and
/// `WMA` that the fast path never supported. That was invisible while the
/// general executor produced all-`NaN` for the same input too; once the math
/// layer learned to compute through a warm-up run it became a live
/// fast-path-vs-plan divergence.
///
/// The `+0` case matters: the optimizer folds the identity away, so the root is
/// the call again and the structural match still fires.
#[test]
fn formula_differential_simple_formula_fast_path_defers() {
    const LEN: usize = 200;

    // A rolling output: 4 leading NaNs, 196 finite values.
    let mut engine = FormulaEngine::new();
    let mut seed_ctx = make_ctx(LEN);
    let warm = run_ast(&mut engine, "MA(CLOSE,5)", &mut seed_ctx);
    assert_eq!(warm.iter().filter(|value| value.is_finite()).count(), 196);

    let cases: &[(&str, usize)] = &[
        ("MA(X,9)", 188),
        ("EMA(X,9)", 188),
        ("RSI(X,9)", 191),
        ("BOLLMID(X,9)", 188),
        ("SUM(X,9)", 188),
        ("HHV(X,9)", 188),
        ("LLV(X,9)", 188),
        ("STD(X,9)", 188),
        ("WMA(X,9)", 188),
        ("SUM(X,9)+0", 188),
        // Controls: these never matched the fast path's structure.
        ("MA(REF(X,3),9)", 185),
        ("SUM(MA(CLOSE,5),9)", 188),
        ("MA(MA(CLOSE,5),9)", 188),
        ("X2:=MA(X,9); X2", 188),
        ("Y:=SUM(X,9); Y", 188),
    ];

    for (source, expected_finite) in cases {
        let mut ctx_fast = make_ctx(LEN);
        ctx_fast.set_variable("X".to_string(), warm.clone());
        let fast = run_ast(&mut engine, source, &mut ctx_fast);

        let mut ctx_plan = make_ctx(LEN);
        ctx_plan.set_variable("X".to_string(), warm.clone());
        let plan = run_plan(source, &ctx_plan);
        assert_arrays_match(source, "plan", &fast, &plan);

        let finite = fast.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, *expected_finite,
            "{source}: got {finite} finite values, expected {expected_finite} \
             (0 means a simple-formula fast path answered all-NaN instead of deferring)"
        );
    }
}

/// The other two "simple formula" fast paths are reached through different
/// public entry points than `eval`: `try_execute_simple_formula_into` through
/// `eval_into`, and `try_execute_simple_formula_slices` through
/// `eval_zero_copy_inputs`. They carried the same defect and were fixed
/// alongside it, so they need their own coverage -- a fix applied to one entry
/// point is exactly how the previous round created a fresh divergence.
///
/// `eval_zero_copy_inputs` cannot be handed a bound variable, so it is driven
/// with OHLCV that itself begins with a NaN run, as an incomplete or resampled
/// feed would be.
#[test]
fn formula_differential_fast_path_entry_points_defer() {
    const LEN: usize = 200;
    const WARM: usize = 4;

    let mut engine = FormulaEngine::new();

    // A rolling output: 4 leading NaNs, 196 finite values.
    let mut seed_ctx = make_ctx(LEN);
    let warm = run_ast(&mut engine, "MA(CLOSE,5)", &mut seed_ctx);
    assert_eq!(warm.iter().filter(|value| value.is_finite()).count(), 196);

    // ---- eval_into ---------------------------------------------------------
    for (source, expected_finite) in [
        ("MA(X,9)", 188),
        ("SUM(X,9)", 188),
        ("HHV(X,9)", 188),
        ("WMA(X,9)", 188),
    ] {
        let formula = engine.compile(source).expect("compile failed");

        let mut ctx_into = make_ctx(LEN);
        ctx_into.set_variable("X".to_string(), warm.clone());
        let mut output = Array1::zeros(LEN);
        engine
            .eval_into(&formula, &mut ctx_into, &mut output)
            .expect("eval_into failed");

        let mut ctx_ref = make_ctx(LEN);
        ctx_ref.set_variable("X".to_string(), warm.clone());
        let reference = run_ast(&mut engine, source, &mut ctx_ref);
        assert_arrays_match(source, "eval", &output, &reference);

        let finite = output.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, expected_finite,
            "eval_into {source}: got {finite} finite values, expected {expected_finite}"
        );
    }

    // ---- eval_zero_copy_inputs --------------------------------------------
    let mut open = vec![f64::NAN; LEN];
    let mut high = vec![f64::NAN; LEN];
    let mut low = vec![f64::NAN; LEN];
    let mut close = vec![f64::NAN; LEN];
    let mut volume = vec![f64::NAN; LEN];
    for index in WARM..LEN {
        let x = index as f64;
        open[index] = 100.0 + x * 0.5;
        high[index] = 105.0 + x * 0.7;
        low[index] = 95.0 + x * 0.3;
        close[index] = 102.0 + x * 0.6;
        volume[index] = 10000.0 + x * 100.0;
    }

    for (source, expected_finite) in [
        ("MA(CLOSE,9)", 188),
        ("SUM(CLOSE,9)", 188),
        ("HHV(CLOSE,9)", 188),
        ("EMA(CLOSE,9)", 188),
        ("RSI(CLOSE,9)", 191),
    ] {
        let formula = engine.compile(source).expect("compile failed");
        let fast = engine
            .eval_zero_copy_inputs(&formula, &open, &high, &low, &close, &volume, None)
            .expect("eval_zero_copy_inputs failed");

        let mut ctx_ref = FormulaContext::new(
            Array1::from_vec(open.clone()),
            Array1::from_vec(high.clone()),
            Array1::from_vec(low.clone()),
            Array1::from_vec(close.clone()),
            Array1::from_vec(volume.clone()),
            None,
        );
        let reference = run_ast(&mut engine, source, &mut ctx_ref);
        assert_arrays_match(source, "eval", &fast, &reference);

        let finite = fast.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, expected_finite,
            "eval_zero_copy_inputs {source}: got {finite} finite values, \
             expected {expected_finite}"
        );
    }
}

/// Round 12: the same absorbing-`NaN` class as the rolling kernels, but living
/// in the **library layer** rather than the formula layer.
///
/// `AVGDEV`, `ZSCORE`, `TSF` and the whole `LINEARREG*` family keep an
/// incremental accumulator (`sum += newest - oldest`). Seeded from a window that
/// still contains an upstream indicator's leading `NaN` warm-up run, the
/// accumulator is poisoned for the entire series, so composing any of them onto
/// a rolling output produced all-`NaN`.
///
/// Two things make this class easy to miss, and both are guarded here:
///
/// * the differential gates cannot see it — before the fix the tree path and
///   the plan path were *identically* all-`NaN`, and `values_match` treats
///   `NaN`/`NaN` as agreement;
/// * the same logical indicator has more than one implementation
///   (`AVGDEV` delegates to the library, `AVEDEV` is hand-written; `fn_zscore`
///   uses `rolling_mean`, not `zscore`), so fixing one says nothing about the
///   other.
///
/// Every count below is an absolute property, not a comparison: a regression
/// that re-introduces the poison reports `0` and fails loudly.
#[test]
fn formula_differential_incremental_accumulators_survive_warmup() {
    const LEN: usize = 200;

    let mut engine = FormulaEngine::new();
    let mut seed_ctx = make_ctx(LEN);
    let warm = run_ast(&mut engine, "MA(CLOSE,5)", &mut seed_ctx);
    assert_eq!(warm.iter().filter(|value| value.is_finite()).count(), 196);

    // ---- tree path: the whole accumulator family ---------------------------
    //
    // `period = 9` over a 196-finite tail gives 196 - 9 + 1 = 188 values.
    for (source, expected_finite) in [
        ("AVGDEV(X,9)", 188),
        ("AVEDEV(X,9)", 188),
        ("ZSCORE(X,9)", 188),
        ("TSF(X,9)", 188),
        ("LINEARREG(X,9)", 188),
        ("LINEAR_REG(X,9)", 188),
        ("LINEARREG_SLOPE(X,9)", 188),
        ("LINEARREG_INTERCEPT(X,9)", 188),
        ("LINEARREG_ANGLE(X,9)", 188),
        ("SLOPE(X,9)", 188),
        ("FORCAST(X,9)", 188),
        // Wider windows already worked; kept as a control.
        ("SKEW(X,9)", 192),
        ("KURT(X,9)", 192),
    ] {
        let mut ctx = make_ctx(LEN);
        ctx.set_variable("X".to_string(), warm.clone());
        let values = run_ast(&mut engine, source, &mut ctx);
        let finite = values.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, expected_finite,
            "{source}: got {finite} finite values, expected {expected_finite} \
             (0 means the library-layer accumulator was seeded from the NaN prefix)"
        );
    }

    // ---- plan path ---------------------------------------------------------
    //
    // `ZSCORE` is the one member of the family with a plan kernel, and that
    // kernel is a *separate* implementation (`statistics::zscore_into`) rather
    // than the `rolling_mean` route the tree path takes. Its slide guard had to
    // be offset by the warm-up start as well: a bare `index >= period` evicted
    // `input[warm_start - period]` on the seed bar, which is still inside the
    // NaN prefix.
    {
        let mut ctx_plan = make_ctx(LEN);
        ctx_plan.set_variable("X".to_string(), warm.clone());
        let plan = run_plan("ZSCORE(X,9)", &ctx_plan);

        let mut ctx_tree = make_ctx(LEN);
        ctx_tree.set_variable("X".to_string(), warm.clone());
        let tree = run_ast(&mut engine, "ZSCORE(X,9)", &mut ctx_tree);
        assert_arrays_match("ZSCORE(X,9)", "plan", &tree, &plan);

        let finite = plan.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, 188,
            "ZSCORE(X,9) plan: got {finite} finite values, expected 188"
        );
    }

    // ---- duplicate implementations must agree ------------------------------
    //
    // These pairs are two independent implementations of the same statistic
    // reaching different code paths. Comparing them is an absolute property that
    // holds even when both paths are wrong in the same way, which is precisely
    // how the `AVGDEV`/`AVEDEV` split stayed hidden.
    for (left, right) in [
        ("AVGDEV(X,9)", "AVEDEV(X,9)"),
        ("SLOPE(X,9)", "LINEARREG_SLOPE(X,9)"),
        ("FORCAST(X,9)", "LINEARREG(X,9)"),
    ] {
        let mut ctx_left = make_ctx(LEN);
        ctx_left.set_variable("X".to_string(), warm.clone());
        let lhs = run_ast(&mut engine, left, &mut ctx_left);

        let mut ctx_right = make_ctx(LEN);
        ctx_right.set_variable("X".to_string(), warm.clone());
        let rhs = run_ast(&mut engine, right, &mut ctx_right);

        assert_arrays_match(left, right, &lhs, &rhs);
    }

    // ---- inert for NaN-free input ------------------------------------------
    //
    // The fix must not touch the ordinary path: with no leading non-finite run
    // the warm-up start is 0 and every index is byte-for-byte what it was. The
    // observable contract is the warm-up shape -- the first `period - 1` rows
    // stay `NaN` and the first finite value lands exactly on row `period - 1`.
    for source in ["AVGDEV(CLOSE,9)", "ZSCORE(CLOSE,9)", "LINEARREG(CLOSE,9)"] {
        let mut ctx = make_ctx(LEN);
        let values = run_ast(&mut engine, source, &mut ctx);
        for (index, value) in values.iter().enumerate().take(8) {
            assert!(
                value.is_nan(),
                "{source}: row {index} should be NaN warm-up on NaN-free input, got {value}"
            );
        }
        assert!(
            values[8].is_finite(),
            "{source}: row 8 should be the first finite value on NaN-free input, got {}",
            values[8]
        );
    }
}

/// The product documentation makes specific, checkable promises about warm-up
/// composition:
///
/// > A rolling indicator's warm-up `NaN` prefix must not poison composition:
/// > `MA(MA(CLOSE,5),9)` and `DEA:=EMA(DIF,9)` return valid values rather than
/// > all-`NaN`.
///
/// (README.md "Performance and correctness", README.zh-CN.md "正确性与性能原则",
/// docs/product-overview.md "Explicit semantics",
/// docs/formula-runtime-contract.md §3.1.)
///
/// Documentation that promises behavior nothing enforces is exactly how the
/// original defect survived: the docs described warm-up `NaN` correctly while
/// composition silently returned all-`NaN`. This gate keeps the two in sync, so
/// a regression fails here instead of quietly making the docs wrong.
#[test]
fn documented_warmup_composition_examples_hold() {
    const LEN: usize = 200;
    let mut engine = FormulaEngine::new();

    // Each entry is a formula quoted in the docs plus the number of finite
    // values it must produce. `MA(CLOSE,5)` over 200 bars leaves 196 finite.
    let cases: &[(&str, usize)] = &[
        // Quoted verbatim in the docs.
        ("MA(MA(CLOSE,5),9)", 188),
        // `EMA(CLOSE,12)`/`EMA(CLOSE,26)` leave 189/175 finite, so `DIF` has 175
        // and `EMA(DIF,9)` has 167.
        (
            "DIF:=EMA(CLOSE,12)-EMA(CLOSE,26); DEA:=EMA(DIF,9); DEA",
            167,
        ),
        // Same guarantee, applied to the statistics/regression family that the
        // round-12 fix unblocked.
        ("ZSCORE(MA(CLOSE,5),9)", 188),
        ("LINEARREG(MA(CLOSE,5),9)", 188),
        ("AVGDEV(MA(CLOSE,5),9)", 188),
        // Three levels deep, to show it is not a one-level special case.
        // Each rolling layer costs `period - 1` bars: 196 -> 188 -> 185.
        ("MA(MA(MA(CLOSE,5),9),4)", 185),
    ];

    for (source, expected_finite) in cases {
        let mut ctx = make_ctx(LEN);
        let values = run_ast(&mut engine, source, &mut ctx);
        let finite = values.iter().filter(|value| value.is_finite()).count();
        assert_eq!(
            finite, *expected_finite,
            "{source}: got {finite} finite values, expected {expected_finite}. \
             The docs promise warm-up composition yields valid values; a \
             0 here means the documentation is now wrong."
        );
    }
}

/// The warm-up composition contract, checked on **every** execution path.
///
/// The round-11 and round-12 gates compare tree against plan (and the entry
/// points against the reference). The bytecode VM, JIT, and SIMD paths had no
/// warm-up coverage at all — and a path that agrees with a broken reference is
/// exactly how three successive bugs stayed invisible. Each case here asserts
/// the absolute finite-value count on the reference path *and* requires every
/// other path to match it.
///
/// Counts come from `MA(CLOSE,5)` over 200 bars, i.e. 196 finite values fed in
/// as `X`; a rolling window of `p` costs `p - 1` bars.
#[test]
fn formula_differential_warmup_composition_all_paths() {
    const LEN: usize = 200;

    let mut engine = FormulaEngine::new();
    let mut seed_ctx = make_ctx(LEN);
    let warm = run_ast(&mut engine, "MA(CLOSE,5)", &mut seed_ctx);
    assert_eq!(warm.iter().filter(|value| value.is_finite()).count(), 196);

    for (source, expected_finite) in [
        // No function call at all: this isolates `load_variable`. Before the
        // bytecode/JIT fallback to `ctx.variables` these three failed outright
        // with "Unknown variable: X" while the tree path returned a series.
        ("X", 196),
        ("X + CLOSE", 196),
        ("X * 2 - CLOSE", 196),
        ("MA(X,9)", 188),
        ("EMA(X,9)", 188),
        ("WMA(X,9)", 188),
        ("RMA(X,9)", 188),
        ("TRIMA(X,9)", 188),
        ("SUM(X,9)", 188),
        ("STD(X,9)", 188),
        ("HHV(X,9)", 188),
        ("LLV(X,9)", 188),
        ("ROLLING_RANGE(X,9)", 188),
        ("ZSCORE(X,9)", 188),
        ("CORREL(X,CLOSE,9)", 188),
        ("RSI(X,9)", 191),
        ("MEDIAN(X,9)", 192),
        ("REF(X,3)", 193),
        ("TRIX(X,9)", 171),
        ("BOLL(X,20,2)", 177),
        ("CCI(X,14)", 183),
        ("HMA(X,9)", 186),
        ("REVERSE(X)", 196),
        ("CUMSUM(X)", 200),
    ] {
        check_all_paths_with_warm_variable(source, source, &warm, expected_finite);
    }
}
