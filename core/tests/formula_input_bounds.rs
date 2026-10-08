//! Crash-surface gate: hostile input must come back as typed errors, not aborts.
//!
//! Batch 6 of the V5 refactor plan locks the following contract:
//!
//! 1. A source larger than the parser budget (1 MiB, matching the Pine
//!    dialect) is rejected at parse time.
//! 2. A flat chain (`1+1+1+…`) with no parenthesis nesting — the input that
//!    slips past the bracket-depth check — is rejected once its AST depth
//!    exceeds the structural cap, on the tree path and the plan path alike.
//! 3. A programmatically built AST deeper than the lowerer's budget reaches
//!    `FormulaComputePlan::compile` and comes back as a *typed*
//!    `ComputePlanError::LoweringDepthExceeded`, checked while lowering runs
//!    (the old code only measured depth and checked it after the stack had
//!    already been spent).
//! 4. Legal formulas of realistic depth still parse and evaluate on both
//!    paths — the limits reject only input that could never have run.
//!
//! Every assertion below is phrased as "returns Err with message X"; a stack
//! overflow would abort the process and fail the harness instead.

use finkit::compute::ComputePlanError;
use finkit::formula::ast::{AstNode, BinaryOperator};
use finkit::formula::compute_ir::FormulaComputePlan;
use finkit::formula::engine::{FormulaEngine, FormulaExecutionMode};
use finkit::formula::parser::parse_formula;
use finkit::formula::types::{FormulaContext, FormulaError};
use ndarray::Array1;

/// Build `1+1+…+1` with `terms` literals (a flat, parenthesis-free source).
fn flat_chain(terms: usize) -> String {
    let mut s = String::with_capacity(terms * 2);
    for i in 0..terms {
        if i > 0 {
            s.push('+');
        }
        s.push('1');
    }
    s
}

/// Build a left-nested `BinaryOp` chain of the given AST depth.
fn deep_ast(depth: usize) -> AstNode {
    let mut node = AstNode::Number(1.0);
    for _ in 1..depth {
        node = AstNode::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(node),
            right: Box::new(AstNode::Number(1.0)),
        };
    }
    node
}

fn make_ctx(len: usize) -> FormulaContext {
    let open = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.1).collect());
    let high = Array1::from_vec((0..len).map(|i| 11.0 + i as f64 * 0.2).collect());
    let low = Array1::from_vec((0..len).map(|i| 9.0 + i as f64 * 0.1).collect());
    let close = Array1::from_vec((0..len).map(|i| 10.0 + i as f64 * 0.15).collect());
    let volume = Array1::from_vec((0..len).map(|i| 1000.0 + i as f64 * 10.0).collect());
    FormulaContext::new(open, high, low, close, volume, None)
}

#[test]
fn flat_chain_beyond_ast_depth_is_rejected_at_parse() {
    // 50_000 terms ≈ 100 KB of source — far under the 1 MiB size cap, so this
    // exercises exactly the depth gate and nothing else. 50_000-deep ASTs used
    // to reach the recursive consumers; the iterative check must stop them
    // with a diagnostic instead of an abort.
    let source = flat_chain(50_000);
    let err = parse_formula(&source).unwrap_err();
    assert!(
        err.contains("expression depth exceeds"),
        "expected depth-limit error, got: {err}"
    );
}

#[test]
fn source_beyond_size_cap_is_rejected_at_parse() {
    // Just over 1 MiB: the size gate must fire before anything else walks it.
    let source = flat_chain(600_000);
    assert!(source.len() > 1 << 20);
    let err = parse_formula(&source).unwrap_err();
    assert!(
        err.contains("size limit"),
        "expected size-limit error, got: {err}"
    );
}

#[test]
fn flat_chain_beyond_depth_errors_on_both_engine_paths() {
    let source = flat_chain(50_000);
    for mode in [FormulaExecutionMode::Tree, FormulaExecutionMode::Plan] {
        let mut ctx = make_ctx(10);
        let mut engine = FormulaEngine::new();
        engine.set_execution_mode(mode);
        let err = match engine.eval(&source, &mut ctx) {
            Err(e) => e,
            Ok(_) => panic!("flat chain beyond depth evaluated on the {mode:?} path"),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("expression depth exceeds"),
            "expected typed depth error on the {mode:?} path, got: {msg}"
        );
    }
}

#[test]
fn programmatic_ast_beyond_lower_budget_fails_with_typed_error() {
    // Built directly as an AST — bypasses the parser, so only the lowerer's
    // own in-walk check can catch it. The error must be the typed variant,
    // surfaced through `fail`, not a process abort.
    //
    // Run on a fat stack so the *test harness* thread (which libtest gives a
    // small default) cannot be the thing that overflows: the assertion is that
    // the check fires at 2048 frames, well before any realistic stack is
    // spent. On a default-size thread the same input previously overflowed
    // while lowering — that is the regression this gate guards.
    let handle = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let ast = deep_ast(10_000);
            match FormulaComputePlan::compile(&ast) {
                Err(ComputePlanError::LoweringDepthExceeded { depth, max }) => {
                    assert_eq!(max, 2048, "the lowerer's documented budget");
                    assert!(depth > max, "reported depth {depth} must exceed {max}");
                }
                Err(other) => panic!("expected LoweringDepthExceeded, got: {other}"),
                Ok(_) => panic!("a 10_000-deep AST must not lower"),
            }
        })
        .expect("spawn deep-stack test thread");
    handle
        .join()
        .expect("deep-stack test thread must not abort");
}

#[test]
fn deep_but_legal_formulas_still_evaluate_on_both_paths() {
    // 200 nested MA calls: AST depth ~200 — comfortably inside every budget,
    // deep enough to prove the limits did not clamp legitimate input.
    let mut source = String::from("CLOSE");
    for _ in 0..200 {
        source = format!("MA({source},2)");
    }
    for mode in [FormulaExecutionMode::Tree, FormulaExecutionMode::Plan] {
        let mut ctx = make_ctx(64);
        let mut engine = FormulaEngine::new();
        engine.set_execution_mode(mode);
        let result = engine.eval(&source, &mut ctx);
        assert!(
            result.is_ok(),
            "a 200-deep legal formula failed on the {mode:?} path: {:?}",
            result.err().map(|e: FormulaError| e.to_string())
        );
    }
}

#[test]
fn flat_chain_within_depth_still_parses_and_compiles() {
    // 1_000 terms: parses (depth ≤ 1024, exactly at the boundary minus one)
    // and lowers (1_000 frames ≤ 2048). Compilation runs on a fat thread so
    // the test's own stack budget cannot interfere with the assertion; the
    // engine paths are covered by `deep_but_legal_formulas_still_evaluate…`
    // at a depth that fits ordinary threads.
    let source = flat_chain(1_000);
    let ast = parse_formula(&source).expect("a 1_000-term chain must parse");
    let handle = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            FormulaComputePlan::compile(&ast).expect("a 1_000-term chain must lower");
        })
        .expect("spawn compile thread");
    handle.join().expect("compile thread must not abort");
}

#[test]
fn engine_default_sandbox_is_unlimited_by_documented_contract() {
    // The sandbox default stays fully unlimited on purpose: the crash surface
    // is closed structurally at parse/lowering (see the other tests here), and
    // the all-`None` state doubles as the engine fast-path eligibility flag —
    // a bounded default would silently route everyone onto the general
    // executor. Depth *budgets* remain available for callers that want them.
    let config = <finkit::formula::sandbox::ExecSandboxConfig as Default>::default();
    assert!(config.max_recursion_depth.is_none());
    assert!(config.timeout_ms.is_none());
    assert!(config.max_memory_bytes.is_none());
}
