#![doc = include_str!("../README.md")]
// ADR 0011: rustdoc 100% coverage policy for the **public surface only**.
// `missing_docs` is set to `allow` at the crate level because the indicator
// implementations across `streaming/`, `indicators/`, `features/`, etc. each
// expose 30-200 small public items (a single function or struct per indicator),
// and a separate `///` for every one of those does not improve the user-facing
// API reference — the **module-level** doc + the public re-exports here are
// what users see.  Truly public surface items (the ones re-exported from this
// `lib.rs`) carry a doc comment.  Internal items use `pub(crate)` or are
// `#[allow(missing_docs)]` at the module level.
#![allow(missing_docs)]
#![allow(deprecated)]
// `missing_debug_implementations` is suppressed at the crate level for the
// same reason: hundreds of internal builder/AST/opcode types live in `formula/`
// and `streaming/` and are not part of the user-facing surface. Adding a
// manual `impl Debug` to each would be noisy without changing the public
// contract. Builders have a derived `Debug` from the macro; everything else
// is fine as-is.
#![allow(missing_debug_implementations)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "std", feature = "no_std"))]
compile_error!("Features \"std\" and \"no_std\" are mutually exclusive");

#[cfg(feature = "std")]
extern crate std;

// B1: the `alloc` crate (Vec/Box/VecDeque/…) is a sysroot crate and is not in
// the extern prelude, so it must be declared explicitly in BOTH `std` and
// `no_std` builds for `alloc::`-prefixed paths (used by the `no_std`-capable
// `math` subset) to resolve.
extern crate alloc;

#[cfg(all(feature = "std", not(feature = "no_std")))]
/// Domain error types for indicator computations and the formula engine.
pub mod error;
#[cfg(feature = "std")]
/// Unified semantic compute plans, factor plans, and runtime execution policies.
pub mod compute;
#[cfg(feature = "formula")]
/// Formula engine with AST parsing, bytecode compilation, JIT optimization, and SIMD acceleration.
pub mod formula;
#[cfg(feature = "std")]
/// Batch technical indicator computations (150+ indicators).
pub mod indicators;
/// Core mathematical primitives: moving averages, linear regression, statistics, and SIMD kernels.
pub mod math;
#[cfg(feature = "std")]
/// Candlestick and chart pattern recognition (60+ patterns).
pub mod patterns;
// `streaming` and `features` consume the *full* indicator surface (they
// reference `crate::indicators::{overlap,volatility_ext,atr,sma,…}` and the
// `formula` engine). They are therefore gated behind `indicators-all` so that
// turning the indicator surface off (tree-shaking) also prunes its dependents.
// Default build keeps `indicators-all` on, so public behaviour is unchanged.
#[cfg(feature = "std")]
/// Lightweight vectorized backtest engine.
pub mod backtest;
#[cfg(feature = "rayon")]
/// Parallel batch API: run multiple independent indicator jobs in parallel
/// over the same input slice. Disabled in no_std builds.
pub mod batch;
#[cfg(feature = "std")]
/// Dependency-aware production factor engine and factor transforms.
pub mod factors;
#[cfg(all(feature = "std", feature = "indicators-all", feature = "formula"))]
/// Feature engineering: multi-period features, signal detection, and ML label generation.
pub mod features;
#[cfg(feature = "std")]
/// Multi-timeframe pattern resonance: 5m/30m/日线 联动信号.
pub mod multi_period_resonance;
#[cfg(feature = "finkit-polars")]
/// Polars DataFrame zero-copy integration for technical analysis.
pub mod polars_ext;
#[cfg(feature = "std")]
/// Canonical indicator/formula metadata registry for bindings and introspection.
pub mod registry;
#[cfg(feature = "std")]
/// Portfolio risk metrics: VaR / CVaR / MDD / Sharpe / Sortino / Calmar.
pub mod risk;
#[cfg(feature = "std")]
/// Zero-copy aligned market-frame and warm-up/NaN runtime contracts.
pub mod runtime;
#[cfg(feature = "std")]
/// 申万一级 31 行业板块轮动.
pub mod sector;
#[cfg(feature = "std")]
/// 选股因子合成 + 横截面排序.
pub mod selectors;
#[cfg(all(feature = "std", feature = "indicators-all"))]
/// Streaming (incremental) O(1) per-bar indicator updates.
pub mod streaming;
#[cfg(feature = "talib-c")]
/// TA-Lib C library FFI compatibility layer.
pub mod talib_ffi;
/// Top-level trait abstractions: [`Ohlcv`], [`StreamingIndicator`], [`BatchIndicator`].
pub mod traits;
#[cfg(feature = "std")]
/// Data transformation pipelines: rolling windows, normalization, and feature scaling.
pub mod transforms;
#[cfg(feature = "std")]
/// Utility functions: input validation, smoothing factors, and array helpers.
pub mod utils;

// ─────────────────── O-1: tracing re-exports ───────────────────
//
// When the `tracing` feature is enabled, downstream users can `use finkit::info`
// etc. to instrument their integration code, and the library's own spans (added via
// `#[instrument]`) become visible once a `tracing-subscriber` is initialized.
#[cfg(feature = "tracing")]
pub use tracing::{debug, error, info, instrument, trace, warn, Level};

// ─────────────────── O-2: metrics facade ───────────────────
//
// `metrics::counter!(...).increment(1);` etc. — these macros always compile, but
// only emit real values when a recorder is installed (e.g. the Prometheus
// exporter behind the `metrics-prometheus` feature).
//
// The module is always compiled in (no `#[cfg(feature = "metrics")]`) so that
// the `streaming_measure!` and `timed!` macro *paths* stay at the crate root
// for every build configuration — including `wasm32-unknown-unknown` and any
// other target where `std::time` is unavailable. The function-level bodies
// stay feature-gated inside `metrics.rs` so `no_std` builds don't pull in
// the `metrics` crate.
pub mod metrics;

#[cfg(feature = "std")]
/// Curated re-exports of the most common error / result types.
///
/// - [`TaError`] — the legacy top-level enum (still widely used; behaviour
///   unchanged).
/// - [`IndicatorError`] — finer-grained errors for indicator computations.
/// - [`FormulaError`] — errors raised by the formula engine.
/// - [`Result`] — the library's default `Result` alias.
pub use error::{FormulaError, IndicatorError, Result, TaError};
/// Top-level trait abstractions re-exported for convenience.
///
/// - [`BatchIndicator`] — compute an indicator over a full input series.
/// - [`StreamingIndicator`] — compute an indicator incrementally, O(1) per bar.
/// - [`Ohlcv`], [`OhlcvBar`], [`OhlcvArrayAdapter`] — uniform OHLCV access
///   across `f64` slices, struct bars, and `ndarray` arrays.
pub use traits::{BatchIndicator, Ohlcv, OhlcvArrayAdapter, OhlcvBar, StreamingIndicator};
