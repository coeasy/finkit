//! Shared discovery contract for the built-in Factor catalog.

use finkit::factor_system::FactorCatalog;
use finkit::factors::{builtin_factor_registry, FactorDirection, FactorKind};
use serde::Serialize;

/// Version of the cross-language Factor catalog envelope.
pub const FACTOR_CATALOG_SCHEMA_VERSION: u16 = 1;

/// Versioned Factor catalog shared by all official bindings.
#[derive(Debug, Clone, Serialize)]
pub struct FactorCatalogEnvelope {
    pub schema_version: u16,
    pub engine_version: &'static str,
    pub factors: Vec<FactorCatalogEntry>,
}

/// Serializable metadata for one built-in Factor.
#[derive(Debug, Clone, Serialize)]
pub struct FactorCatalogEntry {
    pub name: String,
    pub kind: &'static str,
    pub direction: &'static str,
    pub dependencies: Vec<String>,
    pub version: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub deterministic: bool,
    /// Whether the factor can use the finite-lookback streaming contract.
    pub bounded_streaming: bool,
    /// Whether the factor has an explicit serializable O(1) state kernel.
    pub stateful_streaming: bool,
    /// Aggregate streaming capability; callers should inspect the specific
    /// bounded/stateful fields when selecting an execution mode.
    pub streaming: bool,
    pub incremental: bool,
    pub fixed_lookback: Option<usize>,
}

/// Build the stable catalog of portable built-in Factors.
pub fn factor_catalog() -> FactorCatalogEnvelope {
    let registry = builtin_factor_registry();
    let catalog = FactorCatalog::from_registry(registry.clone());
    let factors = registry
        .iter()
        .filter_map(|definition| {
            let descriptor = catalog.descriptor(&definition.name)?;
            Some(FactorCatalogEntry {
                name: descriptor.name,
                kind: factor_kind_name(definition.kind),
                direction: factor_direction_name(definition.direction),
                dependencies: descriptor.dependencies,
                version: descriptor.metadata.version,
                description: descriptor.metadata.description,
                aliases: descriptor.metadata.aliases,
                deterministic: descriptor.metadata.deterministic,
                bounded_streaming: descriptor.metadata.incremental
                    && descriptor.metadata.fixed_lookback.is_some()
                    && definition.kind == FactorKind::TimeSeries,
                stateful_streaming: catalog.stateful_spec(&definition.name).is_some(),
                streaming: descriptor.metadata.streaming,
                incremental: descriptor.metadata.incremental,
                fixed_lookback: descriptor.metadata.fixed_lookback,
            })
        })
        .collect();
    FactorCatalogEnvelope {
        schema_version: FACTOR_CATALOG_SCHEMA_VERSION,
        engine_version: env!("CARGO_PKG_VERSION"),
        factors,
    }
}

/// Serialize the built-in Factor catalog.
pub fn factor_catalog_json() -> Result<String, serde_json::Error> {
    serde_json::to_string(&factor_catalog())
}

fn factor_kind_name(kind: FactorKind) -> &'static str {
    match kind {
        FactorKind::TimeSeries => "time_series",
        FactorKind::CrossSectional => "cross_sectional",
    }
}

fn factor_direction_name(direction: FactorDirection) -> &'static str {
    match direction {
        FactorDirection::HigherBetter => "higher_better",
        FactorDirection::LowerBetter => "lower_better",
        FactorDirection::Neutral => "neutral",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn catalog_is_stable_and_describes_dependencies() {
        let payload: Value = serde_json::from_str(&factor_catalog_json().unwrap()).unwrap();
        assert_eq!(payload["schema_version"], FACTOR_CATALOG_SCHEMA_VERSION);
        let factors = payload["factors"].as_array().unwrap();
        assert_eq!(factors.len(), 9);
        let momentum = factors
            .iter()
            .find(|factor| factor["name"] == "momentum_5")
            .unwrap();
        assert_eq!(momentum["kind"], "time_series");
        assert_eq!(momentum["direction"], "higher_better");
        assert_eq!(momentum["bounded_streaming"], true);
        assert_eq!(momentum["stateful_streaming"], true);
        assert_eq!(momentum["streaming"], true);
        let reversal = factors
            .iter()
            .find(|factor| factor["name"] == "reversal_5")
            .unwrap();
        assert_eq!(reversal["dependencies"][0], "momentum_5");
    }
}
