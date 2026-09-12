#![doc = include_str!("../README.md")]
#![allow(missing_docs)]
#![allow(deprecated)]
#![allow(missing_debug_implementations)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "std", feature = "no_std"))]
compile_error!("Features \"std\" and \"no_std\" are mutually exclusive");

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
pub mod backtest;
#[cfg(feature = "std")]
pub mod backtest_evaluation;
#[cfg(feature = "rayon")]
pub mod batch;
#[cfg(feature = "std")]
pub mod buffer_arena;
#[cfg(feature = "std")]
pub mod calendar;
#[cfg(feature = "std")]
pub mod chan;
#[cfg(feature = "std")]
pub mod chan_mtf;
#[cfg(feature = "std")]
pub mod composite;
#[cfg(feature = "std")]
pub mod compute;
#[cfg(all(feature = "std", not(feature = "no_std")))]
pub mod error;
#[cfg(feature = "std")]
pub mod factor_system;
#[cfg(feature = "std")]
pub mod factors;
#[cfg(all(feature = "std", feature = "indicators-all", feature = "formula"))]
pub mod features;
#[cfg(feature = "formula")]
pub mod formula;
#[cfg(feature = "std")]
pub mod indicators;
pub mod math;
#[cfg(feature = "std")]
pub mod multi_period_resonance;
#[cfg(feature = "std")]
pub mod patterns;
#[cfg(feature = "std")]
pub mod performance;
#[cfg(feature = "finkit-polars")]
pub mod polars_ext;
#[cfg(feature = "std")]
pub mod registry;
#[cfg(feature = "std")]
pub mod returns;
#[cfg(feature = "std")]
pub mod risk;
#[cfg(feature = "std")]
pub mod runtime;
#[cfg(feature = "std")]
pub mod runtime_engine;
#[cfg(feature = "std")]
pub mod schema;
#[cfg(feature = "std")]
pub mod sector;
#[cfg(feature = "std")]
pub mod selectors;
#[cfg(all(feature = "std", feature = "indicators-all"))]
pub mod streaming;
#[cfg(feature = "talib-c")]
pub mod talib_ffi;
pub mod traits;
#[cfg(feature = "std")]
pub mod transforms;
#[cfg(feature = "std")]
pub mod unified_runtime;
#[cfg(feature = "std")]
pub mod utils;

#[cfg(feature = "tracing")]
pub use tracing::{debug, error, info, instrument, trace, warn, Level};

pub mod metrics;

#[cfg(feature = "std")]
pub use error::{FormulaError, IndicatorError, Result, TaError};
pub use traits::{BatchIndicator, Ohlcv, OhlcvArrayAdapter, OhlcvBar, StreamingIndicator};
