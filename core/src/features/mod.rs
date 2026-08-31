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

mod combinations;
pub mod complexity;
mod cross_features;
mod cv_split;
mod encoding;
mod engine;
mod export;
pub mod fourier;
mod garch;
mod importance;
mod labels;
mod market_structure;
mod matrix;
mod meta_labels;
mod microstructure;
mod multi_period;
mod normalization;
mod parallel;
mod pca;
mod regime;
mod rolling_stats;
mod selection;
mod signals;
mod simd_opt;
mod stability;
mod store;
mod time_features;
mod timeseries;
mod types;
pub mod wavelet;

pub use combinations::*;
pub use complexity::*;
pub use cross_features::*;
pub use cv_split::*;
pub use encoding::*;
pub use engine::*;
pub use export::*;
pub use fourier::*;
pub use garch::*;
pub use importance::*;
pub use labels::*;
pub use market_structure::*;
pub use matrix::*;
pub use meta_labels::*;
pub use microstructure::*;
pub use multi_period::*;
pub use normalization::*;
pub use parallel::*;
pub use pca::*;
pub use regime::*;
pub use rolling_stats::*;
pub use selection::*;
pub use signals::*;
pub use simd_opt::*;
pub use stability::*;
pub use store::*;
pub use time_features::*;
pub use timeseries::*;
pub use types::*;
pub use wavelet::*;
