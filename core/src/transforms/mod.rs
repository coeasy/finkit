//! Data transformation pipeline.
//!
//! Provides scikit-learn style data transformers and a composable pipeline.
//!
//! # Example
//!
//! ```
//! use finkit::transforms::{Pipeline, LogReturn, ZScore, Transform};
//!
//! let data = vec![100.0, 105.0, 103.0, 108.0, 110.0, 107.0, 112.0, 115.0, 113.0, 118.0];
//! let result = Pipeline::new()
//!     .add(LogReturn)
//!     .add(ZScore)
//!     .transform(&data);
//! assert!(!result.is_empty());
//! ```

mod log_return;
mod pct_change;
mod zscore;
mod scaler;
mod pipeline;
mod rank;
mod diff;
mod rolling;

pub use log_return::LogReturn;
pub use pct_change::PctChange;
pub use zscore::ZScore;
pub use scaler::{StandardScaler, MinMaxScaler};
pub use pipeline::Pipeline;
pub use rank::{Rank, PercentileRank};
pub use diff::{Diff, DiffN};
pub use rolling::{RollingMean, RollingStd, RollingSum};

/// A data transformation that converts an input slice to a new vector.
pub trait Transform: Send + Sync {
    /// Apply the transformation to the input data.
    fn transform(&self, input: &[f64]) -> Vec<f64>;
}
