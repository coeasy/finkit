//! Typed view of the indicator registry — the single source of truth.
//!
//! The canonical data lives in `docs/indicator_registry.json` and also drives
//! `scripts/gen_ssot_docs.py`. We embed it at compile time so bindings and the
//! (future) code generator read the exact same list the docs are generated
//! from, without a runtime file dependency.

use serde::Deserialize;

/// A single indicator parameter as declared in the registry.
#[derive(Debug, Clone, Deserialize)]
pub struct Param {
    pub name: String,
    pub param_type: String,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub description: String,
}

/// A single registered indicator.
#[derive(Debug, Clone, Deserialize)]
pub struct Indicator {
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub params: Vec<Param>,
    #[serde(default)]
    pub convergence: Option<usize>,
    #[serde(default)]
    pub streaming: bool,
}

/// The full registry document.
#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub version: String,
    pub indicators: Vec<Indicator>,
}

impl Registry {
    /// The registry embedded at compile time from
    /// `docs/indicator_registry.json`.
    pub fn embedded() -> &'static Registry {
        static REGISTRY: std::sync::OnceLock<Registry> = std::sync::OnceLock::new();
        REGISTRY.get_or_init(|| {
            let json = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/indicator_registry.json"
            ));
            serde_json::from_str(json).expect("indicator_registry.json must be valid JSON")
        })
    }

    /// Indicators belonging to a given category.
    pub fn by_category<'a>(&'a self, category: &'a str) -> impl Iterator<Item = &'a Indicator> + 'a {
        self.indicators
            .iter()
            .filter(move |i| i.category == category)
    }

    /// Look up an indicator by (case-sensitive) name.
    pub fn find(&self, name: &str) -> Option<&Indicator> {
        self.indicators.iter().find(|i| i.name == name)
    }

    /// Sorted, de-duplicated list of categories present in the registry.
    pub fn categories(&self) -> Vec<&str> {
        let mut cats: Vec<&str> = self.indicators.iter().map(|i| i.category.as_str()).collect();
        cats.sort_unstable();
        cats.dedup();
        cats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_loads_and_parses() {
        let reg = Registry::embedded();
        assert!(!reg.indicators.is_empty(), "registry must contain indicators");
        // SMA is the canonical first entry in docs/indicator_registry.json.
        let sma = reg.find("SMA").expect("SMA should be registered");
        assert_eq!(sma.category, "overlap");
        assert!(sma.streaming);
        // Categories must be non-empty and de-duplicated.
        let cats = reg.categories();
        assert!(cats.contains(&"overlap"));
        assert_eq!(cats.len(), cats.iter().collect::<std::collections::HashSet<_>>().len());
    }
}
