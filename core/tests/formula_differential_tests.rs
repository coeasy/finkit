//! Differential consistency tests across formula execution paths.
//!
//! For each formula and input, compares AST interpretation, bytecode VM,
//! JIT (when enabled), and SIMD (when enabled). Any divergence beyond
//! tolerance `1e-10` fails the test with index and values printed.

use finkit::formula::{FormulaContext, FormulaEngine};
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
                formula_name, i, path_name,
                reference[i], path_name, candidate[i], TOLERANCE
            );
        }
    }
}

fn run_ast(engine: &mut FormulaEngine, source: &str, ctx: &mut FormulaContext) -> Array1<f64> {
    engine.eval(source, ctx).expect("AST eval failed")
}

fn run_bytecode(
    engine: &mut FormulaEngine,
    source: &str,
    ctx: &FormulaContext,
) -> Array1<f64> {
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

fn check_all_paths(formula_name: &str, source: &str, data_len: usize) {
    let mut engine = FormulaEngine::new();

    let mut ctx_ast = make_ctx(data_len);
    let reference = run_ast(&mut engine, source, &mut ctx_ast);

    let ctx_bc = make_ctx(data_len);
    let bytecode_result = run_bytecode(&mut engine, source, &ctx_bc);
    assert_arrays_match(formula_name, "bytecode", &reference, &bytecode_result);

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
