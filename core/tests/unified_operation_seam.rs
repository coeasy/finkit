//! Contract for the two declarative seams wired into the unified operation façade.
//!
//! `factor_graph` and `factor_provider` were built and tested but had no caller
//! outside their own modules — the "built but not wired" shape. They now reach
//! production through `operation::UnifiedOperationEngine`, and this file holds
//! that from the outside:
//!
//! * a [`FactorGraph`] runs through [`OperationRequest::FactorGraph`] and returns
//!   every node series by id, not just the primary;
//! * a declarative provider request becomes a registered factor via
//!   `define_factor` and is then executable through the ordinary
//!   [`OperationRequest::Factor`] path.
//!
//! Together they make the claim concrete: the declarative frontends are not a
//! second execution engine, they are alternative ways to reach the same one.

use finkit::factor_graph::{FactorGraph, FactorNode};
use finkit::factor_provider::{FactorFactoryRequest, FactorProvider, FactorProviderRegistry};
use finkit::factors::{
    BorrowedFactorContext, FactorDefinition, FactorDirection, FactorInputs, FactorKind,
    FactorRegistry, FactorResult,
};
use finkit::formula::{BinaryOperator, FormulaContext};
use finkit::operation::{OperationRequest, UnifiedOperationEngine};
use ndarray::Array1;
use std::collections::HashMap;
use std::sync::Arc;

/// Rising, oscillating series so smoothing kernels produce distinct values.
#[allow(clippy::cast_precision_loss)] // bar counts here are far below 2^53
fn formula_context(len: usize) -> FormulaContext {
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

/// `RATIO = SMA(CLOSE, 5) / SMA(CLOSE, 20)`.
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

/// A graph executed through the unified façade returns every node by id.
#[test]
fn a_factor_graph_runs_through_the_unified_facade() {
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");
    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
    let mut ctx = formula_context(120);

    // Empty `outputs` means "every node", which is the point of a graph.
    let result = engine
        .execute(OperationRequest::FactorGraph {
            plan: &plan,
            outputs: &[],
            context: &mut ctx,
        })
        .expect("the graph executes through the unified façade");

    for id in ["SLOW", "FAST", "RATIO"] {
        assert!(result.values.contains_key(id), "missing node `{id}`");
        assert_eq!(result.values[id].len(), 120, "node `{id}` length");
    }
    // Three nodes were requested implicitly, so this is a multi-series result.
    assert_eq!(result.values.len(), 3);
}

/// Requesting a subset yields exactly that subset, and the primary follows it.
#[test]
fn a_subset_of_nodes_can_be_selected() {
    let plan = ratio_graph()
        .build("RATIO")
        .expect("the ratio graph compiles");
    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
    let mut ctx = formula_context(120);

    let result = engine
        .execute(OperationRequest::FactorGraph {
            plan: &plan,
            outputs: &["RATIO"],
            context: &mut ctx,
        })
        .expect("the graph executes");

    assert_eq!(result.values.len(), 1);
    assert!(result.values.contains_key("RATIO"));
    assert_eq!(result.primary.as_deref(), Some("RATIO"));
}

/// A declarative provider request registers a factor the façade can then run.
///
/// This is the whole point of the provider boundary: `("SMA", period=5)` is a
/// request a program can build, and it becomes something executable without the
/// caller knowing how the factor is implemented.
#[test]
fn a_declarative_request_registers_an_executable_factor() {
    let mut providers = FactorProviderRegistry::new();
    providers.register(SmaProvider);

    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
    let name = engine
        .define_factor(&providers, &FactorFactoryRequest::new("SMA").with_param("period", "5"))
        .expect("the provider accepts the request");
    assert_eq!(name, "SMA_5");

    let close: Vec<f64> = (1..=10).map(f64::from).collect();
    let context = BorrowedFactorContext::new()
        .with_series("CLOSE", &close)
        .expect("context accepts CLOSE");

    let result = engine
        .execute(OperationRequest::Factor {
            name: &name,
            context: &context,
            data_revision: None,
            cache_scope: None,
        })
        .expect("the declarative factor executes");

    let values = &result.values[&name];
    assert!(values[..4].iter().all(|value| value.is_nan()), "warm-up NaN");
    for (index, expected) in [3.0, 4.0, 5.0, 6.0, 7.0, 8.0].iter().enumerate() {
        assert!(
            (values[index + 4] - expected).abs() < 1e-9,
            "at {}: {} vs {}",
            index + 4,
            values[index + 4],
            expected
        );
    }
}

/// A provider that rejects a request must surface that through the façade
/// instead of the engine silently registering a half-built factor.
#[test]
fn a_rejected_request_never_reaches_the_registry() {
    let mut providers = FactorProviderRegistry::new();
    providers.register(SmaProvider);

    let mut engine = UnifiedOperationEngine::new(FactorRegistry::new());
    let error = engine
        .define_factor(
            &providers,
            &FactorFactoryRequest::new("SMA").with_param("period", "0"),
        )
        .expect_err("period=0 violates the contract");

    assert!(
        matches!(
            error,
            finkit::operation::OperationExecutionError::FactorProvider(_)
        ),
        "expected a provider error, got {error:?}"
    );
}

/// A deliberately simple provider: a plain moving average over `CLOSE`.
struct SmaProvider;

impl FactorProvider for SmaProvider {
    fn name(&self) -> &str {
        "SMA"
    }

    fn create(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<FactorDefinition, finkit::factor_provider::FactorFactoryError> {
        finkit::factor_provider::reject_unknown("SMA", params, &["period"])?;
        let period = finkit::factor_provider::positive_usize("SMA", params, "period", 20)?;
        Ok(FactorDefinition::new(
            format!("SMA_{period}"),
            ["CLOSE"],
            FactorKind::TimeSeries,
            FactorDirection::Neutral,
            Arc::new(move |inputs: &FactorInputs<'_>| -> FactorResult<Vec<f64>> {
                let close = inputs.get("CLOSE")?;
                let mut out = vec![f64::NAN; close.len()];
                if period <= close.len() {
                    let mut sum: f64 = close[..period].iter().sum();
                    out[period - 1] = sum / period as f64;
                    for index in period..close.len() {
                        sum += close[index] - close[index - period];
                        out[index] = sum / period as f64;
                    }
                }
                Ok(out)
            }),
        ))
    }
}
