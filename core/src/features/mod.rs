//! Feature engineering module for quantitative analysis and machine learning.
//!
//! This module provides tools to transform raw OHLCV data and technical indicators
//! into feature matrices suitable for ML model training and strategy backtesting.
//!
//! # Architecture
//!
//! - [`FeatureMatrix`] — 2D matrix of features (rows=time points, columns=features)
//! - [`Feature`] — metadata describing a single feature column
//! - [`FeatureEngine`] — trait for feature generators
//! - [`FeatureSet`] — composable collection of feature generators
//!
//! # Example
//!
//! ```
//! use finkit::features::{FeatureSet, FeatureMatrix, MultiPeriodFeature};
//!
//! let close = vec![100.0, 101.0, 102.0, 101.5, 103.0, 104.0, 103.5, 105.0, 106.0, 107.0];
//! let mut engine = FeatureSet::new();
//! engine.add_indicator("sma", &[5, 10]);
//! let matrix = engine.generate(&close);
//! assert!(matrix.cols() > 0);
//! ```

mod types;
mod engine;
mod matrix;
mod multi_period;
mod signals;
mod timeseries;
mod rolling_stats;
mod normalization;
mod labels;
mod combinations;
mod cross_features;
mod microstructure;
mod selection;
mod pca;
mod importance;
mod export;
mod simd_opt;
mod regime;
mod market_structure;
mod time_features;
mod meta_labels;
mod encoding;
mod stability;
mod garch;
mod cv_split;
mod parallel;
mod store;
pub mod complexity;
pub mod wavelet;
pub mod fourier;

pub use types::*;
pub use engine::*;
pub use matrix::*;
pub use multi_period::*;
pub use signals::*;
pub use timeseries::*;
pub use rolling_stats::*;
pub use normalization::*;
pub use labels::*;
pub use combinations::*;
pub use cross_features::*;
pub use microstructure::*;
pub use selection::*;
pub use pca::*;
pub use importance::*;
pub use export::*;
pub use simd_opt::*;
pub use regime::*;
pub use meta_labels::*;
pub use market_structure::*;
pub use time_features::*;
pub use cv_split::*;
pub use encoding::*;
pub use stability::*;
pub use garch::*;
pub use store::*;
pub use parallel::*;
pub use complexity::*;
pub use wavelet::*;
pub use fourier::*;
