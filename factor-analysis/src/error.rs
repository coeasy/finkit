use thiserror::Error;

/// Errors raised by the factor research layer.
#[derive(Debug, Error)]
pub enum ResearchError {
    #[error("missing column: {0}")]
    MissingColumn(String),
    #[error("missing group column: {0}")]
    MissingGroup(String),
    #[error("length mismatch for {name}: expected {expected}, got {actual}")]
    LengthMismatch {
        name: String,
        expected: usize,
        actual: usize,
    },
    #[error("invalid research configuration: {0}")]
    InvalidConfig(String),
    #[error("research computation failed: {0}")]
    Compute(String),
}

pub type ResearchResult<T> = Result<T, ResearchError>;

impl From<finkit::TaError> for ResearchError {
    fn from(value: finkit::TaError) -> Self {
        Self::Compute(value.to_string())
    }
}
