//! Runtime factor output model.

/// Runtime outputs reuse the factor crate's canonical primary-plus-named-output
/// result instead of maintaining a second incompatible result type.
pub type FactorOutput = finkit_factor::FactorResult;
