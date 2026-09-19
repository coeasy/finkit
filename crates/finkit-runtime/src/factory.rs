//! Typed factor provider and construction errors.

use finkit_factor::Factor;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// A factor instance owned by the execution runtime.
pub type DynFactor = Box<dyn Factor + Send + Sync>;

/// Errors raised while turning a declarative factor request into an instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FactorFactoryError {
    /// A parameter could not be parsed or violates the factor contract.
    InvalidParameter {
        factor: String,
        name: String,
        value: String,
        reason: String,
    },
    /// A request contains a parameter not accepted by the provider.
    UnknownParameter { factor: String, name: String },
    /// A request contains the same parameter more than once.
    DuplicateParameter { factor: String, name: String },
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

#[derive(Clone, Debug)]
pub struct FactorFactoryRequest {
    pub name: String,
    pub params: Vec<(String, String)>,
}

impl FactorFactoryRequest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            params: Vec::new(),
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.push((key.into(), value.into()));
        self
    }

    pub fn params_map(&self) -> HashMap<String, String> {
        self.params.iter().cloned().collect()
    }

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

/// Provider boundary used by the registry to construct executable factors.
pub trait FactorProvider: Send + Sync {
    fn name(&self) -> &str;

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError>;
}

/// The old name is retained as a type alias inside the new typed boundary.
pub use FactorProvider as FactorFactory;

/// A configurable provider useful for plugins and tests.
pub struct SimpleFactorFactory {
    name: String,
    builder: Arc<
        dyn Fn(&HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> + Send + Sync,
    >,
}

impl SimpleFactorFactory {
    pub fn new(
        name: impl Into<String>,
        builder: impl Fn(&HashMap<String, String>) -> Result<DynFactor, FactorFactoryError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            builder: Arc::new(builder),
        }
    }
}

impl fmt::Debug for SimpleFactorFactory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimpleFactorFactory")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl FactorProvider for SimpleFactorFactory {
    fn name(&self) -> &str {
        &self.name
    }

    fn create(&self, params: &HashMap<String, String>) -> Result<DynFactor, FactorFactoryError> {
        (self.builder)(params)
    }
}

/// Parse a positive integer factor parameter and produce a structured error.
pub(crate) fn positive_usize(
    factor: &str,
    params: &HashMap<String, String>,
    name: &str,
    default: usize,
) -> Result<usize, FactorFactoryError> {
    let default_value = default.to_string();
    let value = params
        .get(name)
        .map(String::as_str)
        .unwrap_or(default_value.as_str());
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

pub(crate) fn reject_unknown(
    factor: &str,
    params: &HashMap<String, String>,
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
