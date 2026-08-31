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

mod diff;
mod log_return;
mod pct_change;
mod pipeline;
mod rank;
mod rolling;
mod scaler;
mod zscore;

pub use diff::{Diff, DiffN};
pub use log_return::LogReturn;
pub use pct_change::PctChange;
pub use pipeline::Pipeline;
pub use rank::{PercentileRank, Rank};
pub use rolling::{RollingMean, RollingStd, RollingSum};
pub use scaler::{MinMaxScaler, StandardScaler};
pub use zscore::ZScore;

/// A data transformation that converts an input slice to a new vector.
pub trait Transform: Send + Sync {
    /// Apply the transformation to the input data.
    fn transform(&self, input: &[f64]) -> Vec<f64>;
}
