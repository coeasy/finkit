//! External contract for the declarative factor provider boundary.
//!
//! `core/src/factor_provider.rs` carries its own unit tests but, like
//! `factor_graph`, had no caller outside the module. This file closes that from
//! the outside and holds the properties a caller actually depends on:
//!
//! * **A request becomes a runnable factor.** The whole point of the boundary is
//!   that a declarative request (`"SMA"`, `period=20`) yields a
//!   `FactorDefinition` the existing `FactorEngine` can evaluate. Building the
//!   definition but never running it would prove nothing.
//! * **The three failure modes stay distinct.** Invalid / unknown / duplicate
//!   parameters must remain separately actionable instead of collapsing into one
//!   opaque error.
//! * **`canonical_params` is order-independent**, because it is the shape a cache
//!   key needs: `{period=20, shift=1}` and `{shift=1, period=20}` must hash the
//!   same, or the cache silently misses.

use finkit::factor_provider::{
    positive_usize, reject_unknown, FactorFactoryError, FactorFactoryRequest, FactorProvider,
    FactorProviderError, FactorProviderRegistry,
};
use finkit::factors::{
    FactorContext, FactorDefinition, FactorDirection, FactorEngine, FactorInputs, FactorKind,
    FactorRegistry, FactorResult,
};
use std::collections::HashMap;
use std::sync::Arc;

/// A deliberately simple provider: a plain moving average over `CLOSE`.
struct SmaProvider;

impl FactorProvider for SmaProvider {
    fn name(&self) -> &str {
        "SMA"
    }

    fn create(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<FactorDefinition, FactorFactoryError> {
        reject_unknown("SMA", params, &["period"])?;
        let period = positive_usize("SMA", params, "period", 20)?;
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

fn registry() -> FactorProviderRegistry {
    let mut registry = FactorProviderRegistry::new();
    registry.register(SmaProvider);
    registry
}

/// A declarative request must produce a factor the real engine can evaluate.
#[test]
fn a_declarative_request_produces_a_runnable_factor() {
    let definition = registry()
        .create(&FactorFactoryRequest::new("SMA").with_param("period", "5"))
        .expect("the provider accepts period=5");

    assert_eq!(definition.name, "SMA_5");

    let close: Vec<f64> = (1..=10).map(f64::from).collect();
    let context = FactorContext::new()
        .with_series("CLOSE", close)
        .expect("context accepts CLOSE");

    let mut factors = FactorRegistry::new();
    factors
        .register(definition)
        .expect("the produced definition registers");
    let engine = FactorEngine::new(factors);

    let values = engine
        .evaluate("SMA_5", &context)
        .expect("the produced factor evaluates");

    assert_eq!(values.len(), 10);
    // Warm-up is preserved as NaN rather than being silently filled.
    assert!(values[..4].iter().all(|value| value.is_nan()));
    // mean(1..=5) = 3, mean(2..=6) = 4, ... mean(6..=10) = 8.
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

/// An unparseable or out-of-contract parameter must be reported as such.
#[test]
fn an_invalid_parameter_is_reported_distinctly() {
    let error = registry()
        .create(&FactorFactoryRequest::new("SMA").with_param("period", "0"))
        .err()
        .expect("period=0 violates the contract");

    match error {
        FactorProviderError::Factory(FactorFactoryError::InvalidParameter {
            factor,
            name,
            value,
            ..
        }) => {
            assert_eq!(factor, "SMA");
            assert_eq!(name, "period");
            assert_eq!(value, "0");
        }
        other => panic!("expected InvalidParameter, got {other:?}"),
    }
}

/// A parameter the provider does not accept must not be silently ignored.
#[test]
fn an_unknown_parameter_is_rejected() {
    let error = registry()
        .create(&FactorFactoryRequest::new("SMA").with_param("smoothing", "3"))
        .err()
        .expect("smoothing is not accepted");

    match error {
        FactorProviderError::Factory(FactorFactoryError::UnknownParameter { factor, name }) => {
            assert_eq!(factor, "SMA");
            assert_eq!(name, "smoothing");
        }
        other => panic!("expected UnknownParameter, got {other:?}"),
    }
}

/// Supplying the same parameter twice is a mistake, not a last-write-wins.
#[test]
fn a_duplicate_parameter_is_rejected() {
    let error = FactorFactoryRequest::new("SMA")
        .with_param("period", "5")
        .with_param("period", "10")
        .try_params_map()
        .expect_err("period supplied twice");

    match error {
        FactorFactoryError::DuplicateParameter { factor, name } => {
            assert_eq!(factor, "SMA");
            assert_eq!(name, "period");
        }
        other => panic!("expected DuplicateParameter, got {other:?}"),
    }
}

/// Parameter identity must not depend on the order the parameters were supplied.
///
/// This is what makes `canonical_params` usable as a cache key.
#[test]
fn canonical_params_is_order_independent() {
    let forward = FactorFactoryRequest::new("SMA")
        .with_param("period", "20")
        .with_param("shift", "1");
    let reversed = FactorFactoryRequest::new("SMA")
        .with_param("shift", "1")
        .with_param("period", "20");

    assert_eq!(forward.canonical_params(), reversed.canonical_params());
}

/// An unregistered name must be reported before parameter validation, so the
/// caller learns the provider is missing rather than that its params are odd.
#[test]
fn an_unknown_provider_is_reported_before_validation() {
    let error = registry()
        .create(&FactorFactoryRequest::new("NOT_REGISTERED"))
        .err()
        .expect("no such provider");

    match error {
        FactorProviderError::UnknownProvider(name) => assert_eq!(name, "NOT_REGISTERED"),
        other => panic!("expected UnknownProvider, got {other:?}"),
    }
}
