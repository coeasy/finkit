//! External contract for the declarative factor graph.
//!
//! `core/src/factor_graph.rs` carries its own unit tests, but until now nothing
//! outside the module reached it — the module was declared in `lib.rs`, built a
//! compiled plan, and then had no public way to *run* that plan, so every
//! potential caller had to re-derive the input-binding loop from its tests.
//! That is the "built but not wired" shape this file exists to close.
//!
//! Two properties are held here from the outside:
//!
//! * **Graph ≡ formula string.** A graph and the equivalent formula source must
//!   produce the same numbers. This is the load-bearing claim: if it ever
//!   breaks, the graph has quietly become a second execution engine.
//! * **Binding is explicit and total.** A declared input the context cannot
//!   supply must fail loudly, never resolve to a silently substituted series.

use finkit::factor_graph::{FactorGraph, FactorGraphError, FactorNode};
use finkit::formula::{BinaryOperator, FormulaContext, FormulaEngine};
use ndarray::Array1;

/// Deterministic synthetic OHLCV: a rising, oscillating close so that smoothing
/// kernels produce distinguishable values instead of a constant.
#[allow(clippy::cast_precision_loss)] // bar counts here are far below 2^53
fn context(len: usize) -> FormulaContext {
    let series = |offset: f64| {
        Array1::from_iter((0..len).map(|index| {
            let step = index as f64;
            offset + step * 0.5 + (step * 0.37).sin() * 3.0
        }))
    };
    let close = series(100.0);
    FormulaContext::new(
        series(99.0),
        close.clone() + 2.0,
        series(98.0),
        close,
        Array1::from_iter((0..len).map(|index| 1000.0 + index as f64)),
        None,
    )
}

/// Compare two series element-wise, treating two NaNs as equal.
///
/// Warm-up bars are legitimately NaN — `SMA(CLOSE, 20)` has none defined for its
/// first 19 samples — so a plain `assert_eq!` on the vectors would fail on a
/// correct result.
fn assert_same_series(left: &[f64], right: &[f64], what: &str) {
    assert_eq!(
        left.len(),
        right.len(),
        "{what}: length mismatch ({} vs {})",
        left.len(),
        right.len()
    );
    for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
        let same = if l.is_nan() && r.is_nan() {
            true
        } else {
            (l - r).abs() <= 1e-9
        };
        assert!(same, "{what}: differs at {index}: {l} vs {r}");
    }
}

/// The graph used by every case below: a fast/slow SMA ratio.
///
/// `RATIO = SMA(CLOSE, 5) / SMA(CLOSE, 20)` — three nodes, one binary operator,
/// one declared external input.
fn ratio_graph() -> FactorGraph {
    let mut graph = FactorGraph::new();
    graph.declare_input("CLOSE");
    graph
        .add_node(FactorNode::new("SLOW", "SMA").input("CLOSE").param(20.0))
        .expect("SLOW node is valid");
    graph
        .add_node(FactorNode::new("FAST", "SMA").input("CLOSE").param(5.0))
        .expect("FAST node is valid");
    graph
        .add_node(
            FactorNode::binary("RATIO", BinaryOperator::Div)
                .input("FAST")
                .input("SLOW"),
        )
        .expect("RATIO node is valid");
    graph
}

/// A graph's primary result must match the equivalent formula source.
#[test]
fn factor_graph_matches_the_equivalent_formula_source() {
    let ctx = context(120);
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");

    let from_graph = plan
        .execute_node(&ctx, "RATIO")
        .expect("the graph executes against the context");

    // The same computation, expressed as a formula string and run through the
    // ordinary tree-walking path.
    let mut engine = FormulaEngine::new();
    let mut formula_ctx = context(120);
    let from_formula = engine
        .eval(
            "SLOW:=SMA(CLOSE,20); FAST:=SMA(CLOSE,5); RATIO:FAST/SLOW;",
            &mut formula_ctx,
        )
        .expect("the equivalent formula evaluates");

    assert_same_series(
        &from_graph,
        from_formula.as_slice().expect("contiguous result"),
        "graph vs formula string",
    );
}

/// An intermediate node must be readable by id — that is the reason to use a
/// graph instead of a formula string.
#[test]
fn intermediate_nodes_are_readable_by_id() {
    let ctx = context(120);
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");

    let slow = plan
        .execute_node(&ctx, "SLOW")
        .expect("SLOW is an output node");

    let mut engine = FormulaEngine::new();
    let mut formula_ctx = context(120);
    let expected = engine
        .eval("SMA(CLOSE,20)", &mut formula_ctx)
        .expect("SMA evaluates");

    assert_same_series(
        &slow,
        expected.as_slice().expect("contiguous result"),
        "intermediate node SLOW",
    );
}

/// Binding is total: a declared input the context cannot supply must fail.
///
/// The failure mode this guards against is severe and silent — an unresolved
/// variable in a compiled plan is classified as an *external input*, so a
/// missing series would otherwise be demanded from the caller as data rather
/// than reported as a mistake.
#[test]
fn a_declared_input_the_context_cannot_supply_fails() {
    let mut graph = FactorGraph::new();
    graph.declare_input("ADJ_CLOSE");
    graph
        .add_node(FactorNode::new("SLOW", "SMA").input("ADJ_CLOSE").param(20.0))
        .expect("SLOW node is valid");
    let plan = graph.build("SLOW").expect("the graph compiles");

    let ctx = context(120); // has OPEN/HIGH/LOW/CLOSE/VOLUME, no ADJ_CLOSE.
    match plan.execute(&ctx) {
        Ok(_) => panic!("executing without ADJ_CLOSE must fail"),
        Err(FactorGraphError::MissingInput { name }) => {
            assert_eq!(name, "ADJ_CLOSE");
        }
        Err(other) => panic!("expected MissingInput, got {other:?}"),
    }
}

/// Reading a series that is not a node of the graph is an error, not a panic.
#[test]
fn reading_an_unknown_node_fails_loudly() {
    let ctx = context(120);
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");

    match plan.execute_node(&ctx, "NOT_A_NODE") {
        Ok(_) => panic!("reading an unknown node must fail"),
        Err(FactorGraphError::UnknownNode { id }) => assert_eq!(id, "NOT_A_NODE"),
        Err(other) => panic!("expected UnknownNode, got {other:?}"),
    }
}

/// Node ids are canonicalized, so the spelling used at declaration works at read
/// time regardless of case or surrounding whitespace.
#[test]
fn node_ids_are_case_and_whitespace_insensitive() {
    let ctx = context(120);
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");

    let canonical = plan
        .execute_node(&ctx, "RATIO")
        .expect("RATIO is a node");
    let lower = plan.execute_node(&ctx, "ratio").expect("case-insensitive");
    let padded = plan
        .execute_node(&ctx, " ratio ")
        .expect("whitespace-insensitive");

    assert_same_series(&canonical, &lower, "RATIO vs ratio");
    assert_same_series(&canonical, &padded, "RATIO vs ' ratio '");
}
