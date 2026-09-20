//! Declarative factor construction: a provider boundary with typed parameter
//! validation.
//!
//! [`crate::factors::FactorDefinition`] describes a factor that is *already*
//! registered. This module covers the step before that: turning a declarative
//! request (`"SMA"`, `period=20`) into a `FactorDefinition`, reporting the three
//! failure modes that actually matter as distinct, actionable errors instead of
//! one opaque failure:
//!
//! * [`FactorFactoryError::InvalidParameter`] — present, but unparseable or
//!   outside the factor's contract;
//! * [`FactorFactoryError::UnknownParameter`] — not accepted by the provider;
//! * [`FactorFactoryError::DuplicateParameter`] — supplied more than once.
//!
//! [`FactorFactoryRequest::canonical_params`] produces a stable,
//! order-independent parameter identity. That is exactly what a result-cache key
//! needs, so it is deliberately a pure function of the parameter *set*:
//! `a=1,b=2` and `b=2,a=1` canonicalise identically.
//!
//! # Provenance
//!
//! Ported from the `crates/finkit-runtime` migration track, whose `Executor`,
//! `Scheduler`, `FactorRegistry` and `FactorCache` are superseded by this crate's
//! `unified_executor`, `compute`, `factors` and `operation` modules — see
//! `docs/runtime-carrier-adoption-plan-2026-09-20.md`.
//!
//! Two deliberate omissions from that track:
//!
//! * Its `FactorResult` multi-output struct is **not** ported. Multi-output is
//!   already expressed by [`crate::execution_plan::OutputLayout`] plus
//!   [`crate::formula::FormulaHotPlan::outputs`]; porting the struct would also
//!   have collided with [`crate::factors::FactorResult`], which is a `Result`
//!   alias rather than a result model.
//! * Its warm-up *trimming* output semantics are **not** ported. Warm-up is owned
//!   by [`crate::runtime::WarmupPolicy`], which preserves length; a second,
//!   incompatible convention would make the same formula produce different
//!   lengths on different paths.

use std::collections::HashMap;
use std::fmt;
use std::hash::BuildHasher;
use std::sync::Arc;

use crate::factors::FactorDefinition;

/// Errors raised while turning a declarative factor request into a definition.
///
/// Every variant names the offending factor and parameter, so a binding can
/// surface a structured, actionable response instead of a generic failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorFactoryError {
    /// A parameter could not be parsed or violates the factor contract.
    InvalidParameter {
        /// Factor being constructed.
        factor: String,
        /// Parameter name.
        name: String,
        /// Offending value, as supplied.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
    /// A request contains a parameter not accepted by the provider.
    UnknownParameter {
        /// Factor being constructed.
        factor: String,
        /// Parameter name.
        name: String,
    },
    /// A request contains the same parameter more than once.
    DuplicateParameter {
        /// Factor being constructed.
        factor: String,
        /// Parameter name.
        name: String,
    },
}

impl fmt::Display for FactorFactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParameter {
                factor,
                name,
                value,
                reason,
            } => write!(f, "invalid parameter {factor}.{name}={value:?}: {reason}"),
            Self::UnknownParameter { factor, name } => {
                write!(f, "unknown parameter {factor}.{name}")
            }
            Self::DuplicateParameter { factor, name } => {
                write!(f, "duplicate parameter {factor}.{name}")
            }
        }
    }
}

impl std::error::Error for FactorFactoryError {}

/// A declarative request for one factor instance.
///
/// Parameters are kept as an ordered list rather than a map so that a duplicate
/// can be detected instead of silently collapsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FactorFactoryRequest {
    /// Requested factor name; matched case-insensitively.
    pub name: String,
    /// Parameters in the order they were supplied.
    pub params: Vec<(String, String)>,
}

impl FactorFactoryRequest {
    /// Create a request for `name` with no parameters.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
        }
    }

    /// Add a parameter.
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    /// Collapse the parameters into a map, rejecting duplicates.
    ///
    /// # Errors
    ///
    /// Returns [`FactorFactoryError::DuplicateParameter`] if the same parameter
    /// name appears more than once.
    pub fn try_params_map(&self) -> Result<HashMap<String, String>, FactorFactoryError> {
        let mut params = HashMap::with_capacity(self.params.len());
        for (key, value) in &self.params {
            if params.insert(key.clone(), value.clone()).is_some() {
                return Err(FactorFactoryError::DuplicateParameter {
                    factor: self.name.to_ascii_uppercase(),
                    name: key.clone(),
                });
            }
        }
        Ok(params)
    }

    /// Stable parameter identity for cache keys and execution traces.
    ///
    /// Sorted by name then value, so the result depends only on the parameter
    /// set and not on the order the caller supplied it in.
    #[must_use]
    pub fn canonical_params(&self) -> String {
        let mut params = self.params.clone();
        params.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        params
            .into_iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",")
    }
}

/// Provider boundary: constructs an executable factor definition from parameters.
///
/// A provider is the parameterised half of factor registration:
/// [`crate::factors::FactorDefinition`] is *what* gets registered, a provider is
/// *how* one is built for a given parameter set. Implementations should encode
/// the parameter set in the returned definition's name (for example
/// `SMA(period=20)`) so that differently-parameterised instances stay
/// distinguishable in a [`crate::factors::FactorRegistry`].
pub trait FactorProvider: Send + Sync {
    /// Provider name, matched case-insensitively against a request's `name`.
    fn name(&self) -> &str;

    /// Build a factor definition for `params`.
    ///
    /// Implementations should reject anything they do not accept via
    /// [`reject_unknown`] and parse numerics via [`positive_usize`], so callers
    /// get [`FactorFactoryError`] rather than a silently wrong factor.
    ///
    /// # Errors
    ///
    /// Returns [`FactorFactoryError`] when a parameter is out of contract
    /// ([`FactorFactoryError::InvalidParameter`]), not accepted by this provider
    /// ([`FactorFactoryError::UnknownParameter`]), or supplied more than once
    /// ([`FactorFactoryError::DuplicateParameter`]).
    fn create(
        &self,
        params: &HashMap<String, String>,
    ) -> Result<FactorDefinition, FactorFactoryError>;
}

/// Errors from [`FactorProviderRegistry`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorProviderError {
    /// No provider is registered under the requested name.
    UnknownProvider(String),
    /// The provider rejected the request.
    Factory(FactorFactoryError),
}

impl fmt::Display for FactorProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProvider(name) => write!(f, "unknown factor provider: {name}"),
            Self::Factory(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for FactorProviderError {}

impl From<FactorFactoryError> for FactorProviderError {
    fn from(error: FactorFactoryError) -> Self {
        Self::Factory(error)
    }
}

/// Registry of [`FactorProvider`]s, keyed by upper-cased name.
#[derive(Clone, Default)]
pub struct FactorProviderRegistry {
    providers: HashMap<String, Arc<dyn FactorProvider>>,
}

impl fmt::Debug for FactorProviderRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FactorProviderRegistry")
            .field("provider_count", &self.providers.len())
            .finish()
    }
}

impl FactorProviderRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider, replacing any existing entry with the same name.
    pub fn register<P>(&mut self, provider: P)
    where
        P: FactorProvider + 'static,
    {
        self.providers
            .insert(provider.name().to_ascii_uppercase(), Arc::new(provider));
    }

    /// Look up a provider by name, case-insensitively.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn FactorProvider>> {
        self.providers.get(&name.to_ascii_uppercase()).cloned()
    }

    /// Whether a provider is registered under `name`.
    #[must_use]
    pub fn contains(&self, name: &str) -> bool {
        self.providers.contains_key(&name.to_ascii_uppercase())
    }

    /// Build a factor definition from a declarative request.
    ///
    /// The provider is resolved **before** the request is validated, so an
    /// unknown name reports [`FactorProviderError::UnknownProvider`] even when
    /// the request also has malformed parameters. Callers that need to validate
    /// a request independently of registration should use
    /// [`FactorFactoryRequest::try_params_map`] directly.
    ///
    /// # Errors
    ///
    /// Returns [`FactorProviderError::UnknownProvider`] when no provider matches
    /// the request name, and [`FactorProviderError::Factory`] when the provider
    /// rejects the request (see [`FactorProvider::create`]).
    pub fn create(
        &self,
        request: &FactorFactoryRequest,
    ) -> Result<FactorDefinition, FactorProviderError> {
        let name = request.name.to_ascii_uppercase();
        let provider = self
            .get(&name)
            .ok_or(FactorProviderError::UnknownProvider(name))?;
        let params = request.try_params_map()?;
        provider.create(&params).map_err(Into::into)
    }

    /// Number of registered providers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

/// Parse a positive integer parameter, falling back to `default` when absent.
///
/// Returns [`FactorFactoryError::InvalidParameter`] for a non-numeric value and
/// for `0`, since a zero period is a contract violation rather than a request for
/// an empty result.
///
/// # Errors
///
/// Returns [`FactorFactoryError::InvalidParameter`] when the value is absent and
/// `default` is unusable, is not a `usize`, or is `0`.
pub fn positive_usize<S: BuildHasher>(
    factor: &str,
    params: &HashMap<String, String, S>,
    name: &str,
    default: usize,
) -> Result<usize, FactorFactoryError> {
    let default_value = default.to_string();
    let value = params
        .get(name)
        .map_or(default_value.as_str(), String::as_str);
    let parsed = value
        .parse::<usize>()
        .map_err(|_| FactorFactoryError::InvalidParameter {
            factor: factor.to_string(),
            name: name.to_string(),
            value: value.to_string(),
            reason: "must be a positive integer".to_string(),
        })?;
    if parsed == 0 {
        return Err(FactorFactoryError::InvalidParameter {
            factor: factor.to_string(),
            name: name.to_string(),
            value: value.to_string(),
            reason: "must be greater than zero".to_string(),
        });
    }
    Ok(parsed)
}

/// Reject any parameter the provider does not accept.
///
/// The first offending name is reported; the iteration order of a `HashMap` is
/// unspecified, so callers must not depend on *which* one is reported when more
/// than one is unknown.
///
/// # Errors
///
/// Returns [`FactorFactoryError::UnknownParameter`] for the first parameter not
/// listed in `allowed`.
pub fn reject_unknown<S: BuildHasher>(
    factor: &str,
    params: &HashMap<String, String, S>,
    allowed: &[&str],
) -> Result<(), FactorFactoryError> {
    if let Some(name) = params.keys().find(|name| !allowed.contains(&name.as_str())) {
        return Err(FactorFactoryError::UnknownParameter {
            factor: factor.to_string(),
            name: name.clone(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factors::{FactorDirection, FactorInputs, FactorKind, FactorResult};

    /// A provider that builds an identity factor, so the tests exercise the
    /// construction boundary rather than any indicator semantics.
    struct IdentityProvider;

    impl FactorProvider for IdentityProvider {
        fn name(&self) -> &str {
            "IDENTITY"
        }

        fn create(
            &self,
            params: &HashMap<String, String>,
        ) -> Result<FactorDefinition, FactorFactoryError> {
            reject_unknown(self.name(), params, &["period"])?;
            let period = positive_usize(self.name(), params, "period", 1)?;
            Ok(FactorDefinition::new(
                format!("IDENTITY(period={period})"),
                ["close"],
                FactorKind::TimeSeries,
                FactorDirection::Neutral,
                Arc::new(|inputs: &FactorInputs<'_>| -> FactorResult<Vec<f64>> {
                    Ok(inputs.get("close")?.to_vec())
                }),
            ))
        }
    }

    #[test]
    fn canonical_params_is_order_independent() {
        let forward = FactorFactoryRequest::new("SMA")
            .with_param("a", "1")
            .with_param("b", "2");
        let reverse = FactorFactoryRequest::new("SMA")
            .with_param("b", "2")
            .with_param("a", "1");
        assert_eq!(forward.canonical_params(), "a=1,b=2");
        assert_eq!(reverse.canonical_params(), "a=1,b=2");
        assert_eq!(forward.canonical_params(), reverse.canonical_params());
    }

    #[test]
    fn duplicate_parameter_is_rejected() {
        let request = FactorFactoryRequest::new("SMA")
            .with_param("period", "3")
            .with_param("period", "4");
        assert_eq!(
            request.try_params_map(),
            Err(FactorFactoryError::DuplicateParameter {
                factor: "SMA".to_string(),
                name: "period".to_string(),
            })
        );

        // Through the registry the provider is resolved *before* the request is
        // validated, so this request has to name the registered provider.
        let mut registry = FactorProviderRegistry::new();
        registry.register(IdentityProvider);
        let duplicated = FactorFactoryRequest::new("identity")
            .with_param("period", "3")
            .with_param("period", "4");
        assert!(matches!(
            registry.create(&duplicated),
            Err(FactorProviderError::Factory(
                FactorFactoryError::DuplicateParameter { .. }
            ))
        ));
    }

    #[test]
    fn unknown_parameter_is_rejected() {
        let params = HashMap::from([("bogus".to_string(), "1".to_string())]);
        assert_eq!(
            reject_unknown("IDENTITY", &params, &["period"]),
            Err(FactorFactoryError::UnknownParameter {
                factor: "IDENTITY".to_string(),
                name: "bogus".to_string(),
            })
        );

        let mut registry = FactorProviderRegistry::new();
        registry.register(IdentityProvider);
        assert!(matches!(
            registry.create(&FactorFactoryRequest::new("identity").with_param("bogus", "1")),
            Err(FactorProviderError::Factory(
                FactorFactoryError::UnknownParameter { .. }
            ))
        ));
    }

    #[test]
    fn invalid_parameter_is_rejected() {
        let mut registry = FactorProviderRegistry::new();
        registry.register(IdentityProvider);

        for bad in ["abc", "0", "-1"] {
            assert!(
                matches!(
                    registry.create(&FactorFactoryRequest::new("IDENTITY").with_param("period", bad)),
                    Err(FactorProviderError::Factory(
                        FactorFactoryError::InvalidParameter { .. }
                    ))
                ),
                "period={bad} should be rejected"
            );
        }

        // Absent is fine: the default applies.
        let definition = registry
            .create(&FactorFactoryRequest::new("IDENTITY"))
            .expect("default period is valid");
        assert_eq!(definition.name, "IDENTITY(period=1)");
    }

    #[test]
    fn unknown_provider_is_rejected() {
        let registry = FactorProviderRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        // `matches!` rather than `assert_eq!` on the whole `Result`: the `Ok`
        // type is `FactorDefinition`, which deliberately does not implement
        // `Debug`/`PartialEq` (its `compute` callback is a closure).
        assert!(matches!(
            registry.create(&FactorFactoryRequest::new("NOPE")),
            Err(FactorProviderError::UnknownProvider(name)) if name == "NOPE"
        ));
    }

    #[test]
    fn registry_constructs_a_runnable_definition() {
        let mut registry = FactorProviderRegistry::new();
        registry.register(IdentityProvider);
        assert!(registry.contains("identity"));
        assert_eq!(registry.len(), 1);

        let definition = registry
            .create(&FactorFactoryRequest::new("identity").with_param("period", "5"))
            .expect("identity provider accepts period=5");

        // The parameter set is encoded in the name, so two parameterisations of
        // the same provider stay distinguishable once registered.
        assert_eq!(definition.name, "IDENTITY(period=5)");
        assert_eq!(definition.dependencies, ["close".to_string()]);
        assert_eq!(definition.kind, FactorKind::TimeSeries);

        let mut factors = crate::factors::FactorRegistry::new();
        factors.register(definition).unwrap();
        let engine = crate::factors::FactorEngine::new(factors);
        let context = crate::factors::FactorContext::new()
            .with_series("close", vec![1.0, 2.0, 3.0])
            .unwrap();
        assert_eq!(
            engine.evaluate("IDENTITY(period=5)", &context).unwrap(),
            vec![1.0, 2.0, 3.0]
        );
    }
}
